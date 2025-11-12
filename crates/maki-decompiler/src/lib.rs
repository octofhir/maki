//! # maki-decompiler
//!
//! FHIR to FSH decompiler for Maki
//!
//! This crate provides functionality to convert FHIR resources (JSON/XML) into
//! FHIR Shorthand (FSH) files, replicating the functionality of GoFSH.

pub mod error;
pub mod models;
pub mod lake;
pub mod loader;
pub mod canonical;
pub mod exportable;
pub mod processor;
pub mod extractor;
pub mod optimizer;
pub mod writer;
pub mod organizer;
pub mod config_generator;

// Re-exports for convenience
pub use error::{Error, Result};
pub use lake::{ResourceLake, LakeStats};
pub use loader::{FileLoader, LoadStats, LoadError};
pub use canonical::{
    setup_canonical_environment,
    create_lake_with_session,
    parse_fhir_release,
    parse_package_spec,
    parse_cli_dependencies,
};
pub use exportable::*;
pub use processor::*;
pub use extractor::*;
pub use optimizer::{Optimizer, OptimizerRegistry, OptimizationStats};
pub use writer::FshWriter;
pub use organizer::{FileOrganizer, OrganizationStrategy};
pub use config_generator::{ConfigGenerator, MakiConfig};
