//! # Builders to create expressions and schemas
//!
//! This module contains utilities for programmatically creating expressions, schemas,
//! and types.
//!
//! ## Overview
//!
//! To create expressions you first create a schema, then an expression builder, and
//! then add the expressions you want, and finally build a message.  Here is an example
//! creating an ExtendedExpression message with a single expression `x+3`
//!
//! ```
//! use substrait_expr::builder::schema::SchemaBuildersExt;
//! use substrait_expr::helpers::schema::SchemaInfo;
//! use substrait_expr::helpers::types;
//! use substrait_expr::{
//!     builder::{BuilderParams, ExpressionsBuilder},
//!     functions::functions_arithmetic::FunctionsArithmeticExt,
//!     helpers::literals::literal,
//! };
//!
//! let schema = SchemaInfo::new_full()
//!     .field("score", types::i32(false))
//!     .nested("location", false, |builder| {
//!         builder
//!             .field("x", types::fp32(false))
//!             .field("y", types::fp64(true))
//!     })
//!     .build();
//!
//! let builder = ExpressionsBuilder::new(schema, BuilderParams::default());
//!
//! builder
//!     .add_expression(
//!         "sum",
//!         builder
//!             .functions()
//!             .add(
//!                 builder.fields().resolve_by_name("location.x").unwrap(),
//!                 literal(3.0_f32),
//!             )
//!             .build()
//!             .unwrap(),
//!     )
//!     .unwrap();
//!
//! let expressions = builder.build();
//! ```
//!
//! ## Creating a Schema
//!
//! Before you can create any expressions you will need a schema.  There are four
//! different kinds of schemas, unknown, names only, types only, and full.  Which one
//! you create will depend on how much information you know about the input fields.
//! For more information see the docs on [schema resolution](crate#schema-resolution).
//!
//! Creating an empty schema is simple.
//!
//! ```
//! # use substrait_expr::helpers::schema::EmptySchema;
//! # use substrait_expr::helpers::schema::SchemaInfo;
//!
//! let schema = SchemaInfo::Empty(EmptySchema::default());
//! ```
//!
//! The rest of the schema types have builders.
//!
//! ```
//! use substrait_expr::builder::schema::SchemaBuildersExt;
//! use substrait_expr::helpers::schema::SchemaInfo;
//! use substrait_expr::helpers::types;
//!
//! // Constructing a schema for
//! // {
//! //   "score": fp32?,
//! //   "location": {
//! //     "x": fp64,
//! //     "y": fp64
//! //   }
//! // }
//!
//! // Names only
//! let schema = SchemaInfo::new_names()
//!     .field("score")
//!     .nested("location", |builder| builder.field("x").field("y"));
//!
//! // Types only
//! let schema = SchemaInfo::new_types()
//!     .field(types::fp32(true))
//!     .nested(false, |builder| {
//!         builder.field(types::fp64(false)).field(types::fp64(false))
//!     })
//!     .build();
//!
//! // Full schema
//! // TODO
//! ```
//!
//! If you need to use *parameterized types* or *user defined types* then you
//! can use the schema builder to create those as well.  This works because
//! every schema builder also has a type registry that gets passed through to
//! the created schema.
//!
//! ```
//! use substrait_expr::builder::schema::SchemaBuildersExt;
//! use substrait_expr::helpers::schema::SchemaInfo;
//!
//! let builder = SchemaInfo::new_types();
//! let complex_number = builder
//!     .types()
//!     .user_defined("https://imaginary.com/types", "complex-number");
//! let schema = builder.field(complex_number.with_nullability(true)).build();
//! ```
//!
//! There are also utility macros for creating schemas.  These are mainly
//! used in unit tests since they require you to know the fields in the schema at
//! compile time.
//!
//! ```
//! use substrait_expr::macros::names_schema;
//!
//! // Names only
//! let schema = names_schema!({
//!     score: {},
//!     location: {
//!        x: {},
//!        y: {}
//!     }
//! });
//!
//! // Types only
//! // TODO
//!
//! // Full
//! // TODO
//! ```
//!
//! ## Creating Expressions
//!
//! Once you have a schema you can create an expression builder and start creating
//! expressions.  One important thing to note is that expressions in Substrait
//! cannot stand alone.  They must either be part of a Plan or part of an
//! ExtendedExpression.  An ExtendedExpression is a collection of expressions plus
//! schema/type/function metadata.  This is what the expression builder creates.
//!
//! There is an example above covering the entire process.
//!
//! ### Referencing fields
//!
//! To reference a field in the schema you can use
//! [crate::builder::ExpressionsBuilder::fields].
//!
//! You can reference fields by name
//!
//! ```
//! # use substrait_expr::helpers::schema::EmptySchema;
//! # use substrait_expr::helpers::schema::SchemaInfo;
//! # use substrait_expr::builder::{BuilderParams, ExpressionsBuilder};
//!
//! # let schema = SchemaInfo::Empty(EmptySchema::default());
//! # let builder = ExpressionsBuilder::new(schema, BuilderParams::new_loose());
//! let reference = builder.fields().resolve_by_name("location.x").unwrap();
//! ```
//!
//! The syntax for referencing fields by name is fairly simplistic.  The `.`
//! character will choose a subfield.  To choose a list item you can use `[]`.
//!
//! ```
//! # use substrait_expr::helpers::schema::EmptySchema;
//! # use substrait_expr::helpers::schema::SchemaInfo;
//! # use substrait_expr::builder::{BuilderParams, ExpressionsBuilder};
//!
//! # let schema = SchemaInfo::Empty(EmptySchema::default());
//! # let builder = ExpressionsBuilder::new(schema, BuilderParams::new_loose());
//! let list_item = builder.fields().resolve_by_name("genres[3]").unwrap();
//! ```
//!
//! If you have a map column and the map-key is string then you can also
//! reference it with `[]`.
//!
//! ```
//! # use substrait_expr::helpers::schema::EmptySchema;
//! # use substrait_expr::helpers::schema::SchemaInfo;
//! # use substrait_expr::builder::{BuilderParams, ExpressionsBuilder};
//!
//! # let schema = SchemaInfo::Empty(EmptySchema::default());
//! # let builder = ExpressionsBuilder::new(schema, BuilderParams::new_loose());
//! let map_item = builder.fields().resolve_by_name("metadata[size]").unwrap();
//! ```

