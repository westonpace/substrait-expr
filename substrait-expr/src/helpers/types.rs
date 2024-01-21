use std::ptr::null;

use substrait::proto::{
    r#type::{
        parameter::Parameter, Binary, Boolean, Fp32, Fp64, Kind, Nullability,
        String as SubstraitString, Struct, I16, I32, I64, I8,
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
    /// Returns a human readable name for the type
    ///
    /// The syntax for this is defined at https://substrait.io/types/type_parsing/#type-syntax-parsing
    fn to_human_readable(&self, registry: &ExtensionsRegistry) -> String;
}

// For some reason, prost is giving us i32 for nullability
const NULLABLE: i32 = Nullability::Nullable as i32;
const REQUIRED: i32 = Nullability::Required as i32;

fn null_str(nullability: i32) -> &'static str {
    match nullability {
        NULLABLE => "?",
        REQUIRED => "",
        _ => "INVALID-NULLABILITY",
    }
}

fn vari_str(variation_reference: u32, registry: &ExtensionsRegistry) -> String {
    if variation_reference == NO_VARIATION {
        String::new()
    } else {
        let qualified_name = registry.lookup_variation(variation_reference);
        if let Some(qualified_name) = qualified_name {
            format!("[{}#{}]", qualified_name.uri, qualified_name.name)
        } else {
            "[unknown_variation]".to_string()
        }
    }
}

// Helper macro that converts input to {}
macro_rules! hug {
    ($e:expr) => {
        "{}"
    };
}

// If there are any arguments it yields <{},{},...,{}>
// If there are no arguments it is the empty string
macro_rules! params_format_str {
    () => {""};
    ($first:expr$(, $param:expr),*) => {
        concat!("<",hug!($first) $(,",",hug!($param)),*, ">")
    }
}

