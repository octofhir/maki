//! # maki-decompiler
//!
//! FHIR to FSH decompiler for Maki
//!
//! This crate provides functionality to convert FHIR resources (JSON/XML) into
//! FHIR Shorthand (FSH) files, replicating the functionality of GoFSH.

pub mod canonical;
pub mod config_generator;
pub mod error;
pub mod exportable;
pub mod extractor;
pub mod lake;
pub mod loader;
pub mod models;
pub mod optimizer;
pub mod organizer;
pub mod processor;
pub mod stats;
pub mod writer;

// Re-exports for convenience
pub use canonical::{
    create_lake_with_session, parse_cli_dependencies, parse_fhir_release, parse_package_spec,
    setup_canonical_environment,
};
pub use config_generator::{ConfigGenerator, MakiConfig};
pub use error::{Error, Result};
// Exportable types - explicitly import to avoid conflicts with processor modules
pub use exportable::{
    AssignmentRule, BindingRule, CardinalityFlagRule, CardinalityRule, ContainsItem, ContainsRule,
    Exportable, ExportableCodeSystem, ExportableConfiguration, ExportableExtension,
    ExportableInstance, ExportableLogical, ExportableProfile, ExportableResource, ExportableRule,
    ExportableValueSet, FlagRule, FshCode, FshCodeableConcept, FshCoding, FshQuantity,
    FshReference, FshValue, IncludeRule, LocalCodeRule, escape_string, format_multiline_string,
};
pub use extractor::*;
pub use lake::{LakeStats, ResourceLake};
pub use loader::{FileLoader, LoadError, LoadStats};
pub use optimizer::{
    AddReferenceKeywordOptimizer, CombineAssignmentsOptimizer, CombineCardAndFlagRulesOptimizer,
    CombineContainsRulesOptimizer, OptimizationStats, Optimizer, OptimizerRegistry,
    RemoveChoiceSlicingRulesOptimizer, RemoveDuplicateRulesOptimizer,
    RemoveExtensionURLAssignmentOptimizer, RemoveGeneratedTextRulesOptimizer,
    RemoveImpliedCardinalityOptimizer, RemoveZeroZeroCardRulesOptimizer,
    SimplifyArrayIndexingOptimizer, SimplifyCardinalityOptimizer,
};
pub use organizer::{FileOrganizer, OrganizationStrategy};
// Processor types - explicitly import to avoid conflicts with exportable modules
pub use processor::{
    CodeSystemProcessor, ConfigProcessor, DefinitionType, InstanceProcessor,
    ProcessableElementDefinition, StructureDefinitionProcessor, ValueSetProcessor,
};
pub use stats::{GoFshSummary, ProcessingStats, WriteStats};
pub use writer::FshWriter;
