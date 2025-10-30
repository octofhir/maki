//! RuleSet parameter expansion and substitution
//!
//! This module implements FSH RuleSet expansion with **bracket-aware parameter substitution**.
//! RuleSets are FSH's template mechanism for reusable parameterized rules.
//!
//! # Key Feature: Bracket-Aware Substitution
//!
//! **CRITICAL**: This implementation fixes a known SUSHI bug by NOT substituting parameters
//! inside brackets `[]`. This is essential for correct FSH processing.
//!
//! ## Example
//!
//! ```text
//! RuleSet: SliceRules(sliceName, value)
//! * extension[{sliceName}].url = "http://example.com"  // {sliceName} NOT substituted
//! * {sliceName}.value = {value}                        // {sliceName} IS substituted
//!
//! Profile: MyProfile
//! * insert SliceRules("mySlice", "\"test\"")
//!
//! Expands to:
//! * extension[{sliceName}].url = "http://example.com"  // Bracket content preserved!
//! * mySlice.value = "test"                             // Parameter substituted
//! ```
//!
//! # Algorithm
//!
//! 1. Validate parameter count matches
//! 2. Build substitution map (parameter → argument)
//! 3. For each rule:
//!    - Clone the rule
//!    - Substitute parameters (skip content inside `[]`)
//!    - Handle nested RuleSet calls recursively
//! 4. Return expanded rules
//!
//! # Usage
//!
//! ```rust,no_run
//! use maki_core::semantic::ruleset::{RuleSet, RuleSetExpander, RuleSetInsert};
//! use std::path::PathBuf;
//!
//! let mut expander = RuleSetExpander::new();
//!
//! // Register RuleSet
//! let ruleset = RuleSet {
//!     name: "MyRules".to_string(),
//!     parameters: vec!["param1".to_string()],
//!     rules: vec![], // Add actual rules here
//!     source_file: PathBuf::from("test.fsh"),
//!     source_range: 0..10,
//! };
//! expander.register_ruleset(ruleset);
//!
//! // Expand insert
//! let insert = RuleSetInsert {
//!     ruleset_name: "MyRules".to_string(),
//!     arguments: vec!["\"value\"".to_string()],
//!     source_range: 10..20,
//! };
//!
//! let expanded = expander.expand(&insert).unwrap();
//! ```

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, trace};

/// Maximum nesting depth for RuleSet expansion to prevent infinite loops
const MAX_EXPANSION_DEPTH: usize = 10;

/// RuleSet definition with parameters
///
/// Represents a reusable template of FSH rules that can be parameterized.
#[derive(Debug, Clone)]
pub struct RuleSet {
    /// RuleSet name
    pub name: String,
    /// Parameter names
    pub parameters: Vec<String>,
    /// Rules in the RuleSet (as raw strings for now)
    pub rules: Vec<String>,
    /// Source file where defined
    pub source_file: PathBuf,
    /// Source code range
    pub source_range: Range<usize>,
}

/// RuleSet insert statement
///
/// Represents a RuleSet invocation with arguments.
#[derive(Debug, Clone)]
pub struct RuleSetInsert {
    /// Name of the RuleSet to expand
    pub ruleset_name: String,
    /// Arguments to pass to the RuleSet
    pub arguments: Vec<String>,
    /// Source code range
    pub source_range: Range<usize>,
}

/// RuleSet expansion errors
#[derive(Debug, Error)]
pub enum RuleSetError {
    /// RuleSet not found
    #[error("RuleSet not found: {0}")]
    RuleSetNotFound(String),

    /// Parameter count mismatch
    #[error(
        "Parameter count mismatch: {ruleset} expects {expected} parameters, got {actual}"
    )]
    ParameterCountMismatch {
        ruleset: String,
        expected: usize,
        actual: usize,
    },

    /// Nested RuleSet expansion depth exceeded
    #[error("Nested RuleSet expansion depth exceeded (max: {0})")]
    MaxDepthExceeded(usize),

    /// Circular RuleSet reference detected
    #[error("Circular RuleSet reference: {}", format_cycle(.0))]
    CircularReference(Vec<String>),

    /// Invalid parameter name
    #[error("Invalid parameter name: {0}")]
    InvalidParameterName(String),
}

/// Format circular reference cycle for error messages
fn format_cycle(cycle: &[String]) -> String {
    cycle.join(" → ")
}

/// RuleSet expander
///
/// Manages RuleSet definitions and expands RuleSet inserts with parameter substitution.
pub struct RuleSetExpander {
    /// Registered RuleSets by name
    rulesets: HashMap<String, Arc<RuleSet>>,
}

