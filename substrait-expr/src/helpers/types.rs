use substrait::proto::{
    r#type::{
        parameter::Parameter, Decimal, FixedBinary, FixedChar, Kind, List, Map, Nullability,
        Struct, VarChar,
    },
    Type,
};

use crate::error::{Result, SubstraitExprError};
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
                Kind::UserDefinedTypeReference(_) => panic!(
                    "substrait-expr does not support the legacy UserDefinedTypeReference message"
                ),
                Kind::Uuid(uuid) => readify!("uuid", uuid, registry),
                Kind::Varchar(varchar) => {
                    readify!("varchar", varchar, registry, varchar.length)
                }
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

macro_rules! simple_type {
    ($kind:ident, $typname:ident, $nullable:expr) => {
        Type {
            kind: Some(substrait::proto::r#type::Kind::$kind(
                substrait::proto::r#type::$typname {
                    nullability: nullability($nullable),
                    type_variation_reference: NO_VARIATION,
                },
            )),
        }
    };
}

macro_rules! simple_infer {
    ($kind:ident) => {
        simple_infer!($kind, $kind);
    };
    ($kind:ident, $typname: ident) => {
        fn as_substrait(nullable: bool) -> Type {
            simple_type!($kind, $typname, nullable)
        }
    };
}

impl TypeInfer for i8 {
    simple_infer!(I8);
}
impl TypeInfer for i16 {
    simple_infer!(I16);
}
impl TypeInfer for i32 {
    simple_infer!(I32);
}
impl TypeInfer for i64 {
    simple_infer!(I64);
}
impl TypeInfer for bool {
    simple_infer!(Bool, Boolean);
}
impl TypeInfer for f32 {
    simple_infer!(Fp32);
}
impl TypeInfer for f64 {
    simple_infer!(Fp64);
}
impl TypeInfer for String {
    simple_infer!(String);
}
impl TypeInfer for &str {
    simple_infer!(String);
}
impl TypeInfer for &[u8] {
    simple_infer!(Binary);
}
impl TypeInfer for Vec<u8> {
    simple_infer!(Binary);
}

#[cfg(feature = "chrono")]
impl TypeInfer for chrono::naive::NaiveDate {
    simple_infer!(Date);
}

#[cfg(feature = "chrono")]
impl TypeInfer for chrono::naive::NaiveTime {
    simple_infer!(Time);
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
/// Create an instance of the timestamp type
pub fn timestamp(nullable: bool) -> Type {
    simple_type!(Timestamp, Timestamp, nullable)
}
/// Create an instance of the timestamp_tz type
pub fn timestamp_tz(nullable: bool) -> Type {
    simple_type!(TimestampTz, TimestampTz, nullable)
}
/// Create an instance of the date type
pub fn date(nullable: bool) -> Type {
    simple_type!(Date, Date, nullable)
}
/// Create an instance of the time type
pub fn time(nullable: bool) -> Type {
    simple_type!(Time, Time, nullable)
}
/// Create an instance of the interval_year type
pub fn interval_year(nullable: bool) -> Type {
    simple_type!(IntervalYear, IntervalYear, nullable)
}
/// Create an instance of the interval_day type
pub fn interval_day(nullable: bool) -> Type {
    simple_type!(IntervalDay, IntervalDay, nullable)
}
/// Create an instance of the uuid type
pub fn uuid(nullable: bool) -> Type {
    simple_type!(Uuid, Uuid, nullable)
}
/// Create an instance of the fixed char type
///
/// Although length is a u32 the range allowed by Substrait is only those
/// accepted by positive i32 values.  In other words [1, 2^32-1]
///
/// An error is returned if the length is out of range
pub fn fixed_char(length: u32, nullable: bool) -> Result<Type> {
    let length = i32::try_from(length).map_err(|_| {
        SubstraitExprError::invalid_input(
            "Substrait does not support fixed_char types with length greater than 2^31-1",
        )
    })?;
    Ok(Type {
        kind: Some(Kind::FixedChar(FixedChar {
            length,
            nullability: nullability(nullable),
            type_variation_reference: NO_VARIATION,
        })),
    })
}
/// Create an instance of the varchar type
///
/// Although length is a u32 the range allowed by Substrait is only those
/// accepted by positive i32 values.  In other words [1, 2^32-1]
///
/// An error is returned if the length is out of range
pub fn varchar(length: u32, nullable: bool) -> Result<Type> {
    let length = i32::try_from(length).map_err(|_| {
        SubstraitExprError::invalid_input(
            "Substrait does not support fixed_char types with length greater than 2^31-1",
        )
    })?;
    Ok(Type {
        kind: Some(Kind::Varchar(VarChar {
            length,
            nullability: nullability(nullable),
            type_variation_reference: NO_VARIATION,
        })),
    })
}
/// Create an instance of the fixed binary type
///
/// Although length is a u32 the range allowed by Substrait is only those
/// accepted by positive i32 values.  In other words [1, 2^32-1]
///
/// An error is returned if the length is out of range
pub fn fixed_binary(length: u32, nullable: bool) -> Result<Type> {
    let length = i32::try_from(length).map_err(|_| {
        SubstraitExprError::invalid_input(
            "Substrait does not support fixed_binary types with length greater than 2^31-1",
        )
    })?;
    Ok(Type {
        kind: Some(Kind::FixedBinary(FixedBinary {
            length,
            nullability: nullability(nullable),
            type_variation_reference: NO_VARIATION,
        })),
    })
}
/// Create an instance of the decimal type
///
/// `precision` must be in the range (0, 38]
/// `scale` must be in the range [0, precision]
///
/// Returns an error if precision or scale are out of bounds
pub fn decimal(precision: u8, scale: u8, nullable: bool) -> Result<Type> {
    if precision == 0 || precision > 38 {
        Err(SubstraitExprError::InvalidInput(format!(
            "invalid precision ({}), must be in the range (0, 38]",
            precision
        )))
    } else if scale > precision {
        Err(SubstraitExprError::InvalidInput(format!(
            "invalid scale ({}) given precision ({}), scale must be less than or equal to precision",
            scale, precision
        )))
    } else {
        Ok(Type {
            kind: Some(Kind::Decimal(Decimal {
                precision: precision as i32,
                scale: scale as i32,
                nullability: nullability(nullable),
                type_variation_reference: NO_VARIATION,
            })),
        })
    }
}
/// Create an instance of the list type
pub fn list(item_type: Type, nullable: bool) -> Type {
    Type {
        kind: Some(Kind::List(Box::new(List {
            r#type: Some(Box::new(item_type)),
            nullability: nullability(nullable),
            type_variation_reference: NO_VARIATION,
        }))),
    }
}
/// Create an instance of the map type
pub fn map(key_type: Type, value_type: Type, nullable: bool) -> Type {
    Type {
        kind: Some(Kind::Map(Box::new(Map {
            key: Some(Box::new(key_type)),
            value: Some(Box::new(value_type)),
            nullability: nullability(nullable),
            type_variation_reference: NO_VARIATION,
        }))),
    }
}
/// Create an instance of the struct type
pub fn struct_(children: Vec<Type>, nullable: bool) -> Type {
    Type {
        kind: Some(Kind::Struct(Struct {
            types: children,
            nullability: nullability(nullable),
            type_variation_reference: NO_VARIATION,
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
