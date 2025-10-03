//! FSH Lint Rules
//!
//! Built-in rules and rule engine for FSH linter.
//! This crate provides the default rule set and rule management functionality.

pub mod builtin;
pub mod engine;
pub mod gritql;

// Re-export commonly used types
pub use engine::{
    DefaultRuleEngine, RuleDiscoveryConfig, RuleEngineStatistics, RulePack, RulePackDependency,
    RulePackMetadata, RulePrecedence, RuleRegistry,
};
pub use gritql::{CompiledGritQLPattern, GritQLCompiler, GritQLMatch, matches_to_diagnostics};

/// Initialize the built-in rules
pub fn init_builtin_rules() -> RuleRegistry {
    let registry = RuleRegistry::new();

    // TODO: Register built-in rules in later tasks
    tracing::info!("Initialized built-in rules registry");

    registry
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
