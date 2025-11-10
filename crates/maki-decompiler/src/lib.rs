//! # maki-decompiler
//!
//! FHIR to FSH decompiler for Maki
//!
//! This crate provides functionality to convert FHIR resources (JSON/XML) into
//! FHIR Shorthand (FSH) files, replicating the functionality of GoFSH.

pub mod error;
pub mod models;

// Re-exports for convenience
pub use error::{Error, Result};
