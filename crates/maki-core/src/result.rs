//! Result type alias for FSH linting operations

use crate::error::MakiError;

/// Standard Result type for FSH linting operations
pub type Result<T> = std::result::Result<T, MakiError>;

/// Extension trait for Result to provide additional convenience methods
pub trait ResultExt<T> {
    /// Convert an error to a recoverable error if possible
    fn recoverable(self) -> Result<Option<T>>;

    /// Log the error and continue with None if recoverable
    fn log_and_continue(self) -> Option<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn recoverable(self) -> Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(err) if err.is_recoverable() => {
                tracing::warn!("Recoverable error: {}", err);
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }

    fn log_and_continue(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                if err.is_recoverable() {
                    tracing::warn!("Continuing after error: {}", err);
                    None
                } else {
                    tracing::error!("Fatal error: {}", err);
                    None
                }
            }
        }
    }
}
