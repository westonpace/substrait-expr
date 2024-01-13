//! # Helpers to extract information from Substrait plans
//!
//! Substrait objects are protobuf generated objects.  They generally
//! have getters and setters and nothing much useful beyond that.  This makes
//! protobuf objects difficult to work with.
//!
//! Rather than create yet another in-memory representation of Substrait messages
//! we chose, in this library, to stick with the underlying protobuf representation
//! as much as possible.  In some cases we could not.  This crate is a combination
//! of non-protobuf types (e.g. for things like the schema and registry) and extension
//! traits that add new functionality to the existing protobuf types.  There are
//! extension traits for [expressions](crate::helpers::expr::ExpressionExt),
//! [types](crate::helpers::expr::TypeExt), and [literals](crate::helpers::literals::LiteralExt)

pub mod expr;
pub mod literals;
pub mod registry;
pub mod schema;
pub mod types;
