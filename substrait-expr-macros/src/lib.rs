use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};

// Convert rust code to a field in a names-only schema
// Example Input:
//  a: { b: {}, c: { d: {} } }
//
// Example Output:
//  NamesOnlySchemaNode { name: "a".to_string(), children: vec![
//    NamesOnlySchemaNode { name: "b".to_string(), children: vec![] },
//    NamesOnlySchemaNode { name: "c".to_string(), children: vec![
//      NamesOnlySchemaNode { name: "d".to_string(), children: vec![] }
//    ]}
//  ]}
fn rust_field_to_names_field(field: &Field) -> proc_macro2::TokenStream {
    let name = field.name.to_string();
    let children = rust_to_names_fields(&field.ty);
    quote! {substrait_expr::helpers::schema::NamesOnlySchemaNode {
        name: #name.to_string(),
        children: #children,
    }}
}

// Convert rust code to a vector of NamesOnlySchemaNode
//
// Example input:
//  { a: { b: {}, c: { d: {} } } }
//
// Example Output:
// vec![
//   NamesOnlySchemaNode { name: "a".to_string(), children: vec![
//     NamesOnlySchemaNode { name: "b".to_string(), children: vec![] },
//     NamesOnlySchemaNode { name: "c".to_string(), children: vec![
//       NamesOnlySchemaNode { name: "d".to_string(), children: vec![] }
//     ]}
//   ]}
// ]
fn rust_to_names_fields(schema: &NestedType) -> proc_macro2::TokenStream {
    let parsed_fields = schema
        .fields
        .iter()
        .map(|field| rust_field_to_names_field(field))
        .collect::<Vec<_>>();
    quote! {vec![#(#parsed_fields),*]}
}

// Convert rust code to a root NamesOnlySchemaNode (that has the empty string for a name)
//
// Example input:
//  { a: { b: {}, c: { d: {} } } }
//
// Example Output:
// NamesOnlySchemaNode {
//   name: "".to_string(),
//   vec![
//     NamesOnlySchemaNode { name: "a".to_string(), children: vec![
//       NamesOnlySchemaNode { name: "b".to_string(), children: vec![] },
//       NamesOnlySchemaNode { name: "c".to_string(), children: vec![
//         NamesOnlySchemaNode { name: "d".to_string(), children: vec![] }
//       ]}
//     ]}
//   ]
// }
fn rust_to_names_schema(schema: &NestedType) -> proc_macro2::TokenStream {
    let children = rust_to_names_fields(schema);
    quote! {
        substrait_expr::helpers::schema::SchemaInfo::Names(substrait_expr::helpers::schema::NamesOnlySchema::new(#children))
    }
}

// New rust syntax for a field in a names only schema
//
// Examples:
//  foo: {}
//  blah: { x: {}, y: { z: {} } }
struct Field {
    name: syn::Ident,
    _colon_token: syn::Token![:],
    ty: NestedType,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Field {
            name: input.parse()?,
            _colon_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}

// New rust syntax for a nested names-only type ({field, field, field})
//
// Examples:
//  { foo: {} }
//  { blah: { x: {}, y: { z: {} } } }
struct NestedType {
    _brace_token: syn::token::Brace,
    fields: syn::punctuated::Punctuated<Field, syn::Token![,]>,
}

impl Parse for NestedType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _brace_token: syn::braced!(content in input),
            fields: content.parse_terminated(Field::parse, syn::Token![,])?,
        })
    }
}

fn names_schema2(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let struct_type: NestedType = syn::parse2(input)?;
    Ok(rust_to_names_schema(&struct_type))
}

/// A macro to create names-only schemas from a dictionary-like rust syntax
///
/// # Examples
/// ```ignore
/// use substrait_expr::macros::names_schema;
///
/// let schema = names_schema!({
///   vector: {},
///   metadata: {
///     caption: {},
///     user_score: {}
///  }
/// });
/// ```
#[proc_macro]
pub fn names_schema(input: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    names_schema2(input).unwrap().into()
}
