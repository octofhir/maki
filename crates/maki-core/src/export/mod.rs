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
//! - `build` - Build orchestrator for complete IG generation
//!
//! ## Status
//!
//! - ✅ Profile exporter (Task 17)
//! - ✅ Extension exporter (Task 18)
//! - ✅ Logical/Resource exporter (Task 19)
//! - ✅ Snapshot/Differential generator (Task 20)
//! - ✅ Instance exporter (Task 21)
//! - ✅ ValueSet exporter (Task 22)
//! - ✅ CodeSystem exporter (Task 23)
//! - ✅ Build command (Task 28)

pub mod build;
pub mod codesystem_exporter;
pub mod extension_exporter;
pub mod fhir_types;
pub mod file_structure;
pub mod ig_generator;
pub mod instance_exporter;
pub mod logical_exporter;
pub mod package_json;
pub mod predefined_resources;
pub mod profile_exporter;
pub mod snapshot;
pub mod valueset_exporter;

pub use build::{BuildError, BuildOptions, BuildOrchestrator, BuildResult, BuildStats};
pub use codesystem_exporter::CodeSystemExporter;
pub use extension_exporter::ExtensionExporter;
pub use fhir_types::*;
pub use file_structure::{
    FileStructureError, FileStructureGenerator, FshIndexEntry, format_fsh_index_table,
    FSH_GENERATED_DIR, RESOURCES_DIR, INCLUDES_DIR, DATA_DIR,
};
pub use ig_generator::{
    ImplementationGuide, ImplementationGuideGenerator, DependsOn, Definition,
    Grouping, ResourceEntry, Reference, Page,
};
pub use instance_exporter::InstanceExporter;
pub use logical_exporter::LogicalExporter;
pub use package_json::{PackageJson, Maintainer, Repository};
pub use predefined_resources::{
    PredefinedResource, PredefinedResourcesLoader, GeneratedResourceInfo,
    ConflictInfo, PredefinedResourceError, PREDEFINED_PACKAGE_NAME, PREDEFINED_PACKAGE_VERSION,
};
pub use profile_exporter::{ExportError, ProfileExporter};
pub use snapshot::{SnapshotError, SnapshotGenerator};
pub use valueset_exporter::ValueSetExporter;

use crate::Result;
