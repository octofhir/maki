//! MAKI Language Server Protocol (LSP)
//!
//! Provides IDE support for FHIR Shorthand files including:
//! - Syntax highlighting
//! - Diagnostics (errors, warnings)
//! - Code completion
//! - Go-to-definition
//! - Hover information
//! - Code actions (quick fixes)
//! - Document formatting

pub mod server;

pub use server::MakiLanguageServer;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
