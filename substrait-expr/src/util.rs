use crate::error::{Result, SubstraitExprError};

/// Helper trait for extracting a property that should always be present
/// from a protobuf message and returning an error if it is not
pub(crate) trait HasRequiredPropertiesRef<T> {
    fn required(&self, prop_name: &str) -> Result<&T>;
}

impl<T> HasRequiredPropertiesRef<T> for Option<T> {
    fn required(&self, prop_name: &str) -> Result<&T> {
        self.as_ref().ok_or_else(|| {
            SubstraitExprError::InvalidSubstrait(format!(
                "The required property {} is missing",
                prop_name
            ))
        })
    }
}
