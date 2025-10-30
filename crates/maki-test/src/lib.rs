//! MAKI Test Framework
//!
//! Testing framework for FHIR Shorthand files.
//! Provides functionality for:
//! - Running test suites
//! - Validating FSH instances
//! - Comparing expected vs actual output
//! - Integration testing for Implementation Guides

pub mod runner;

pub use runner::TestRunner;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
