use std::{collections::BTreeMap, sync::RwLock};

use substrait::proto::{
    expression::{RexType, ScalarFunction},
    extensions::{
        simple_extension_declaration::{ExtensionFunction, MappingType},
        SimpleExtensionDeclaration,
    },
    function_argument::ArgType,
    Expression, FunctionArgument, FunctionOption, Type,
};

use crate::{
    error::{Result, SubstraitExprError},
    helpers::{
        schema::SchemaInfo,
        types::{self, unknown, TypeExt},
        UriRegistry,
    },
};

use super::ExpressionExt;

/// This is a rust equivalent of a YAML function definition
///
/// We chose to use mirror types here as the YAML schema is still
/// a little loose and we wanted something simpler.  The full types
/// can be obtained using the substrait library and serde_yaml.
#[derive(Clone, Debug)]
pub struct FunctionDefinition {
    /// The URI of the function
    ///
    /// Note: this is the one field that is not actually present in the YAML
    /// but is generally the URI of the YAML itself
    pub uri: String,
    /// The name of the function
    pub name: String,
    /// The various implementation kernels supported by the function
    pub implementations: Vec<FunctionImplementation>,
}

/// Represents a function argument
#[derive(Clone, Debug)]
pub enum ImplementationArgType {
    /// The argument is a constant choice between a small set of possible values
    ///
    /// For example, the "extract" function uses an enum to select the field to
    /// extract from a datetime value.
    Enum(Vec<String>),
    /// A regular argument provided by an expression of the given type
    Value(Type),
}

/// A named function argument
#[derive(Clone, Debug)]
pub struct ImplementationArg {
    /// The name of the argument
    ///
    /// This is used for documentation and readability purposes.  Consumers
    /// don't generally care what the name is.
    pub name: String,
    /// The type of the argument
    pub arg_type: ImplementationArgType,
}

impl ImplementationArg {
    /// Returns true if an expression of the given type could be used as this argument
    ///
    /// There is no "enum" type so enum arguments will only recognize the string type
    pub fn matches(&self, arg_type: &Type) -> Result<bool> {
        if arg_type.is_unknown() {
            Ok(true)
        } else {
            match &self.arg_type {
                ImplementationArgType::Enum(_) => arg_type.same_kind(&types::string(true)),
                ImplementationArgType::Value(expected_type) => arg_type.same_kind(expected_type),
            }
        }
    }
}

/// A potential implementation of a function
#[derive(Clone, Debug)]
pub struct FunctionImplementation {
    /// The input arguments
    pub args: Vec<ImplementationArg>,
    /// The type that should be output from the function
    pub output_type: Type,
}

impl FunctionImplementation {
    /// Returns true if expressions with types specified by `arg_types` would match this implementation
    pub fn matches(&self, arg_types: &[Type]) -> bool {
        if arg_types.len() != self.args.len() {
            false
        } else {
            self.args
                .iter()
                .zip(arg_types)
                .all(|(imp_arg, arg_type)| imp_arg.matches(arg_type).unwrap_or(false))
        }
    }

    fn relax(&self, types: Vec<Type>) -> Result<FunctionImplementation> {
        if self.args.len() != types.len() {
            Err(SubstraitExprError::InvalidInput(format!(
                "Attempt to relax implementation with {} args using {} types",
                self.args.len(),
                types.len()
            )))
        } else {
            let relaxed_args = self
                .args
                .iter()
                .zip(types.iter())
                .map(|(arg, typ)| {
                    if typ.is_unknown() {
                        ImplementationArg {
                            name: arg.name.clone(),
                            arg_type: ImplementationArgType::Value(typ.clone()),
                        }
                    } else {
                        arg.clone()
                    }
                })
                .collect::<Vec<_>>();
            let has_unknown = types.iter().any(|typ| typ.is_unknown());
            let output_type = if has_unknown {
                types::unknown()
            } else {
                self.output_type.clone()
            };
            Ok(FunctionImplementation {
                args: relaxed_args,
                output_type,
            })
        }
    }
}

impl FunctionDefinition {
    /// Given input expressions this attempts to find a matching implementation
    ///
    /// This is still very experimental and the implementation resolution rules
    /// are subject to change.
    ///
    /// Currently this looks for an implementation that exactly matches the input
    /// expressions' types.  If any of the input types are the unknown type then
    /// those arguments are considered matching but the return type is changed to
    /// unknown.
    pub fn pick_implementation_from_args(
        &self,
        args: &[Expression],
        schema: &SchemaInfo,
    ) -> Result<Option<FunctionImplementation>> {
        let types = args
            .iter()
            .map(|arg| arg.output_type(schema))
            .collect::<Result<Vec<_>>>()?;
        self.implementations
            .iter()
            .find(|imp| imp.matches(&types))
            .map(|imp| imp.relax(types))
            .transpose()
    }
}

/// The URI of the special function we use to indicate a late lookup
///
/// See [`lookup_field_by_name`](crate::builder::functions::FunctionsBuilder::lookup_field_by_name)
///
/// This is very likely to change when Substrait formally adopts a late lookup feature
pub const LOOKUP_BY_NAME_FUNC_URI: &'static str = "https://substrait.io/functions";
/// The name of the special function we use to indicate a late lookup
pub const LOOKUP_BY_NAME_FUNC_NAME: &'static str = "lookup_by_name";

/// A builder that can create scalar function expressions
pub struct FunctionsBuilder<'a> {
    registry: &'a FunctionsRegistry,
    schema: &'a SchemaInfo,
}

impl<'a> FunctionsBuilder<'a> {
    pub(crate) fn new(registry: &'a FunctionsRegistry, schema: &'a SchemaInfo) -> Self {
        Self { registry, schema }
    }

