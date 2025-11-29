//! FSH Lint Core
//!
//! Core linting engine for FHIR Shorthand (FSH) files.
//! This crate provides the fundamental components for parsing, analyzing,
//! and linting FSH files.

pub mod autofix;
pub mod cache;
pub mod canonical;
pub mod config;
pub mod console; // Terminal console utilities for rich output
pub mod cst; // Concrete Syntax Tree (lossless, Rowan-based)
pub mod diagnostics;
pub mod discovery;
pub mod error;
pub mod executor;
pub mod export; // FHIR exporters (profiles, instances, etc.)
pub mod formatter;
pub mod parser;
pub mod result;
pub mod rules;
pub mod semantic;

// Alias for FHIR utilities (canonical package management)
pub use canonical as fhir;

// Re-export commonly used types
pub use autofix::{
    AutofixEngine, AutofixEngineConfig, ConflictGroup, ConflictType, DefaultAutofixEngine, Fix,
    FixConfig, FixPreview, FixResult, RollbackPlan,
};
pub use cache::{
    Cache, CacheManager, CacheManagerStats, CacheStats, ContentHash, ParseResultCache,
};
pub use canonical::{
    CanonicalFacade, CanonicalLoaderError, CanonicalOptions, CanonicalResult, DefinitionResource,
    DefinitionSession, FhirRelease, LazySession, PackageCoordinate, create_default_maki_config,
};
// Re-export fishable types for convenience
pub use canonical::fishable::{FhirMetadata, FhirType, Fishable};
// Re-export version types for convenience
pub use canonical::version::{FhirVersionExt, VersionError, VersionResolver, VersionSpecifier};
// Configuration system
pub use config::{
    ConfigLoader, FilesConfiguration, FormatterConfiguration, LinterConfiguration, RuleSeverity,
    RulesConfiguration, UnifiedConfig,
};
// Console utilities for rich terminal output
pub use console::{Color, Console};
pub use diagnostics::{
    Advices, Applicability, CodeSuggestion, DefaultDiagnosticCollector, DefaultOutputFormatter,
    Diagnostic, DiagnosticCategory, DiagnosticCollector, DiagnosticFormatter,
    DiagnosticOutputFormatter, DiagnosticRenderer, DiffRenderer, Label, ListAdvice, Location,
    LogAdvice, LogCategory, OutputFormat, Severity, SourceMap, Visit,
};
pub use discovery::{
    DefaultFileDiscovery, FileChangeEvent, FileChangeKind, FileDiscovery, FileWatcher,
};
pub use error::{ErrorKind, MakiError};
pub use executor::{
    DefaultExecutor, ExecutionContext, Executor, FileExecutionResult, ProgressCallback,
    ProgressInfo, ResourceStats,
};
pub use export::{InstanceExporter, ProfileExporter};
pub use formatter::{
    AstFormatter, CaretAlignment, DiffChange, DiffChangeType, FormatDiff, FormatMode, FormatResult,
    Formatter, FormatterManager, Range,
};
pub use parser::{
    CachedFshParser, FshParser, ParseError, ParseErrorKind, ParseResult, Parser, ParserConfig,
};
pub use result::Result;
pub use rules::{
    AutofixTemplate, CompiledRule, FixSafety, GritQLMatcher, Rule, RuleCategory, RuleConfig,
    RuleEngine, RuleEngineConfig, RuleMetadata,
};
pub use semantic::{
    Cardinality, CombinedSymbolTable, Constraint, ConstraintType, DefaultSemanticAnalyzer,
    DeferralReason, DeferredRule, DeferredRuleQueue, Element, ElementFlag, EnhancedSymbolTable,
    FhirResource, FishingContext, FshTank, Package, Reference, ReferenceType, ResourceMetadata,
    ResourceType, SemanticAnalyzer, SemanticAnalyzerConfig, SemanticModel, Symbol, SymbolError,
    SymbolTable, SymbolType, TypeInfo, UnresolvedRef,
};

/// Initialize the tracing subscriber for logging
pub fn init_tracing() {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("maki=info"));

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
