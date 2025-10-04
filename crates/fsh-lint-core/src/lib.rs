//! FSH Lint Core
//!
//! Core linting engine for FHIR Shorthand (FSH) files.
//! This crate provides the fundamental components for parsing, analyzing,
//! and linting FSH files.

pub mod ast;
pub mod autofix;
pub mod cache;
pub mod config;
pub mod diagnostics;
pub mod discovery;
pub mod error;
pub mod executor;
pub mod formatter;
pub mod lexer;
pub mod parser;
mod parser_chumsky;
pub mod result;
pub mod rules;
pub mod semantic;

// Re-export commonly used types
pub use autofix::{
    AutofixEngine, AutofixEngineConfig, ConflictGroup, ConflictType, DefaultAutofixEngine, Fix,
    FixConfig, FixPreview, FixResult, RollbackPlan,
};
pub use cache::{
    Cache, CacheManager, CacheManagerStats, CacheStats, ContentHash, ParseResultCache,
};
pub use config::Config;
pub use diagnostics::{
    Advices, Applicability, CodeSuggestion, DefaultDiagnosticCollector, DefaultOutputFormatter,
    Diagnostic, DiagnosticCategory, DiagnosticCollector, DiagnosticFormatter,
    DiagnosticOutputFormatter, Label, ListAdvice, Location, LogAdvice, LogCategory, Severity,
    SourceMap, Suggestion, Visit,
};
pub use discovery::{
    DefaultFileDiscovery, FileChangeEvent, FileChangeKind, FileDiscovery, FileWatcher,
};
pub use error::{ErrorKind, FshLintError};
pub use executor::{
    DefaultExecutor, ExecutionContext, Executor, FileExecutionResult, ProgressCallback,
    ProgressInfo, ResourceStats,
};
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
    Cardinality, Constraint, ConstraintType, DefaultSemanticAnalyzer, Element, ElementFlag,
    FhirResource, Reference, ReferenceType, ResourceMetadata, ResourceType, SemanticAnalyzer,
    SemanticAnalyzerConfig, SemanticModel, Symbol, SymbolTable, SymbolType, TypeInfo,
};

/// Initialize the tracing subscriber for logging
pub fn init_tracing() {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("fsh_lint=info"));

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
