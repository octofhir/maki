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
pub mod build_cache;
pub mod codesystem_exporter;
pub mod differential_generator;
pub mod extension_exporter;
pub mod fhir_types;
pub mod file_structure;
pub mod ig_generator;
pub mod instance_exporter;
pub mod invariant_processor;
pub mod logical_exporter;
pub mod mapping_exporter;
pub mod menu_generator;
pub mod package_json;
pub mod predefined_resources;
pub mod profile_exporter;
pub mod ruleset_integration;
pub mod snapshot;
pub mod valueset_exporter;

pub use build::{BuildError, BuildOptions, BuildOrchestrator, BuildResult, BuildStats};
pub use build_cache::{BuildCache, CacheStats, IncrementalBuildInfo};
pub use codesystem_exporter::CodeSystemExporter;
pub use differential_generator::{
    DifferentialError, DifferentialGenerator, RuleContext, RuleProcessor,
};
pub use extension_exporter::ExtensionExporter;
pub use fhir_types::*;
pub use file_structure::{
    DATA_DIR, FSH_GENERATED_DIR, FileStructureError, FileStructureGenerator, FshIndexEntry,
    INCLUDES_DIR, RESOURCES_DIR, format_fsh_index_table,
};
pub use ig_generator::{
    Definition, DependsOn, Grouping, ImplementationGuide, ImplementationGuideGenerator, Page,
    Reference, ResourceEntry,
};
pub use instance_exporter::InstanceExporter;
pub use invariant_processor::InvariantProcessor;
pub use logical_exporter::LogicalExporter;
pub use mapping_exporter::MappingExporter;
pub use menu_generator::MenuGenerator;
pub use package_json::{Maintainer, PackageJson, Repository};
pub use predefined_resources::{
    ConflictInfo, GeneratedResourceInfo, PREDEFINED_PACKAGE_NAME, PREDEFINED_PACKAGE_VERSION,
    PredefinedResource, PredefinedResourceError, PredefinedResourcesLoader,
};
pub use profile_exporter::{ExportError, ProfileExporter};
pub use snapshot::{SnapshotError, SnapshotGenerator};
pub use valueset_exporter::ValueSetExporter;

/// Execute a blocking filesystem operation without starving Tokio's scheduler.
///
/// Uses `tokio::task::block_in_place` when running inside a multi-threaded
/// Tokio runtime. Otherwise, runs the operation directly.
pub(crate) fn run_blocking_io<F, R>(operation: F) -> R
where
    F: FnOnce() -> R,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current()
        && handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread
    {
        tokio::task::block_in_place(operation)
    } else {
        operation()
    }
}
