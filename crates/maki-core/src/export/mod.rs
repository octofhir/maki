//! FHIR Export Module
//!
//! This module provides functionality for exporting FSH resources to FHIR JSON.
//! Implements SUSHI-compatible export capabilities.
//!
//! ## Overview
//!
//! The export module handles the transformation of FSH Abstract Syntax Trees
//! into valid FHIR resources (Profiles, Extensions, ValueSets, CodeSystems, etc.)
//!
//! ## Modules
//!
//! - `fhir_types` - FHIR type definitions (StructureDefinition, ElementDefinition, etc.)
//! - `profile_exporter` - Exports FSH Profiles to FHIR StructureDefinitions
//!
//! ## Status
//!
//! - ✅ Profile exporter (Task 17)
//! - ✅ Extension exporter (Task 18)
//! - 🔜 ValueSet exporter (Task 19)
//! - 🔜 CodeSystem exporter (Task 19)
//! - 🔜 Instance exporter (Task 20)
//! - 🔜 ImplementationGuide exporter (Task 28)

pub mod extension_exporter;
pub mod fhir_types;
pub mod profile_exporter;

pub use extension_exporter::ExtensionExporter;
pub use fhir_types::*;
pub use profile_exporter::{ExportError, ProfileExporter};

use crate::Result;

/// Instance exporter stub
///
/// This will be implemented in future tasks to export FSH instances to FHIR JSON.
pub struct InstanceExporter {
    // Implementation will be added
}

impl InstanceExporter {
    /// Create a new instance exporter
    pub fn new() -> Self {
        Self {}
    }

    /// Export an instance
    pub fn export(&self) -> Result<()> {
        // Will be implemented in Task 28
        Ok(())
    }
}

impl Default for InstanceExporter {
    fn default() -> Self {
        Self::new()
    }
}