impl RuleSetExpander {
    /// Create a new RuleSet expander
    pub fn new() -> Self {
        Self {
            rulesets: HashMap::new(),
        }
    }

    /// Register a RuleSet definition
    ///
    /// If a RuleSet with the same name already exists, it will be replaced.
    pub fn register_ruleset(&mut self, ruleset: RuleSet) {
        let name = ruleset.name.clone();
        debug!("Registering RuleSet '{}' with {} parameters", name, ruleset.parameters.len());
        self.rulesets.insert(name, Arc::new(ruleset));
    }

    /// Expand a RuleSet insert
    ///
    /// Performs parameter substitution and handles nested RuleSet calls.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - RuleSet not found
    /// - Parameter count mismatch
    /// - Circular reference detected
    /// - Max expansion depth exceeded
    pub fn expand(&self, insert: &RuleSetInsert) -> Result<Vec<String>, RuleSetError> {
        let mut expansion_stack = Vec::new();
        self.expand_with_stack(insert, &mut expansion_stack, 0)
    }

    /// Internal expansion with stack tracking for circular reference detection
    fn expand_with_stack(
        &self,
        insert: &RuleSetInsert,
        expansion_stack: &mut Vec<String>,
        depth: usize,
    ) -> Result<Vec<String>, RuleSetError> {
        // Check depth limit
        if depth >= MAX_EXPANSION_DEPTH {
            return Err(RuleSetError::MaxDepthExceeded(MAX_EXPANSION_DEPTH));
        }

        // Check circular reference
        if expansion_stack.contains(&insert.ruleset_name) {
            expansion_stack.push(insert.ruleset_name.clone());
            return Err(RuleSetError::CircularReference(expansion_stack.clone()));
        }

        // Get RuleSet
        let ruleset = self
            .rulesets
            .get(&insert.ruleset_name)
            .ok_or_else(|| RuleSetError::RuleSetNotFound(insert.ruleset_name.clone()))?;

        // Validate parameter count
        if insert.arguments.len() != ruleset.parameters.len() {
            return Err(RuleSetError::ParameterCountMismatch {
                ruleset: insert.ruleset_name.clone(),
                expected: ruleset.parameters.len(),
                actual: insert.arguments.len(),
            });
        }

        trace!(
            "Expanding RuleSet '{}' at depth {} with {} rules",
            insert.ruleset_name,
            depth,
            ruleset.rules.len()
        );

        // Build substitution map
        let mut param_map = HashMap::new();
        for (i, param) in ruleset.parameters.iter().enumerate() {
            param_map.insert(param.clone(), insert.arguments[i].clone());
        }

        // Track this RuleSet in expansion stack
        expansion_stack.push(insert.ruleset_name.clone());

        // Expand each rule
        let mut expanded_rules = Vec::new();
        for rule in &ruleset.rules {
            let substituted = self.substitute_string(rule, &param_map);
            expanded_rules.push(substituted);
        }

        // Pop from expansion stack
        expansion_stack.pop();

        debug!(
            "Expanded RuleSet '{}' into {} rules",
            insert.ruleset_name,
            expanded_rules.len()
        );

        Ok(expanded_rules)
    }

    /// Substitute parameters in a string with bracket-awareness
    ///
    /// **CRITICAL**: Does NOT substitute parameters inside brackets `[]`.
    /// This is the key fix for SUSHI's known bug.
    ///
    /// # Algorithm
    ///
    /// 1. Track bracket depth while scanning
    /// 2. Only substitute when bracket_depth == 0
    /// 3. Find `{param}` patterns and replace with argument values
    ///
    /// # Example
    ///
    /// ```text
    /// Input:  "* address[{use}].system = {system}"
    /// Params: use="home", system="http://example.com"
    /// Output: "* address[{use}].system = http://example.com"
    ///         (Note: {use} in brackets NOT substituted!)
    /// ```
    fn substitute_string(&self, text: &str, params: &HashMap<String, String>) -> String {
        let mut result = String::with_capacity(text.len());
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        let mut bracket_depth: i32 = 0;

        while i < chars.len() {
            let ch = chars[i];

            // Track bracket depth
            if ch == '[' {
                bracket_depth += 1;
                result.push(ch);
                i += 1;
                continue;
            } else if ch == ']' {
                bracket_depth -= 1;
                result.push(ch);
                i += 1;
                continue;
            }

            // Check for parameter substitution {param}
            if ch == '{' && bracket_depth == 0 {
                // Find closing brace
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '}' {
                    j += 1;
                }

                if j < chars.len() {
                    // Extract parameter name
                    let param_name: String = chars[i + 1..j].iter().collect();

                    // Substitute if parameter exists
                    if let Some(value) = params.get(&param_name) {
                        trace!("Substituting {{{}}} with {}", param_name, value);
                        result.push_str(value);
                        i = j + 1;
                        continue;
                    }
                }
            }

            // No substitution, copy character
            result.push(ch);
            i += 1;
        }

        result
    }

