//! Rich diagnostic rendering
//!
//! This module provides diagnostic rendering capabilities:
//! - Terminal output with code frames
//! - Rich diff rendering for suggestions
//! - Console utilities with color support
//! - Multiple output formats (text, JSON)

pub mod diff;
pub mod renderer;
pub mod types;

// Re-export new renderers
pub use diff::DiffRenderer;
pub use renderer::{DiagnosticRenderer, OutputFormat};

// Re-export all types from types module for backward compatibility
pub use types::*;
