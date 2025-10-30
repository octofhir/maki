//! MAKI Formatter
//!
//! Provides formatting capabilities for FHIR Shorthand files.
//! This crate wraps the formatter functionality from maki-core
//! and provides a clean public API.

pub use maki_core::{
    AstFormatter, CaretAlignment, DiffChange, DiffChangeType, FormatDiff, FormatMode, FormatResult,
    Formatter, FormatterConfiguration, FormatterManager, Range,
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use super::{
        AstFormatter, CaretAlignment, FormatMode, FormatResult, Formatter, FormatterConfiguration,
        FormatterManager,
    };
}
