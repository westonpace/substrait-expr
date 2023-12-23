//! Error handling utilities for the crate
use thiserror::Error;

/// All errors raised by this crate will be instances of SubstraitExprError
#[derive(Error, Debug)]
pub enum SubstraitExprError {
    /// This indicates that a substrait message is invalid
    #[error("Invalid substrait: {0}")]
    InvalidSubstrait(String),
    /// This indicates that a user is trying to do something with the library that is invalid
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl SubstraitExprError {
    /// Shortcut for creating InvalidInput from &str
    pub fn invalid_input(message: impl Into<String>) -> Self {
        SubstraitExprError::InvalidInput(message.into())
    }

    /// Shortcut for creating InvalidSubstrait from &str
    pub fn invalid_substrait(message: impl Into<String>) -> Self {
        SubstraitExprError::InvalidSubstrait(message.into())
    }
}

pub(crate) type Result<T> = std::result::Result<T, SubstraitExprError>;
