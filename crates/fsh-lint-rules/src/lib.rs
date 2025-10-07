//! FSH Lint Rules
//!
//! Built-in rules and rule engine for FSH linter.
//! This crate provides the default rule set and rule management functionality.

pub mod builtin;
pub mod engine;
pub mod fhir_registry;
pub mod gritql;
pub mod gritql_ast;
pub mod pattern_parser;

// Re-export commonly used types
pub use builtin::BuiltinRules;
pub use engine::{
    DefaultRuleEngine, RuleDiscoveryConfig, RuleEngineStatistics, RulePack, RulePackDependency,
    RulePackMetadata, RulePrecedence, RuleRegistry, ThreadSafeRuleRegistry,
};
pub use gritql::{CompiledGritQLPattern, GritQLCompiler, GritQLMatch};
pub use gritql_ast::{
    AstMatch, AstPattern, NodeType, Predicate, execute_pattern,
    matches_to_diagnostics as ast_matches_to_diagnostics,
};
pub use pattern_parser::parse_pattern;

/// Initialize the built-in rules registry
///
/// Note: Rules are not pre-registered in the registry. They are loaded
/// dynamically by the rule engine when needed. This function creates
/// an empty registry that will be populated by the DefaultRuleEngine
/// as rules are loaded via load_rule().
pub fn init_builtin_rules() -> RuleRegistry {
    let registry = RuleRegistry::new();
    tracing::debug!("Initialized built-in rules registry");
    registry
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
