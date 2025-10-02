//! Rule engine implementation

use std::collections::HashMap;

/// Registry for managing rules
pub struct RuleRegistry {
    rules: HashMap<String, Box<dyn Rule>>,
}

/// Trait for implementing linting rules
pub trait Rule: Send + Sync {
    /// Get the rule identifier
    fn id(&self) -> &str;
    
    /// Get the rule description
    fn description(&self) -> &str;
    
    /// Get the default severity for this rule
    fn default_severity(&self) -> fsh_lint_core::diagnostics::Severity;
}

/// Rule engine for executing rules
pub struct RuleEngine {
    registry: RuleRegistry,
}

impl RuleRegistry {
    /// Create a new rule registry
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }
    
    /// Register a new rule
    pub fn register(&mut self, rule: Box<dyn Rule>) {
        let id = rule.id().to_string();
        self.rules.insert(id, rule);
    }
    
    /// Get a rule by ID
    pub fn get(&self, id: &str) -> Option<&dyn Rule> {
        self.rules.get(id).map(|r| r.as_ref())
    }
    
    /// List all registered rule IDs
    pub fn list_ids(&self) -> Vec<&str> {
        self.rules.keys().map(|s| s.as_str()).collect()
    }
}

impl RuleEngine {
    /// Create a new rule engine with the given registry
    pub fn new(registry: RuleRegistry) -> Self {
        Self { registry }
    }
    
    /// Get the rule registry
    pub fn registry(&self) -> &RuleRegistry {
        &self.registry
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}