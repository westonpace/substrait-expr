use substrait::proto::{
    r#type::{
        Binary, Boolean, Fp32, Fp64, Kind, Nullability, String as SubstraitString, Struct, I16,
        I32, I64, I8,
    },
    Type,
};

use crate::error::Result;
use crate::util::HasRequiredPropertiesRef;

use super::registry::ExtensionsRegistry;

/// Helper methods for substrait Type objects
pub trait TypeExt {
    /// Return true if two types are the same "kind", ignoring nullability and type parameters
    fn same_kind(&self, other: &Type) -> Result<bool>;
    /// Returns true if this is the unknown type
    fn is_unknown(&self, registry: &ExtensionsRegistry) -> bool;
    /// Returns the total number of types (including this one) represented by this type
    ///
    /// Will return 1 if this is not a struct type
    fn num_types(&self) -> u32;
    /// Returns the child types
    fn children(&self) -> Vec<&Type>;
}

impl TypeExt for Type {
    fn same_kind(&self, other: &Type) -> Result<bool> {
        let self_kind = self.kind.required("kind")?;
        let other_kind = other.kind.required("kind")?;
        Ok(std::mem::discriminant(self_kind) == std::mem::discriminant(other_kind))
    }

    fn is_unknown(&self, registry: &ExtensionsRegistry) -> bool {
        match &self.kind {
            Some(Kind::UserDefined(user_defined)) => {
                let type_name = registry.lookup_type(user_defined.type_reference);
                match type_name {
                    Some(type_name) => {
                        type_name.uri == UNKNOWN_TYPE_URI && type_name.name == UNKNOWN_TYPE_NAME
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }

    fn num_types(&self) -> u32 {
        match &self.kind {
            Some(Kind::Struct(strct)) => {
                strct.types.iter().map(|typ| typ.num_types()).sum::<u32>() + 1
            }
            _ => 1,
        }
    }

    fn children(&self) -> Vec<&Type> {
        match &self.kind {
            Some(Kind::Struct(strct)) => strct.types.iter().collect(),
            _ => vec![],
        }
    }
}

pub(crate) const fn nullability(nullable: bool) -> i32 {
    if nullable {
        Nullability::Nullable as i32
    } else {
        Nullability::Required as i32
    }
}

/// This trait helps convert from rust types to substrait types
///
/// It's implemented for all the standard types
///
/// Implement this to use methods like
/// [`from_rust`](crate::helpers::types::from_rust) on your own user defined types
pub trait TypeInfer {
    /// Return an instance of the substrait type for this type
    fn as_substrait(nullable: bool) -> Type;
}

impl TypeInfer for i8 {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::I8(I8 {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for i16 {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::I16(I16 {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for i32 {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::I32(I32 {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for i64 {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::I64(I64 {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for bool {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::Bool(Boolean {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for f32 {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::Fp32(Fp32 {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for f64 {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::Fp64(Fp64 {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for String {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::String(SubstraitString {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for &str {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::String(SubstraitString {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for &[u8] {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::Binary(Binary {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

impl TypeInfer for Vec<u8> {
    fn as_substrait(nullable: bool) -> Type {
        Type {
            kind: Some(substrait::proto::r#type::Kind::Binary(Binary {
                nullability: nullability(nullable),
                type_variation_reference: 0,
            })),
        }
    }
}

/// Create a substrait type from a rust type
pub fn from_rust<T: TypeInfer>(nullable: bool) -> Type {
    <T as TypeInfer>::as_substrait(nullable)
}
/// Create an instance of the bool type
pub fn bool(nullable: bool) -> Type {
    from_rust::<bool>(nullable)
}
/// Create an instance of the i8 type
pub fn i8(nullable: bool) -> Type {
    from_rust::<i8>(nullable)
}
/// Create an instance of the i16 type
pub fn i16(nullable: bool) -> Type {
    from_rust::<i16>(nullable)
}
/// Create an instance of the i32 type
pub fn i32(nullable: bool) -> Type {
    from_rust::<i32>(nullable)
}
/// Create an instance of the i64 type
pub fn i64(nullable: bool) -> Type {
    from_rust::<i64>(nullable)
}
/// Create an instance of the fp32 type
pub fn fp32(nullable: bool) -> Type {
    from_rust::<f32>(nullable)
}
/// Create an instance of the fp64 type
pub fn fp64(nullable: bool) -> Type {
    from_rust::<f64>(nullable)
}
/// Create an instance of the string type
pub fn string(nullable: bool) -> Type {
    from_rust::<&str>(nullable)
}
/// Create an instance of the binary type
pub fn binary(nullable: bool) -> Type {
    from_rust::<&[u8]>(nullable)
}
/// Create an instance of the struct type
pub fn struct_(nullable: bool, children: Vec<Type>) -> Type {
    Type {
        kind: Some(Kind::Struct(Struct {
            types: children,
            nullability: nullability(nullable),
            ..Default::default()
        })),
    }
}
/// The URI of the unknown type
pub const UNKNOWN_TYPE_URI: &'static str = "https://substrait.io/types";
/// The name of the unknown type
pub const UNKNOWN_TYPE_NAME: &'static str = "unknown";
/// A friendly name that indicates there is no type variation being used
pub const NO_VARIATION: u32 = 0;