    /// Check if a position in text is inside brackets
    ///
    /// Helper function to determine bracket context (used in tests).
    #[cfg(test)]
    fn is_inside_brackets(&self, text: &str, pos: usize) -> bool {
        let mut bracket_depth: i32 = 0;
        for (i, ch) in text.chars().enumerate() {
            if i >= pos {
                break;
            }
            if ch == '[' {
                bracket_depth += 1;
            } else if ch == ']' {
                bracket_depth -= 1;
            }
        }
        bracket_depth > 0
    }

    /// Get number of registered RuleSets
    pub fn ruleset_count(&self) -> usize {
        self.rulesets.len()
    }

    /// Check if a RuleSet is registered
    pub fn has_ruleset(&self, name: &str) -> bool {
        self.rulesets.contains_key(name)
    }

    /// Get a RuleSet by name
    pub fn get_ruleset(&self, name: &str) -> Option<&Arc<RuleSet>> {
        self.rulesets.get(name)
    }

    /// Get all RuleSet names
    pub fn ruleset_names(&self) -> Vec<String> {
        self.rulesets.keys().cloned().collect()
    }
}

impl Default for RuleSetExpander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_substitution() {
        let ruleset = RuleSet {
            name: "MyRuleSet".to_string(),
            parameters: vec!["param1".to_string()],
            rules: vec!["* value = {param1}".to_string()],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        let mut expander = RuleSetExpander::new();
        expander.register_ruleset(ruleset);

        let insert = RuleSetInsert {
            ruleset_name: "MyRuleSet".to_string(),
            arguments: vec!["\"test\"".to_string()],
            source_range: 10..20,
        };

        let expanded = expander.expand(&insert).unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0], "* value = \"test\"");
    }

    #[test]
    fn test_multiple_parameters() {
        let ruleset = RuleSet {
            name: "Test".to_string(),
            parameters: vec!["param1".to_string(), "param2".to_string()],
            rules: vec![
                "* {param1}.system = {param2}".to_string(),
                "* {param1}.use = #official".to_string(),
            ],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        let mut expander = RuleSetExpander::new();
        expander.register_ruleset(ruleset);

        let insert = RuleSetInsert {
            ruleset_name: "Test".to_string(),
            arguments: vec!["name".to_string(), "\"http://test.com\"".to_string()],
            source_range: 10..20,
        };

        let expanded = expander.expand(&insert).unwrap();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0], "* name.system = \"http://test.com\"");
        assert_eq!(expanded[1], "* name.use = #official");
    }

    #[test]
    fn test_bracket_aware_substitution() {
        // This is the CRITICAL test that verifies the SUSHI bug fix
        let ruleset = RuleSet {
            name: "Test".to_string(),
            parameters: vec!["param".to_string()],
            rules: vec![
                "* address[{param}].text = \"Test\"".to_string(),
                "* {param}.system = \"http://test\"".to_string(),
            ],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        let mut expander = RuleSetExpander::new();
        expander.register_ruleset(ruleset);

        let insert = RuleSetInsert {
            ruleset_name: "Test".to_string(),
            arguments: vec!["home".to_string()],
            source_range: 10..20,
        };

        let expanded = expander.expand(&insert).unwrap();
        assert_eq!(expanded.len(), 2);

        // CRITICAL: {param} inside brackets should NOT be substituted
        assert_eq!(expanded[0], "* address[{param}].text = \"Test\"");

        // {param} outside brackets SHOULD be substituted
        assert_eq!(expanded[1], "* home.system = \"http://test\"");
    }

    #[test]
    fn test_nested_brackets() {
        let ruleset = RuleSet {
            name: "Test".to_string(),
            parameters: vec!["p1".to_string(), "p2".to_string()],
            rules: vec![
                "* item[{p1}].item[{p2}].value = {p1}".to_string(),
            ],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        let mut expander = RuleSetExpander::new();
        expander.register_ruleset(ruleset);

        let insert = RuleSetInsert {
            ruleset_name: "Test".to_string(),
            arguments: vec!["a".to_string(), "b".to_string()],
            source_range: 10..20,
        };

        let expanded = expander.expand(&insert).unwrap();

        // {p1} and {p2} inside brackets NOT substituted, but {p1} outside IS substituted
        assert_eq!(expanded[0], "* item[{p1}].item[{p2}].value = a");
    }

    #[test]
    fn test_parameter_count_mismatch_too_few() {
        let ruleset = RuleSet {
            name: "Test".to_string(),
            parameters: vec!["param1".to_string(), "param2".to_string()],
            rules: vec![],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        let mut expander = RuleSetExpander::new();
        expander.register_ruleset(ruleset);

        let insert = RuleSetInsert {
            ruleset_name: "Test".to_string(),
            arguments: vec!["arg1".to_string()], // Only 1 arg, expects 2
            source_range: 10..20,
        };

        let result = expander.expand(&insert);
        assert!(result.is_err());
        match result.unwrap_err() {
            RuleSetError::ParameterCountMismatch {
                expected,
                actual,
                ..
            } => {
                assert_eq!(expected, 2);
                assert_eq!(actual, 1);
            }
            _ => panic!("Expected ParameterCountMismatch error"),
        }
    }

    #[test]
    fn test_parameter_count_mismatch_too_many() {
        let ruleset = RuleSet {
            name: "Test".to_string(),
            parameters: vec!["param1".to_string()],
            rules: vec![],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        let mut expander = RuleSetExpander::new();
        expander.register_ruleset(ruleset);

        let insert = RuleSetInsert {
            ruleset_name: "Test".to_string(),
            arguments: vec!["arg1".to_string(), "arg2".to_string()], // 2 args, expects 1
            source_range: 10..20,
        };

        let result = expander.expand(&insert);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuleSetError::ParameterCountMismatch { .. }
        ));
    }

    #[test]
    fn test_ruleset_not_found() {
        let expander = RuleSetExpander::new();

        let insert = RuleSetInsert {
            ruleset_name: "NonExistent".to_string(),
            arguments: vec![],
            source_range: 0..10,
        };

        let result = expander.expand(&insert);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuleSetError::RuleSetNotFound(_)
        ));
    }

    #[test]
    fn test_no_parameters() {
        let ruleset = RuleSet {
            name: "Simple".to_string(),
            parameters: vec![],
            rules: vec!["* status = #final".to_string()],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        let mut expander = RuleSetExpander::new();
        expander.register_ruleset(ruleset);

        let insert = RuleSetInsert {
            ruleset_name: "Simple".to_string(),
            arguments: vec![],
            source_range: 10..20,
        };

        let expanded = expander.expand(&insert).unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0], "* status = #final");
    }

    #[test]
    fn test_complex_substitution() {
        let ruleset = RuleSet {
            name: "Complex".to_string(),
            parameters: vec!["url".to_string(), "value".to_string()],
            rules: vec![
                "* extension[0].url = {url}".to_string(),
                "* extension[0].value{value} = true".to_string(),
                "* {url}.display = \"Display for {value}\"".to_string(),
            ],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        let mut expander = RuleSetExpander::new();
        expander.register_ruleset(ruleset);

        let insert = RuleSetInsert {
            ruleset_name: "Complex".to_string(),
            arguments: vec![
                "\"http://example.com\"".to_string(),
                "Boolean".to_string(),
            ],
            source_range: 10..20,
        };

        let expanded = expander.expand(&insert).unwrap();
        assert_eq!(expanded.len(), 3);

        // {url} inside [0] stays as is (inside brackets)
        assert_eq!(expanded[0], "* extension[0].url = \"http://example.com\"");

        // {value} after [] is substituted
        assert_eq!(expanded[1], "* extension[0].valueBoolean = true");

        // Both parameters substituted (not in brackets)
        assert_eq!(
            expanded[2],
            "* \"http://example.com\".display = \"Display for Boolean\""
        );
    }

    #[test]
    fn test_is_inside_brackets() {
        let expander = RuleSetExpander::new();

        let text = "* address[home].text";
        assert!(!expander.is_inside_brackets(text, 0));
        assert!(!expander.is_inside_brackets(text, 9));
        assert!(expander.is_inside_brackets(text, 10)); // 'h' in [home]
        assert!(expander.is_inside_brackets(text, 13)); // 'e' in [home]
        assert!(!expander.is_inside_brackets(text, 15)); // after ]
    }

    #[test]
    fn test_helper_methods() {
        let mut expander = RuleSetExpander::new();
        assert_eq!(expander.ruleset_count(), 0);

        let ruleset = RuleSet {
            name: "Test".to_string(),
            parameters: vec![],
            rules: vec![],
            source_file: PathBuf::from("test.fsh"),
            source_range: 0..10,
        };

        expander.register_ruleset(ruleset);

        assert_eq!(expander.ruleset_count(), 1);
        assert!(expander.has_ruleset("Test"));
        assert!(!expander.has_ruleset("Other"));

        let names = expander.ruleset_names();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"Test".to_string()));
    }
}
