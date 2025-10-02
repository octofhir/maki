//! FSH Lint Rules
//! 
//! Built-in rules and rule engine for FSH linter.
//! This crate provides the default rule set and rule management functionality.

pub mod builtin;
pub mod engine;

// Re-export commonly used types
pub use engine::{RuleEngine, RuleRegistry};

/// Initialize the built-in rules
pub fn init_builtin_rules() -> RuleRegistry {
    let mut registry = RuleRegistry::new();
    
    // TODO: Register built-in rules in later tasks
    tracing::info!("Initialized built-in rules registry");
    
    registry
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");