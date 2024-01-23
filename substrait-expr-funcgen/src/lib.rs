use std::io::Write;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use substrait::text::simple_extensions::{
    ArgumentsItem, ScalarFunction, ScalarFunctionImplsItem, SimpleExtensions, Type,
};
use thiserror::Error;

/// All errors raised by this crate will be instances of SubstraitExprError
#[derive(Error, Debug)]
pub enum FuncGenError {
    #[error("I/O Error")]
    Disconnect(#[from] std::io::Error),
    #[error("Error processing yaml")]
    YamlError(#[from] serde_yaml::Error),
    #[error("Error in generated code")]
    SynError(#[from] syn::Error),
    #[error("Error generating code")]
    LexError(#[from] proc_macro2::LexError),
    #[error("General error: {0}")]
    GeneralError(String),
}

type Result<T> = std::result::Result<T, FuncGenError>;

fn generate_type(fn_name: &str, type_name: &str) -> Option<TokenStream> {
    let (typ, nullability) = if type_name.ends_with('?') {
        (&type_name[0..type_name.len() - 1], true)
    } else {
        (type_name, false)
    };
    match typ {
        "" => None,
        "i8" => Some(quote!(types::i8(#nullability))),
        "i16" => Some(quote!(types::i16(#nullability))),
        "i32" => Some(quote!(types::i32(#nullability))),
        "i64" => Some(quote!(types::i64(#nullability))),
        "fp32" => Some(quote!(types::fp32(#nullability))),
        "fp64" => Some(quote!(types::fp64(#nullability))),
        "boolean" => Some(quote!(types::bool(#nullability))),
        // Bleah, let's cleanup the yaml files!
        "BOOLEAN" => Some(quote!(types::bool(#nullability))),
        _ => {
            println!(
                "cargo:warning=Ignoring impl of {} with unrecognized type in YAML file: {}",
                fn_name, type_name
            );
            None
        }
    }
}

fn generate_arg_type(fn_name: &str, typ: &Type) -> Option<TokenStream> {
    let type_name = match typ {
        Type::Variant0(type_str) => type_str.as_str(),
        Type::Variant1(_) => "",
    };
    if type_name.is_empty() {
        return None;
    }
    if type_name.contains("any") || type_name == "T" {
        Some(quote!(ImplementationArgType::TemplateValue(#type_name.to_string())))
    } else {
        let typ = generate_type(fn_name, type_name)?;
        Some(quote!(ImplementationArgType::Value(#typ)))
    }
}

fn generate_arg_return(fn_name: &str, typ: &Type) -> Option<TokenStream> {
    let type_name = match typ {
        Type::Variant0(type_str) => type_str.as_str(),
        Type::Variant1(_) => "",
    };
    if type_name.is_empty() {
        return None;
    }
    if type_name.contains("any") || type_name == "T" {
        Some(quote!(FunctionReturn::Templated(#type_name.to_string())))
    } else {
        let typ = generate_type(fn_name, type_name)?;
        Some(quote!(FunctionReturn::Typed(#typ)))
    }
}

fn generate_arg_block(fn_name: &str, arg: &ArgumentsItem) -> Option<TokenStream> {
    match arg {
        ArgumentsItem::Variant0 { .. } => {
            println!(
                "cargo:warning=Ignoring implementation of {} containing variant0 arg item",
                fn_name
            );
            None
        }
        ArgumentsItem::Variant1 { name, value, .. } => {
            let name = name.as_ref()?;
            let typ = generate_arg_type(fn_name, value)?;
            Some(quote!(
                ImplementationArg {
                    name: #name.to_string(),
                    arg_type: #typ
                }
            ))
        }
        ArgumentsItem::Variant2 { .. } => {
            println!("cargo:warning=Ignoring implementation containing variant0 arg item");
            None
        }
    }
}

fn generate_implementation_block(
    fn_name: &str,
    imp: &ScalarFunctionImplsItem,
) -> Option<TokenStream> {
    let output_type = generate_arg_return(fn_name, &imp.return_.0)?;
    let args = imp.args.as_ref()?;
    let args = args
        .iter()
        .map(|arg| generate_arg_block(fn_name, arg))
        .collect::<Option<Vec<_>>>()?;

    Some(quote!(
        FunctionImplementation {
            output_type: #output_type,
            args: vec![#(#args),*],
        }
    ))
}

fn generate_function_block(uri: &str, func: &ScalarFunction) -> Result<TokenStream> {
    let func_name_caps: TokenStream = func.name.to_uppercase().parse()?;
    let func_name = &func.name;

    let implementations = func
        .impls
        .iter()
        .map(|imp| generate_implementation_block(func_name, imp))
        .filter(|imp| imp.is_some())
        .collect::<Vec<_>>();

    Ok(quote!(
        pub static #func_name_caps: Lazy<FunctionDefinition> = Lazy::new(|| FunctionDefinition {
            uri: #uri.to_string(),
            name: #func_name.to_string(),
            implementations: vec![#(#implementations),*]
        });
    ))
}

// pub trait ArithmeticFunctionsExt {
//     fn add(&self, lhs: Expression, rhs: Expression) -> FunctionBuilder;
// }

// impl<'a> ArithmeticFunctionsExt for FunctionsBuilder<'a> {
//     fn add(&self, lhs: Expression, rhs: Expression) -> FunctionBuilder {
//         let func_reference = self.registry.register(&ADD);
//         FunctionBuilder {
//             func: &ADD,
//             func_reference,
//             args: vec![lhs, rhs],
//             options: BTreeMap::new(),
//             schema: self.schema,
//         }
//     }
// }

fn generate_ext_impls(function: &ScalarFunction) -> Result<Vec<(TokenStream, TokenStream)>> {
    let mut num_args = function
        .impls
        .iter()
        .map(|imp| imp.args.as_ref().map(|args| args.len()).unwrap_or(0))
        .filter(|arg| *arg != 0)
        .collect::<Vec<_>>();
    num_args.sort();
    num_args.dedup();

    let fn_name = function.name.to_case(Case::Snake);
    let func_name_caps: TokenStream = function.name.to_uppercase().parse()?;

    Ok(num_args
        .iter()
        .enumerate()
        .map(|(idx, num_args)| {
            let fn_name_token = if idx == 0 {
                fn_name.parse::<TokenStream>().unwrap()
            } else {
                format!("{}{}", fn_name, num_args).as_str().parse().unwrap()
            };
            let arg_name_tokens = (0..*num_args)
                .map(|arg_idx| {
                    format!("arg{}", arg_idx)
                        .as_str()
                        .parse::<TokenStream>()
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let prototype = quote!(fn #fn_name_token(&self, #(#arg_name_tokens: Expression),*) -> FunctionBuilder;);
            let imp = quote!(
                fn #fn_name_token(&self, #(#arg_name_tokens: Expression),*) -> FunctionBuilder {
                    self.new_builder(&#func_name_caps, vec![#(#arg_name_tokens),*])
                }
            );
            (prototype, imp)
        })
        .collect::<Vec<_>>())
}

fn generate_function_blocks(
    uri: &str,
    mod_name: &str,
    extensions: SimpleExtensions,
) -> Result<TokenStream> {
    let statics = extensions
        .scalar_functions
        .iter()
        .map(|ext| generate_function_block(uri, ext))
        .collect::<Result<Vec<_>>>()?;

    let ext_name = mod_name.to_case(Case::Pascal);
    let trait_name = format_ident!("{}Ext", ext_name);

    let prototypes_impls = extensions
        .scalar_functions
        .iter()
        .map(|func| generate_ext_impls(func))
        .flat_map(|impls| match impls {
            Ok(impls) => impls.into_iter().map(Ok).collect(),
            Err(err) => vec![Err(err)],
        })
        .collect::<Result<Vec<_>>>()?;

    let prototypes = prototypes_impls
        .iter()
        .map(|(proto, _)| proto)
        .collect::<Vec<_>>();
    let impls = prototypes_impls
        .iter()
        .map(|(_, imp)| imp)
        .collect::<Vec<_>>();

    Ok(quote!(
        #(#statics)*

        pub trait #trait_name {
            #(#prototypes)*
        }

        impl<'a> #trait_name for FunctionsBuilder<'a> {
            #(#impls)*
        }
    ))
}

pub fn generate_functions_for_yaml(uri: &str, filepath: &str) -> Result<TokenStream> {
    let file = std::fs::File::open(filepath)?;
    let extensions = serde_yaml::from_reader::<_, SimpleExtensions>(file)?;
    let mod_name = std::path::Path::new(filepath)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();

    let func_blocks = generate_function_blocks(uri, &mod_name, extensions)?;

    let mod_name_token: TokenStream = mod_name.parse()?;

    Ok(quote!(
        pub mod #mod_name_token {
            use super::*;

            #func_blocks
        }
    ))
}

#[derive(Default)]
pub struct Options {
    pub outdir: Option<String>,
    pub crate_name: Option<String>,
}

impl Options {
    fn get_outdir(&self) -> String {
        self.outdir.clone().unwrap_or_else(|| {
            std::env::var("OUT_DIR")
                .expect("outdir was not specified and OUT_DIR env variable is not set")
        })
    }

    fn get_crate_name(&self) -> String {
        self.crate_name
            .clone()
            .unwrap_or_else(|| "substrait_expr".to_string())
    }
}

pub fn generate_functions(entries: &[(&str, &str)], options: Options) -> Result<()> {
    let yaml_modules = entries
        .iter()
        .map(|entry| generate_functions_for_yaml(entry.0, &entry.1))
        .collect::<Result<Vec<_>>>()?;
    let crate_name_token: TokenStream = options.get_crate_name().parse()?;

    let tokens = quote!(
        use once_cell::sync::Lazy;
        use substrait::proto::Expression;
        use #crate_name_token::builder::functions::{FunctionDefinition, FunctionImplementation,
            ImplementationArg, ImplementationArgType, FunctionBuilder, FunctionsBuilder, FunctionReturn};
        use #crate_name_token::helpers::types;

        #(#yaml_modules)*
    );

    let syntax_tree = syn::parse2(tokens)?;
    let contents = prettyplease::unparse(&syntax_tree);

    let destdir = options.get_outdir();
    std::fs::create_dir_all(format!("{}/src", destdir))?;
    let mut out_file = std::fs::File::create(format!("{}/src/functions.rs", destdir))?;
    write!(out_file, "{}", contents)?;

    Ok(())
}
