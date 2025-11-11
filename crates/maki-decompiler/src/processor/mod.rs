//! Processors convert FHIR resources into Exportable objects
//!
//! This module provides processors for different FHIR resource types:
//! - StructureDefinition → Profile/Extension/Logical/Resource
//! - ValueSet → ExportableValueSet
//! - CodeSystem → ExportableCodeSystem
//! - Instances → ExportableInstance
//! - ImplementationGuide → ExportableConfiguration

pub mod structure_definition;
pub mod processable;
pub mod value_set;
pub mod code_system;
pub mod instance;
pub mod config;

// Re-exports
pub use structure_definition::*;
pub use processable::*;
pub use value_set::*;
pub use code_system::*;
pub use instance::*;
pub use config::*;
