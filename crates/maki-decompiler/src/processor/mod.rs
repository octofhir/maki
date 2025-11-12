//! Processors convert FHIR resources into Exportable objects
//!
//! This module provides processors for different FHIR resource types:
//! - StructureDefinition → Profile/Extension/Logical/Resource
//! - ValueSet → ExportableValueSet
//! - CodeSystem → ExportableCodeSystem
//! - Instances → ExportableInstance
//! - ImplementationGuide → ExportableConfiguration

pub mod code_system;
pub mod config;
pub mod instance;
pub mod processable;
pub mod structure_definition;
pub mod value_set;

// Re-exports
pub use code_system::*;
pub use config::*;
pub use instance::*;
pub use processable::*;
pub use structure_definition::*;
pub use value_set::*;
