//! GritQL pattern registry - CST-based
//!
//! Manages compiled GritQL patterns for the rule engine.

use super::executor::CompiledGritQLPattern;
use std::collections::HashMap;

/// Registry for compiled GritQL patterns
#[derive(Debug, Clone, Default)]
pub struct GritQLRegistry {
    patterns: HashMap<String, CompiledGritQLPattern>,
}

impl GritQLRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
        }
    }

    /// Register a compiled pattern
    pub fn register(&mut self, rule_id: String, pattern: CompiledGritQLPattern) {
        self.patterns.insert(rule_id, pattern);
    }

    /// Get a pattern by rule ID
    pub fn get(&self, rule_id: &str) -> Option<&CompiledGritQLPattern> {
        self.patterns.get(rule_id)
    }

    /// Get all registered patterns
    pub fn all_patterns(&self) -> impl Iterator<Item = (&String, &CompiledGritQLPattern)> {
        self.patterns.iter()
    }

    /// Get the number of registered patterns
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gritql::GritQLCompiler;

    #[test]
    fn test_registry_creation() {
        let registry = GritQLRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_pattern() {
        let mut registry = GritQLRegistry::new();
        let compiler = GritQLCompiler::new().unwrap();
        let pattern = compiler
            .compile_pattern("Profile: $name", "test-rule")
            .unwrap();

        registry.register("test-rule".to_string(), pattern);

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.get("test-rule").is_some());
    }
}
