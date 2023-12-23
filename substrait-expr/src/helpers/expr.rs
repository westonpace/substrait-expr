use substrait::proto::{
    expression::{field_reference::ReferenceType, Literal, RexType},
    Expression, Type,
};

use crate::{
    error::{Result, SubstraitExprError},
    util::HasRequiredPropertiesRef,
};

use super::{
    literals::{LiteralExt, LiteralInference},
    schema::SchemaInfo,
};

/// Extends the protobuf Expression object with useful helper methods
pub trait ExpressionExt {
    /// The rex_type is a required property
    ///
    /// This converts an expression's Option<&RexType> into Result<&RexType>
    /// that fails with a "required property missing" error
    ///
    /// TODO: Should this be public?
    fn try_rex_type(&self) -> Result<&RexType>;
    /// Tries to decode the expression as a rust literal of the given type
    fn try_as_rust_literal<T: LiteralInference>(&self) -> Result<T>;
    /// Tries to decode the expression as a Substrait literal
    fn try_as_literal(&self) -> Result<&Literal>;
    /// Determines the output type of the expression
    ///
    /// TODO: Explain this more
    fn output_type(&self, schema: &SchemaInfo) -> Result<Type>;
}

impl ExpressionExt for Expression {
    fn try_rex_type(&self) -> Result<&RexType> {
        self.rex_type.as_ref().ok_or_else(|| {
            SubstraitExprError::invalid_substrait(
                "The required property rex_type was missing from an expression",
            )
        })
    }

    fn try_as_rust_literal<T: LiteralInference>(&self) -> Result<T> {
        let literal = self.try_as_literal()?;
        T::try_from_substrait(literal.literal_type.as_ref().ok_or_else(|| {
            SubstraitExprError::invalid_substrait(
                "The required property literal_type was missing from a literal",
            )
        })?)
    }

    fn try_as_literal(&self) -> Result<&Literal> {
        match self.try_rex_type()? {
            RexType::Literal(literal) => Ok(literal),
            _ => Err(SubstraitExprError::invalid_substrait(
                "Expected a literal but received something else",
            )),
        }
    }

    fn output_type(&self, schema: &SchemaInfo) -> Result<Type> {
        match self.try_rex_type()? {
            RexType::Literal(literal) => literal.data_type(),
            RexType::ScalarFunction(func) => func.output_type.required("output_type").cloned(),
            RexType::Selection(selection) => {
                match selection.root_type.as_ref().required("root_type")? {
                    substrait::proto::expression::field_reference::RootType::Expression(_) => {
                        todo!()
                    }
                    substrait::proto::expression::field_reference::RootType::RootReference(_) => {
                        match selection.reference_type.as_ref().required("reference_type")? {
                            ReferenceType::DirectReference(root_segment) => {
                                schema.resolve_type(root_segment)
                            },
                            ReferenceType::MaskedReference(_) => {
                                Err(SubstraitExprError::invalid_substrait("A root reference did not have a reference type of direct reference"))
                            }
                        }
                    }
                    substrait::proto::expression::field_reference::RootType::OuterReference(_) => {
                        todo!()
                    }
                }
            }
            _ => todo!(),
        }
    }
}