    /// Creates a new [FunctionBuilder] based on a given function definition.
    ///
    /// This method is not typically used directly.  Instead, extension functions
    /// like `add` or `subtract` are used which call this function.
    ///
    /// However, this could be used directly for UDFs if you don't want to create an
    /// extension trait.
    pub fn new_builder(
        &self,
        func: &'static FunctionDefinition,
        args: Vec<Expression>,
    ) -> FunctionBuilder {
        let func_reference = self.registry.register(func);
        FunctionBuilder {
            func: func,
            func_reference,
            args,
            options: BTreeMap::new(),
            schema: self.schema,
        }
    }

    /// Creates a "late lookup" function expression
    ///
    /// This is not really a function call.  It's a placeholder we are currently
    /// using to indicate an unresolved field reference.  This is created whenever
    /// a user creates a field reference by name but the schema is unknown or does
    /// not know names.
    pub fn lookup_field_by_name(&self, name: impl Into<String>) -> Expression {
        let arg = FunctionArgument {
            arg_type: Some(ArgType::Enum(name.into())),
        };
        let function_reference = self
            .registry
            .register_by_name(LOOKUP_BY_NAME_FUNC_URI, LOOKUP_BY_NAME_FUNC_NAME);
        Expression {
            rex_type: Some(RexType::ScalarFunction(ScalarFunction {
                arguments: vec![arg],
                function_reference,
                // TODO: Use the proper unknown type
                output_type: Some(unknown()),
                options: vec![],
                ..Default::default()
            })),
        }
    }
}

/// A builder object to create a scalar function expression
///
/// This can be used to parameterize the function call with options
pub struct FunctionBuilder<'a> {
    func: &'static FunctionDefinition,
    func_reference: u32,
    args: Vec<Expression>,
    options: BTreeMap<String, Vec<String>>,
    schema: &'a SchemaInfo,
}

impl<'a> FunctionBuilder<'a> {
    /// Consume the builder and create a function expression
    pub fn build(self) -> Result<Expression> {
        let implementation = self
            .func
            .pick_implementation_from_args(&self.args, self.schema)?
            .ok_or_else(|| {
                SubstraitExprError::invalid_input(format!(
                    "Cannot find matching call to function {:?} that takes the given arguments",
                    self.func
                ))
            })?;
        let arguments = self
            .args
            .into_iter()
            .zip(implementation.args.iter())
            .map(|(arg, imp_arg)| match &imp_arg.arg_type {
                ImplementationArgType::Enum(vals) => {
                    let value = arg.try_as_rust_literal::<&str>()?.to_string();
                    if vals.contains(&value) {
                        Ok(FunctionArgument {
                            arg_type: Some(ArgType::Enum(value)),
                        })
                    } else {
                        Err(SubstraitExprError::InvalidInput(format!(
                            "The value {} is not valid for the argument {}",
                            value, imp_arg.name
                        )))
                    }
                }
                ImplementationArgType::Value(_) => Ok(FunctionArgument {
                    arg_type: Some(ArgType::Value(arg)),
                }),
            })
            .collect::<Result<Vec<_>>>()?;
        let output_type = &implementation.output_type;
        let options = self
            .options
            .into_iter()
            .map(|(key, value)| FunctionOption {
                name: key,
                preference: value,
            })
            .collect::<Vec<_>>();
        Ok(Expression {
            rex_type: Some(RexType::ScalarFunction(ScalarFunction {
                arguments,
                function_reference: self.func_reference,
                output_type: Some(output_type.clone()),
                options,
                ..Default::default()
            })),
        })
    }
}

pub struct FunctionsRegistryRecord {
    uri: String,
    name: String,
    anchor: u32,
}

struct FunctionRegistryInternal {
    function_map: BTreeMap<String, FunctionsRegistryRecord>,
    counter: u32,
}

impl FunctionRegistryInternal {
    fn register(&mut self, uri: &str, name: &str) -> u32 {
        let key = uri.to_string() + name;
        let entry = self.function_map.entry(key);
        entry
            .or_insert_with(|| {
                let counter = self.counter;
                self.counter += 1;
                FunctionsRegistryRecord {
                    uri: uri.to_string(),
                    name: name.to_string(),
                    anchor: counter,
                }
            })
            .anchor
    }
}

pub struct FunctionsRegistry {
    internal: RwLock<FunctionRegistryInternal>,
}

impl FunctionsRegistry {
    pub fn new() -> Self {
        Self {
            internal: RwLock::new(FunctionRegistryInternal {
                function_map: BTreeMap::new(),
                counter: 1,
            }),
        }
    }

    pub fn register(&self, function: &FunctionDefinition) -> u32 {
        let mut internal = self.internal.write().unwrap();
        internal.register(&function.uri, &function.name)
    }

    pub(crate) fn register_by_name(&self, uri: &str, name: &str) -> u32 {
        let mut internal = self.internal.write().unwrap();
        internal.register(uri, name)
    }

    pub(crate) fn add_to_extensions(
        &self,
        uris: &mut UriRegistry,
        extensions: &mut Vec<SimpleExtensionDeclaration>,
    ) {
        let internal = self.internal.read().unwrap();
        for record in internal.function_map.values() {
            let uri_ref = uris.register(record.uri.clone());
            let declaration = SimpleExtensionDeclaration {
                mapping_type: Some(MappingType::ExtensionFunction(ExtensionFunction {
                    extension_uri_reference: uri_ref,
                    function_anchor: record.anchor,
                    name: record.name.clone(),
                })),
            };
            extensions.push(declaration);
        }
    }
}
