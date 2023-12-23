use substrait::proto::{
    expression::{literal::LiteralType, Literal},
    Expression, Type,
};

use crate::error::{Result, SubstraitExprError};

use super::types;

/// Extends the protobuf Literal object with useful helper methods
pub trait LiteralExt {
    /// Get the substrait type of a literal
    fn data_type(&self) -> Result<Type>;
}

impl LiteralExt for Literal {
    fn data_type(&self) -> Result<Type> {
        match &self.literal_type {
            Some(LiteralType::Binary(_)) => Ok(types::binary(self.nullable)),
            Some(LiteralType::Boolean(_)) => Ok(types::bool(self.nullable)),
            Some(LiteralType::Fp32(_)) => Ok(types::fp32(self.nullable)),
            Some(LiteralType::Fp64(_)) => Ok(types::fp64(self.nullable)),
            Some(LiteralType::I8(_)) => Ok(types::i8(self.nullable)),
            Some(LiteralType::I16(_)) => Ok(types::i16(self.nullable)),
            Some(LiteralType::I32(_)) => Ok(types::i32(self.nullable)),
            Some(LiteralType::I64(_)) => Ok(types::i64(self.nullable)),
            Some(LiteralType::Null(data_type)) => Ok(data_type.clone()),
            Some(LiteralType::String(_)) => Ok(types::string(self.nullable)),
            None => Err(SubstraitExprError::invalid_substrait(
                "Literal was missing required literal_type property",
            )),
            _ => todo!(),
        }
    }
}

/// A trait that helps convert from rust types to substrait types
///
/// This trait is implemented for all the standard rust types
///
/// Implementing this for your UDF will allow you to use methods like
/// [`try_as_rust_literal`](crate::helpers::expr::ExpressionExt::try_as_rust_literal)
/// on your own types.
pub trait LiteralInference {
    /// Convert self to a substrait literal
    fn to_substrait(self) -> LiteralType;
    /// Try to convert from a substrait literal to an instance of Self
    fn try_from_substrait(lit: &LiteralType) -> Result<Self>
    where
        Self: Sized;
}

impl LiteralInference for bool {
    fn to_substrait(self) -> LiteralType {
        LiteralType::Boolean(self)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::Boolean(value) => Ok(*value),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected a boolean literal but found {:?}", lit),
            )),
        }
    }
}

impl LiteralInference for i8 {
    fn to_substrait(self) -> LiteralType {
        LiteralType::I8(self as i32)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::I8(value) => i8::try_from(*value).map_err(|_| {
                crate::error::SubstraitExprError::invalid_substrait(
                    "The substrait message had an i8 literal but the value did not fit in an i8",
                )
            }),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected an int8 literal but found {:?}", lit),
            )),
        }
    }
}

impl LiteralInference for i16 {
    fn to_substrait(self) -> LiteralType {
        LiteralType::I16(self as i32)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::I16(value) => i16::try_from(*value).map_err(|_| {
                crate::error::SubstraitExprError::invalid_substrait(
                    "The substrait message had an i16 literal but the value did not fit in an i16",
                )
            }),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected an int16 literal but found {:?}", lit),
            )),
        }
    }
}

impl LiteralInference for i32 {
    fn to_substrait(self) -> LiteralType {
        LiteralType::I32(self)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::I32(value) => Ok(*value),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected an int32 literal but found {:?}", lit),
            )),
        }
    }
}

impl LiteralInference for i64 {
    fn to_substrait(self) -> LiteralType {
        LiteralType::I64(self as i64)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::I64(value) => Ok(*value),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected an int64 literal but found {:?}", lit),
            )),
        }
    }
}

impl LiteralInference for f32 {
    fn to_substrait(self) -> LiteralType {
        LiteralType::Fp32(self)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::Fp32(value) => Ok(*value),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected an fp32 literal but found {:?}", lit),
            )),
        }
    }
}

impl LiteralInference for f64 {
    fn to_substrait(self) -> LiteralType {
        LiteralType::Fp64(self)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::Fp64(value) => Ok(*value),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected an fp64 literal but found {:?}", lit),
            )),
        }
    }
}

impl LiteralInference for &str {
    fn to_substrait(self) -> LiteralType {
        LiteralType::String(self.to_owned())
    }
    fn try_from_substrait(_: &LiteralType) -> Result<Self> {
        todo!()
    }
}