use std::cell::RefCell;

use substrait::proto::expression_reference::ExprType;
use substrait::proto::{Expression, ExpressionReference, ExtendedExpression};

use crate::error::{Result, SubstraitExprError};
use crate::helpers::expr::ExpressionExt;
use crate::helpers::schema::SchemaInfo;
use crate::helpers::types::TypeExt;

use self::functions::FunctionsBuilder;
use self::schema::RefBuilder;

pub mod functions;
pub mod schema;
pub mod types;

pub struct BuilderParams {
    pub allow_late_name_lookup: bool,
    pub allow_loose_types: bool,
    pub allow_unknown_types: bool,
}

impl Default for BuilderParams {
    fn default() -> Self {
        Self {
            allow_late_name_lookup: false,
            allow_loose_types: false,
            allow_unknown_types: false,
        }
    }
}

impl BuilderParams {
    pub fn new_loose() -> Self {
        Self {
            allow_late_name_lookup: true,
            allow_loose_types: true,
            allow_unknown_types: true,
        }
    }
}

struct NamedExpression {
    expr: Expression,
    output_names: Vec<String>,
}

impl NamedExpression {
    fn try_new(expr: Expression, output_names: Vec<String>, schema: &SchemaInfo) -> Result<Self> {
        let expr_type = expr.output_type(schema)?;
        let num_types = expr_type.num_types();
        let num_names = output_names.len() as u32;
        if num_types != num_names {
            Err(SubstraitExprError::InvalidInput(format!(
                "An expression was given that returns {} types but only {} names were given",
                num_types, num_names
            )))
        } else {
            Ok(Self { expr, output_names })
        }
    }
}

/// A builder object to create expressions
///
/// Note that the output of this builder is not an "Expression" message.  Expression is not
/// a top-level message in the Substrait specification because an expression
/// references a schema and various extension metadata.  Instead, the top level message is
/// ExtendedExpression, which holds a collection of expressions.  If you only need to serialize
/// a single expression then you can create an ExtendedExpression that contains a single expression.
pub struct ExpressionsBuilder {
    schema: SchemaInfo,
    params: BuilderParams,
    expressions: RefCell<Vec<NamedExpression>>,
}

pub trait IntoExprOutputNames {
    fn into_names(self) -> Vec<String>;
}

impl<'a> IntoExprOutputNames for &'a str {
    fn into_names(self) -> Vec<String> {
        vec![self.to_string()]
    }
}

impl IntoExprOutputNames for String {
    fn into_names(self) -> Vec<String> {
        vec![self]
    }
}

impl IntoExprOutputNames for Vec<String> {
    fn into_names(self) -> Vec<String> {
        self
    }
}

impl ExpressionsBuilder {
    pub fn new(schema: SchemaInfo, params: BuilderParams) -> Self {
        Self {
            schema,
            params,
            expressions: RefCell::new(Vec::new()),
        }
    }

    pub fn fields(&self) -> RefBuilder {
        RefBuilder::new(&self.schema, &self.params, self.functions())
    }

    pub fn functions(&self) -> FunctionsBuilder {
        FunctionsBuilder::new(&self.schema)
    }

    pub fn add_expression(
        &self,
        output_names: impl IntoExprOutputNames,
        expression: Expression,
    ) -> Result<&Self> {
        let mut expressions = self.expressions.borrow_mut();
        expressions.push(NamedExpression::try_new(
            expression,
            output_names.into_names(),
            &self.schema,
        )?);
        Ok(self)
    }

    pub fn build(self) -> ExtendedExpression {
        let (extension_uris, extensions) = self.schema.extensions_registry().to_substrait();
        let referred_expr = self
            .expressions
            .into_inner()
            .into_iter()
            .map(|named_expr| ExpressionReference {
                output_names: named_expr.output_names,
                expr_type: Some(ExprType::Expression(named_expr.expr)),
            })
            .collect::<Vec<_>>();
        ExtendedExpression {
            version: Some(substrait::version::version_with_producer("substrait-expr")),
            extension_uris,
            extensions,
            advanced_extensions: None,
            expected_type_urls: Vec::new(),
            base_schema: Some(self.schema.to_substrait()),
            referred_expr,
        }
    }
}

#[cfg(test)]
mod tests {
    use substrait_expr_macros::names_schema;

    use super::*;
    use crate as substrait_expr;

    #[test]
    fn prevent_unknown_types_via_unknown_field_ref() {
        let params = BuilderParams {
            allow_unknown_types: false,
            ..Default::default()
        };
        let schema = names_schema!({
            x: {}
        });
        let builder = ExpressionsBuilder::new(schema, params);
        assert!(builder.fields().resolve_by_name("x").is_err());
        assert!(builder.fields().field_builder().field("x").is_err());
    }
}
