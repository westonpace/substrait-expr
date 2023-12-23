//! # Susbstrait Expressions
//!
//! This crate provides utilities for working with Substrait expressions.  Substrait is a platform-independent
//! specification for relational algreba plans (often called query plans).  Expressions are one piece of a query
//! plan, typically found in filters and projections.  However, expressions can be used on their own, outside
//! of a query plan.
//!
//! ## What is Provided
//!
//! * An [expression builder](crate::builder) is provided which makes it easy to programmatically
//!   create expressions
//! * Helper functions make it easy to get information about parts of an expression
//! * (TODO) Utilities for converting to/from other Rust libraries
//! * (TODO) An SQL parser allows you to create expressions from SQL strings
//!
//! ## Who Should Use This
//!
//! Common uses cases for this library include:
//!
//! * Users trying to create Substrait bindings for their own libraries, e.g. to map Substrait's
//!   expressions to a library's internal expressions concept.
//! * Users that need to use multiple query libraries and want a common starting point.
//! * Users who have found that the library they are using supports Substrait but does not
//!   have all of the expression building capabilities they want.
//!
//! Otherwise, if you are happily using a library like polars or datafusion, which has its own
//! expressions concept, you won't gain much from this library (except perhaps a different
//! API)
//!
//! ## Expressions Overview
//!
//! This section gives an overview of the core concepts and assumes only a basic familiarity with
//! Substrait.  If you only need to know how to use this library then you can skip this section and
//! visit one of the sections mentioned above.
//!
//! ### What are Expressions?
//!
//! Expressions are programming statements that describe how to calculate a value.  They are often
//! used to describe filters, to calculate new columns based on input, or to select some portion
//! of an input value.  The following are all expressions (shown as SQL):
//!
//! ```sql
//! x
//! x + 3
//! x < 7 OR x > 50
//! x + y.z
//! ```
//!
//! ### Schemas
//!
//! A schema is not actually part of an expression but, in Substrait, it often accompanies an
//! expression.  A schema describes what is known about the input data.  For example, when we
//! query a table in a database the table typically has a schema describing the name and type
//! of each column.
//!
//! #### Schema Resolution
//!
//! Different parts of an application often know different amounts of schema information.
//!
//! An **empty** schema doesn't know anything about the input data.  We don't know how many
//! fields there are, what their names are, or what the data types are.  An empty schema
//! will produce a *loose* expression.  Other systems sometimes refer to these expressions as
//! *unresolved* or *unbound* expressions.
//!
//! There is not much that can be done with loose expressions.  They cannot be optimized,
//! validated, or executed.  However, they are often useful for converting to Substrait
//! early in an application.  For example an application may parse SQL queries/statements
//! at the edge and will need to send these statements to a data server where the schema
//! will be looked up in a catalog.  At that point the expression can be *bound* to the
//! schema to create a types only or full schema.
//!
//! A **names only** schema describes the names of the input columns but not their types.
//! This is rather unusual in practice and support for these schemas is mainly included
//! for the sake of completeness.
//!
//! A **types only** schema describes the types of the input columns but not their names.
//! This is very common.  Knowing the types of fields is enough information for validation,
//! optimization, and execution.  The core of Substrait is designed with types only systems
//! in mind.  Users sometimes find it strange to throw away the names and only work with
//! types.  However, once plans begin to get optimized the names start to have less meaning.
//! For example, an expression may be refactored into several common expressions and those
//! common expressions might not have meaningful names.
//!
//! A **full** schema describes the types and names of all of the input columns.  This is
//! also very common, especially for components close to the user.  The main advantage of
//! knowing the names in addition to the types is to make plans more human readable and
//! debuggable.
//!
//! This library aims to support all four types of schemas/plans.  There are utilities for
//! building schemas and types in the builders module.  There are helper methods for working
//! with schemas and types in the helpers module.
//!
//! ### AST
//!
//! The expression itself is an abstract syntax tree that is made up of three different
//! kinds of nodes, function calls, field references, and literals.  Field references
//! can either reference the schema or they can reference the return value of the previous
//! expression (typically to select a portion of a complex return value).  Utilities
//! for building all three types of nodes can be found in the builders module.
//!
//! ### Substrait Extensions
//!
//! In order to support as many scenarios as possible this library works with some Substrait
//! extensions.
//!
//! #### Unknown Type
//!
//! All nodes in the expression AST have types.  When a schema does not have type information
//! then field references created by that schema will have the unknown type.  This is a special
//! type that can fill in for any other type in a function call, changing the function call's
//! return type to the unknown type.  For example, add(int32, unknown) will be a valid function
//! call and will return *unknown*.
//!
//! #### Name Lookups (TODO)
//!
//! Sometimes users will ask for a field by name.  For example, they expression `x + y` refers
//! to fields `x` and `y`.  If our schema is not aware of names then we cannot perform this
//! lookup.  If name lookups are enabled then these field references will turn into a special
//! "name lookup" AST node.  This behaves just like a field reference whose field is the
//! unknown type.
//!
//! #### Name Annotations (TODO)
//!
//! AST nodes in Substrait do not have names.  Some expression libraries support naming AST
//! nodes.  For example, an SQL query contain `x + y AS foo`.  Here, `foo` is the name of
//! a function call node.  We aim to be able to round trip this plan without losing that
//! name information.  We do this by attaching a name annotation to the AST node.

pub mod builder;
pub mod error;
/// # Function definitions for common functions
///
/// This module contains code that has been generated from the
/// [YAML files](https://github.com/substrait-io/substrait/tree/main/extensions) included alongside
/// the spec.  These are functions that are generally available in many different consumer libraries
/// and are often considered "standard" functions.
///
/// The generated code includes both [FunctionDefinition][crate::builder::functions::FunctionDefinition]
/// objects and traits that extend the [FunctionsBuilder][crate::builder::functions::FunctionsBuilder].
/// The easiest way to use these functions is through the trait objects.
///
/// ```
/// # use substrait_expr::helpers::schema::EmptySchema;
/// # use substrait_expr::helpers::schema::SchemaInfo;
/// # use substrait_expr::builder::{BuilderParams, ExpressionsBuilder};
/// // The extension trait provides the `add` method used below
/// use substrait_expr::functions::functions_arithmetic::FunctionsArithmeticExt;
///
/// # let schema = SchemaInfo::Empty(EmptySchema::default());
/// # let builder = ExpressionsBuilder::new(schema, BuilderParams::new_loose());
/// builder.functions().add(literal(3), literal(5));
/// ```
pub mod functions {
    include!(concat!(env!("OUT_DIR"), "/src/functions.rs"));
}
pub mod helpers;
pub(crate) mod util;

pub use substrait_expr_macros as macros;
