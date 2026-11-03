//! FSH Lint Core
//!
//! Core linting engine for FHIR Shorthand (FSH) files.
//! This crate provides the fundamental components for parsing, analyzing,
//! and linting FSH files.

#[cfg(feature = "project")]
pub mod autofix;
pub mod cache;
#[cfg(feature = "project")]
pub mod canonical;
pub mod config;
pub mod console; // Terminal console utilities for rich output
pub mod cst; // Concrete Syntax Tree (lossless, Rowan-based)
pub mod diagnostics;
pub mod discovery;
pub mod error;
#[cfg(feature = "project")]
pub mod executor;
#[cfg(feature = "project")]
pub mod export; // FHIR exporters (profiles, instances, etc.)
pub mod formatter;
pub mod parser;
pub mod result;
#[cfg(feature = "project")]
pub mod rules;
#[cfg(feature = "project")]
pub mod semantic;

// Alias for FHIR utilities (canonical package management)
#[cfg(feature = "project")]
pub use canonical as fhir;

// Re-export commonly used types
#[cfg(feature = "project")]
pub use autofix::{
    AutofixEngine, AutofixEngineConfig, ConflictGroup, ConflictType, DefaultAutofixEngine, Fix,
    FixConfig, FixPreview, FixResult, RollbackPlan,
};
pub use cache::{
    Cache, CacheManager, CacheManagerStats, CacheStats, ContentHash, ParseResultCache,
};
#[cfg(feature = "project")]
pub use canonical::{
    CanonicalFacade, CanonicalLoaderError, CanonicalOptions, CanonicalResult, DefinitionResource,
    DefinitionSession, FhirRelease, LazySession, PackageCoordinate, create_default_maki_config,
};
// Re-export fishable types for convenience
#[cfg(feature = "project")]
pub use canonical::fishable::{FhirMetadata, FhirType, Fishable};
// Re-export version types for convenience
#[cfg(feature = "project")]
pub use canonical::version::{FhirVersionExt, VersionError, VersionResolver, VersionSpecifier};
// Configuration system
#[cfg(feature = "project")]
pub use config::{ConfigLoader, UnifiedConfig};
pub use config::{
    FilesConfiguration, FormatterConfiguration, LinterConfiguration, RuleSeverity,
    RulesConfiguration,
};
// Console utilities for rich terminal output
pub use console::{Color, Console};
pub use diagnostics::{
    Advices, Applicability, CodeSuggestion, DefaultDiagnosticCollector, DefaultOutputFormatter,
    Diagnostic, DiagnosticCategory, DiagnosticCollector, DiagnosticFormatter,
    DiagnosticOutputFormatter, DiagnosticRenderer, DiffRenderer, IgnoreErrors, Label, ListAdvice,
    Location, LogAdvice, LogCategory, OutputFormat, Severity, SourceMap, Visit,
};
#[cfg(feature = "project")]
pub use discovery::{DefaultFileDiscovery, FileDiscovery, FileWatcher};
pub use discovery::{FileChangeEvent, FileChangeKind};
pub use error::{ErrorKind, MakiError};
#[cfg(feature = "project")]
pub use executor::{
    DefaultExecutor, ExecutionContext, Executor, FileExecutionResult, ProgressCallback,
    ProgressInfo, ResourceStats,
};
#[cfg(feature = "project")]
pub use export::{InstanceExporter, ProfileExporter};
pub use formatter::{
    AstFormatter, CaretAlignment, DiffChange, DiffChangeType, FormatDiff, FormatMode, FormatResult,
    Formatter, FormatterManager, Range,
};
pub use parser::{
    CachedFshParser, FshParser, ParseError, ParseErrorKind, ParseResult, Parser, ParserConfig,
};
pub use result::Result;
#[cfg(feature = "project")]
pub use rules::{
    AutofixTemplate, CompiledRule, FixSafety, GritQLMatcher, Rule, RuleCategory, RuleConfig,
    RuleEngine, RuleEngineConfig, RuleMetadata,
};
#[cfg(feature = "project")]
pub use semantic::{
    Cardinality, CombinedSymbolTable, Constraint, ConstraintType, DefaultSemanticAnalyzer,
    DeferralReason, DeferredRule, DeferredRuleQueue, Element, ElementFlag, EnhancedSymbolTable,
    FhirResource, FishingContext, FshTank, Package, Reference, ReferenceType, ResourceMetadata,
    ResourceType, SemanticAnalyzer, SemanticAnalyzerConfig, SemanticModel, Symbol, SymbolError,
    SymbolTable, SymbolType, TypeInfo, UnresolvedRef,
};

/// Initialize the tracing subscriber for logging
#[cfg(feature = "project")]
pub fn init_tracing(default_filter: &str) {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    // Honor an explicit RUST_LOG if the user exported one; otherwise use the
    // caller-provided filter (e.g. derived from CLI verbosity). This avoids
    // mutating the process environment just to configure the subscriber.
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true),
        )
        .init();
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