impl LiteralInference for String {
    fn to_substrait(self) -> LiteralType {
        LiteralType::String(self)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::String(value) => Ok(value.to_string()),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected a string literal but found {:?}", lit),
            )),
        }
    }
}

impl LiteralInference for &[u8] {
    fn to_substrait(self) -> LiteralType {
        LiteralType::Binary(Vec::from(self))
    }
    fn try_from_substrait(_: &LiteralType) -> Result<Self> {
        todo!()
    }
}

impl LiteralInference for Vec<u8> {
    fn to_substrait(self) -> LiteralType {
        LiteralType::Binary(self)
    }
    fn try_from_substrait(lit: &LiteralType) -> Result<Self> {
        match lit {
            LiteralType::Binary(value) => Ok(value.clone()),
            _ => Err(crate::error::SubstraitExprError::invalid_substrait(
                format!("Expected a binary literal but found {:?}", lit),
            )),
        }
    }
}

const NO_TYPE_VARIATION: u32 = 0;

fn make_literal(lit_type: LiteralType, nullable: bool) -> Expression {
    Expression {
        rex_type: Some(substrait::proto::expression::RexType::Literal(Literal {
            nullable,
            type_variation_reference: NO_TYPE_VARIATION,
            literal_type: Some(lit_type),
        })),
    }
}

/// Methods for creating literals from rust
pub mod literals {
    use substrait::proto::expression::literal::{Struct, VarChar};

    use crate::{error::SubstraitExprError, helpers::expr::ExpressionExt};

    use super::*;

    /// Create a fixed-char literal
    ///
    /// There are three primary string types in Substrait, string, fixed-char, and var-char
    ///
    /// By default, rust string types coerce into Substrait string literals.
    /// This method will give you a fixed-char literal instead.
    pub fn fixed_char(value: impl Into<String>) -> Expression {
        make_literal(LiteralType::FixedChar(value.into()), false)
    }

    /// Create a fixed-binary literal
    ///
    /// There are two binary types in Substrait, binary, and fixed-binary
    ///
    /// By default, rust binary types coerce into Substrait binary literals.
    /// This method will give you a fixed-length binary instead.
    pub fn fixed_binary(value: Vec<u8>) -> Expression {
        make_literal(LiteralType::FixedBinary(value), false)
    }

    /// Create a var-char literal
    ///
    /// There are three primary string types in Substrait, string, fixed-char, and var-char
    ///
    /// By default, rust string types coerce into Substrait string literals.
    /// This method will give you a var-char literal instead.
    ///
    /// This method will return an error if the provided string is longer than length
    pub fn try_varchar(value: impl Into<String>, length: u32) -> Result<Expression> {
        let value = value.into();
        if (length as usize) < value.len() {
            Err(SubstraitExprError::invalid_input(format!(
                "String of length {} does not fit in a varchar literal field of length {}",
                value.len(),
                length
            )))
        } else {
            Ok(make_literal(
                LiteralType::VarChar(VarChar {
                    value: value.into(),
                    length,
                }),
                false,
            ))
        }
    }

    /// Create a struct literal
    ///
    /// `children` must all be literal expressions and will be the children of the struct
    pub fn try_struct(children: &[Expression]) -> Result<Expression> {
        let fields = children
            .iter()
            .map(|expr| expr.try_as_literal().cloned())
            .collect::<Result<Vec<_>>>()?;
        Ok(make_literal(LiteralType::Struct(Struct { fields }), false))
    }
}

/// Create a null literal of the given type
pub fn null_literal(data_type: Type) -> Expression {
    make_literal(LiteralType::Null(data_type), true)
}

/// Create a literal from a rust value
pub fn literal<T: LiteralInference>(value: T) -> Expression {
    make_literal(value.to_substrait(), false)
}

/// Createa a nullable literal from a rust value (unusual)
pub fn nullable_literal<T: LiteralInference>(value: T) -> Expression {
    make_literal(value.to_substrait(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literals() {
        let x = literal(1_i8);
        let y = literal(1_i16);
        let z = literal(1_i32);
        literals::try_struct(&[x, y, z]).unwrap();

        literal("hello");
        literals::fixed_char("hello");
        literals::fixed_binary(vec![0, 1, 2]);
        literals::try_varchar("hello", 30).unwrap();
        literal(vec![0, 1, 2]);

        assert!(literals::try_varchar("hello", 3).is_err());
    }
}
