use substrait::proto::{
    r#type::{Kind, UserDefined},
    Type,
};

use crate::helpers::{
    registry::ExtensionsRegistry,
    types::{nullability, NO_VARIATION, UNKNOWN_TYPE_NAME, UNKNOWN_TYPE_URI},
};

pub fn unknown(registry: &ExtensionsRegistry) -> Type {
    let anchor = registry.register_type(UNKNOWN_TYPE_URI.to_string(), UNKNOWN_TYPE_NAME);
    Type {
        kind: Some(Kind::UserDefined(UserDefined {
            nullability: nullability(true),
            type_parameters: vec![],
            type_reference: anchor,
            type_variation_reference: NO_VARIATION,
        })),
    }
}