macro_rules! readify {
    ($name:literal, $e:expr, $reg:expr $(, $param:expr)*) => {
        format!(
            concat!($name, "{}{}", params_format_str!($($param),*)),
            null_str($e.nullability),
            vari_str($e.type_variation_reference, $reg),
            $($param),*
        )
    };
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

    fn to_human_readable(&self, registry: &ExtensionsRegistry) -> String {
        if let Some(kind) = &self.kind {
            match kind {
                Kind::Binary(binary) => readify!("binary", binary, registry),
                Kind::Bool(bool) => readify!("bool", bool, registry),
                Kind::Date(date) => readify!("date", date, registry),
                Kind::Decimal(decimal) => readify!(
                    "decimal",
                    decimal,
                    registry,
                    decimal.precision,
                    decimal.scale
                ),
                Kind::FixedBinary(fixed_binary) => {
                    readify!("fixedbinary", fixed_binary, registry, fixed_binary.length)
                }
                Kind::FixedChar(fixed_char) => {
                    readify!("fixedchar", fixed_char, registry, fixed_char.length)
                }
                Kind::Fp32(fp32) => readify!("fp32", fp32, registry),
                Kind::Fp64(fp64) => readify!("fp64", fp64, registry),
                Kind::I16(i16) => readify!("i16", i16, registry),
                Kind::I32(i32) => readify!("i32", i32, registry),
                Kind::I64(i64) => readify!("i64", i64, registry),
                Kind::I8(i8) => readify!("i8", i8, registry),
                Kind::IntervalDay(interval_day) => readify!("interval_day", interval_day, registry),
                Kind::IntervalYear(interval_year) => {
                    readify!("interval_year", interval_year, registry)
                }
                Kind::List(list) => {
                    readify!(
                        "list",
                        list,
                        registry,
                        list.r#type
                            .as_ref()
                            .map(|typ| typ.to_human_readable(registry))
                            .unwrap_or("invalid_inner_type".to_string())
                    )
                }
                Kind::Map(map) => {
                    readify!(
                        "map",
                        map,
                        registry,
                        map.key
                            .as_ref()
                            .map(|key| key.to_human_readable(registry))
                            .unwrap_or("invalid_key_type".to_string()),
                        map.value
                            .as_ref()
                            .map(|val| val.to_human_readable(registry))
                            .unwrap_or("invalid_value_type".to_string())
                    )
                }
                Kind::String(string) => readify!("string", string, registry),
                Kind::Struct(strct) => {
                    let child_types = strct
                        .types
                        .iter()
                        .map(|typ| typ.to_human_readable(registry))
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(
                        "struct{}{}<{}>",
                        null_str(strct.nullability),
                        vari_str(strct.type_variation_reference, registry),
                        child_types
                    )
                }
                Kind::Time(time) => readify!("time", time, registry),
                Kind::Timestamp(timestamp) => readify!("timestamp", timestamp, registry),
                Kind::TimestampTz(timestamp_tz) => readify!("timestamp_tz", timestamp_tz, registry),
                Kind::UserDefined(user_defined) => {
                    let params = user_defined
                        .type_parameters
                        .iter()
                        .map(|param| match &param.parameter {
                            None => "invalid_type_parameter".to_string(),
                            Some(Parameter::DataType(typ)) => typ.to_human_readable(registry),
                            Some(Parameter::Boolean(bool)) => bool.to_string(),
                            Some(Parameter::Enum(val)) => val.clone(),
                            Some(Parameter::Integer(i)) => i.to_string(),
                            Some(Parameter::Null(_)) => "null".to_string(),
                            Some(Parameter::String(str)) => str.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let qualified_name = registry.lookup_type(user_defined.type_reference);
                    let name = if let Some(qualified_name) = qualified_name {
                        format!("{}#{}", qualified_name.uri, qualified_name.name)
                    } else {
                        "unknown_user_defined_type".to_string()
                    };
                    let params = if params.is_empty() {
                        "".to_string()
                    } else {
                        format!("<{}>", params)
                    };
                    format!(
                        "{}{}{}{}",
                        name,
                        null_str(user_defined.nullability),
                        vari_str(user_defined.type_variation_reference, registry),
                        params
                    )
                }
                _ => todo!(),
            }
        } else {
            "invalid-type".to_string()
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

#[cfg(test)]
mod tests {
    use substrait::proto::{
        r#type::{Binary, Decimal, FixedChar, Kind, List},
        Type,
    };

    use crate::helpers::{
        registry::ExtensionsRegistry,
        types::{nullability, TypeExt, NO_VARIATION},
    };

    #[test]
    fn test_human_readable() {
        let reg = ExtensionsRegistry::default();
        let my_variation = reg.register_variation("my_uri".to_string(), "my_variation");
        assert_eq!(
            "binary",
            Type {
                kind: Some(Kind::Binary(Binary {
                    type_variation_reference: NO_VARIATION,
                    nullability: nullability(false)
                }))
            }
            .to_human_readable(&reg)
        );
        assert_eq!(
            "binary?",
            Type {
                kind: Some(Kind::Binary(Binary {
                    type_variation_reference: NO_VARIATION,
                    nullability: nullability(true)
                }))
            }
            .to_human_readable(&reg)
        );
        assert_eq!(
            "binary?[my_uri#my_variation]",
            Type {
                kind: Some(Kind::Binary(Binary {
                    type_variation_reference: my_variation,
                    nullability: nullability(true)
                }))
            }
            .to_human_readable(&reg)
        );
        assert_eq!(
            "binary?[unknown_variation]",
            Type {
                kind: Some(Kind::Binary(Binary {
                    type_variation_reference: 5,
                    nullability: nullability(true)
                }))
            }
            .to_human_readable(&reg)
        );
        assert_eq!(
            "decimal?<38,6>",
            Type {
                kind: Some(Kind::Decimal(Decimal {
                    type_variation_reference: NO_VARIATION,
                    nullability: nullability(true),
                    precision: 38,
                    scale: 6
                }))
            }
            .to_human_readable(&reg)
        );
        assert_eq!(
            "list?<fixedchar<8>>",
            Type {
                kind: Some(Kind::List(Box::new(List {
                    type_variation_reference: NO_VARIATION,
                    nullability: nullability(true),
                    r#type: Some(Box::new(Type {
                        kind: Some(Kind::FixedChar(FixedChar {
                            type_variation_reference: NO_VARIATION,
                            nullability: nullability(false),
                            length: 8,
                        }))
                    }))
                })))
            }
            .to_human_readable(&reg)
        )
    }
}
