//! FSH Lint Core
//! 
//! Core linting engine for FHIR Shorthand (FSH) files.
//! This crate provides the fundamental components for parsing, analyzing,
//! and linting FSH files.

pub mod error;
pub mod result;
pub mod config;
pub mod parser;
pub mod semantic;
pub mod rules;
pub mod diagnostics;
pub mod cache;
pub mod discovery;

// Re-export commonly used types
pub use error::{FshLintError, ErrorKind};
pub use result::Result;
pub use config::Config;
pub use diagnostics::{Diagnostic, Severity, Location};
pub use discovery::{FileDiscovery, DefaultFileDiscovery, FileWatcher, FileChangeEvent, FileChangeKind};
pub use parser::{Parser, ParseResult, ParseError, FshParser, CachedFshParser, ParserConfig};
pub use cache::{Cache, ContentHash, ParseResultCache, CacheStats, CacheManager, CacheManagerStats};
pub use semantic::{
    SemanticModel, SemanticAnalyzer, DefaultSemanticAnalyzer, SemanticAnalyzerConfig,
    FhirResource, ResourceType, Element, Cardinality, TypeInfo, Constraint, ConstraintType,
    ElementFlag, ResourceMetadata, SymbolTable, Symbol, SymbolType, Reference, ReferenceType
};

/// Initialize the tracing subscriber for logging
pub fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("fsh_lint=info"));
    
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true)
        )
        .init();
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");