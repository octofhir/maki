//! Rule engine and GritQL integration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::semantic::SemanticModel;
use crate::{Diagnostic, FshLintError, Result, Severity};

/// A linting rule with GritQL pattern and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier for the rule
    pub id: String,
    /// Default severity level for diagnostics from this rule
    pub severity: Severity,
    /// Human-readable description of what the rule checks
    pub description: String,
    /// GritQL pattern for matching FSH constructs
    pub gritql_pattern: String,
    /// Optional autofix template for automatic corrections
    pub autofix: Option<AutofixTemplate>,
    /// Additional metadata for the rule
    pub metadata: RuleMetadata,
}

/// Compiled version of a rule ready for execution
#[derive(Debug, Clone)]
pub struct CompiledRule {
    /// Rule metadata including ID, name, description, etc.
    pub metadata: RuleMetadata,
    /// Compiled GritQL matcher for pattern matching
    pub matcher: GritQLMatcher,
    /// Optional autofix template for generating fixes
    pub autofix_template: Option<AutofixTemplate>,
}

/// Metadata associated with a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMetadata {
    /// Unique identifier for the rule
    pub id: String,
    /// Human-readable name for the rule
    pub name: String,
    /// Detailed description of what the rule checks
    pub description: String,
    /// Default severity level
    pub severity: Severity,
    /// Category this rule belongs to
    pub category: RuleCategory,
    /// Tags for organizing and filtering rules
    pub tags: Vec<String>,
    /// Version of the rule
    pub version: Option<String>,
    /// Documentation URL for the rule
    pub docs_url: Option<String>,
}

/// Categories for organizing rules
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleCategory {
    /// Correctness issues such as syntax and semantic violations
    Correctness,
    /// Suspicious patterns that often indicate bugs
    Suspicious,
    /// Excessive complexity that reduces readability or maintainability
    Complexity,
    /// Performance and optimization suggestions
    Performance,
    /// Style and formatting preferences
    Style,
    /// Experimental or incubating rules
    Nursery,
    /// Accessibility and inclusive design checks
    Accessibility,
    /// Documentation, metadata, and guidance improvements
    Documentation,
    /// Security-related checks
    Security,
    /// Custom category using a bespoke slug
    Custom(String),
}

impl RuleCategory {
    /// Return the kebab-case slug used for IDs and filtering
    pub fn slug(&self) -> &str {
        match self {
            RuleCategory::Correctness => "correctness",
            RuleCategory::Suspicious => "suspicious",
            RuleCategory::Complexity => "complexity",
            RuleCategory::Performance => "performance",
            RuleCategory::Style => "style",
            RuleCategory::Nursery => "nursery",
            RuleCategory::Accessibility => "accessibility",
            RuleCategory::Documentation => "documentation",
            RuleCategory::Security => "security",
            RuleCategory::Custom(name) => name.as_str(),
        }
    }

    /// Create a category from its slug, mapping unknown slugs to custom categories
    pub fn from_slug(slug: &str) -> Self {
        match slug {
            "correctness" | "syntax" | "semantic" => RuleCategory::Correctness,
            "suspicious" => RuleCategory::Suspicious,
            "complexity" => RuleCategory::Complexity,
            "performance" => RuleCategory::Performance,
            "style" => RuleCategory::Style,
            "nursery" => RuleCategory::Nursery,
            "accessibility" | "a11y" => RuleCategory::Accessibility,
            "documentation" | "best-practice" | "docs" => RuleCategory::Documentation,
            "security" => RuleCategory::Security,
            other => RuleCategory::Custom(other.to_string()),
        }
    }
}

impl serde::Serialize for RuleCategory {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.slug())
    }
}

impl<'de> serde::Deserialize<'de> for RuleCategory {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let slug = String::deserialize(deserializer)?;
        Ok(RuleCategory::from_slug(&slug))
    }
}

/// Template for generating automatic fixes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutofixTemplate {
    /// Description of what the fix does
    pub description: String,
    /// Template for generating the replacement text
    pub replacement_template: String,
    /// Safety level of the fix
    pub safety: FixSafety,
}

/// Safety classification for automatic fixes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FixSafety {
    /// Safe to apply automatically without user confirmation
    Safe,
    /// Requires user confirmation before applying
    Unsafe,
}

/// Compiled GritQL matcher
#[derive(Debug, Clone)]
pub struct GritQLMatcher {
    /// The original pattern
    pub pattern: String,
    /// Rule ID for error reporting
    pub rule_id: String,
}

/// Rule engine trait for loading and executing rules
pub trait RuleEngine {
    /// Load rules from the specified directories
    fn load_rules(&mut self, rule_dirs: &[PathBuf]) -> Result<()>;

    /// Compile a rule into an executable form
    fn compile_rule(&self, rule: &Rule) -> Result<CompiledRule>;

    /// Execute all loaded rules against a semantic model
    fn execute_rules(&self, model: &SemanticModel) -> Vec<Diagnostic>;

    /// Get all loaded rules
    fn get_rules(&self) -> &[CompiledRule];

    /// Get a specific rule by ID
    fn get_rule(&self, id: &str) -> Option<&CompiledRule>;

    /// Validate rule metadata and configuration
    fn validate_rule(&self, rule: &Rule) -> Result<()>;
}

/// Configuration for rule loading and execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEngineConfig {
    /// Directories to search for rule files
    pub rule_dirs: Vec<PathBuf>,
    /// Rule-specific configuration overrides
    pub rule_configs: HashMap<String, RuleConfig>,
    /// Whether to fail fast on rule compilation errors
    pub fail_fast: bool,
    /// Maximum number of diagnostics per rule
    pub max_diagnostics_per_rule: Option<usize>,
}

/// Configuration for a specific rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// Override the default severity
    pub severity: Option<Severity>,
    /// Rule-specific options
    pub options: HashMap<String, serde_json::Value>,
    /// Whether the rule is enabled
    pub enabled: bool,
}

impl Rule {
    /// Create a new rule with the given parameters
    pub fn new(
        id: String,
        name: String,
        description: String,
        severity: Severity,
        gritql_pattern: String,
    ) -> Self {
        let category = id
            .split('/')
            .filter(|segment| !segment.is_empty())
            .rev()
            .nth(1)
            .map(RuleCategory::from_slug)
            .unwrap_or(RuleCategory::Correctness);
        Self {
            id: id.clone(),
            severity,
            description: description.clone(),
            gritql_pattern,
            autofix: None,
            metadata: RuleMetadata {
                id,
                name,
                description,
                severity,
                category,
                tags: Vec::new(),
                version: None,
                docs_url: None,
            },
        }
    }

    /// Validate the rule configuration
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(FshLintError::RuleError {
                rule_id: self.id.clone(),
                message: "Rule ID cannot be empty".to_string(),
            });
        }

        if self.metadata.id != self.id {
            return Err(FshLintError::RuleError {
                rule_id: self.id.clone(),
                message: "Rule metadata ID must match rule ID".to_string(),
            });
        }

        if self.description.trim().is_empty() {
            return Err(FshLintError::RuleError {
                rule_id: self.id.clone(),
                message: "Rule description cannot be empty".to_string(),
            });
        }

        // NOTE: GritQL pattern can be empty for AST-based rules that use direct tree-sitter traversal
        // These rules implement their logic in custom check functions instead of using GritQL patterns

        let segments: Vec<&str> = self
            .id
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        if segments.len() < 2 {
            return Err(FshLintError::RuleError {
                rule_id: self.id.clone(),
                message: "Rule ID must follow '<namespace/>?<category>/<rule-name>' format"
                    .to_string(),
            });
        }

        let rule_slug = segments.last().unwrap();
        if !Self::is_valid_slug(rule_slug) {
            return Err(FshLintError::RuleError {
                rule_id: self.id.clone(),
                message: format!(
                    "Rule slug '{}' must be lower-case and use hyphenated segments",
                    rule_slug
                ),
            });
        }

        let category_slug = segments[segments.len() - 2];
        if !Self::is_valid_slug(category_slug) {
            return Err(FshLintError::RuleError {
                rule_id: self.id.clone(),
                message: format!(
                    "Category slug '{}' must be lower-case and use hyphenated segments",
                    category_slug
                ),
            });
        }

        let expected_slug = self.metadata.category.slug();
        if category_slug != expected_slug {
            return Err(FshLintError::RuleError {
                rule_id: self.id.clone(),
                message: format!(
                    "Rule ID category '{}' must match metadata category '{}'",
                    category_slug, expected_slug
                ),
            });
        }

        for namespace in &segments[..segments.len().saturating_sub(2)] {
            if !Self::is_valid_slug(namespace) {
                return Err(FshLintError::RuleError {
                    rule_id: self.id.clone(),
                    message: format!(
                        "Namespace segment '{}' must be lower-case and use hyphenated segments",
                        namespace
                    ),
                });
            }
        }

        if let RuleCategory::Custom(name) = &self.metadata.category {
            if name.trim().is_empty() {
                return Err(FshLintError::RuleError {
                    rule_id: self.id.clone(),
                    message: "Custom rule category slug cannot be empty".to_string(),
                });
            }
            if !Self::is_valid_slug(name) {
                return Err(FshLintError::RuleError {
                    rule_id: self.id.clone(),
                    message: format!(
                        "Custom category slug '{}' must be lower-case and use hyphenated segments",
                        name
                    ),
                });
            }
        }

        Ok(())
    }
}

impl Rule {
    fn is_valid_slug(value: &str) -> bool {
        !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }
}

impl CompiledRule {
    /// Create a new compiled rule
    pub fn new(metadata: RuleMetadata, matcher: GritQLMatcher) -> Self {
        Self {
            metadata,
            matcher,
            autofix_template: None,
        }
    }

    /// Get the rule ID
    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    /// Get the rule severity
    pub fn severity(&self) -> Severity {
        self.metadata.severity
    }

    /// Check if this rule has autofix capabilities
    pub fn has_autofix(&self) -> bool {
        self.autofix_template.is_some()
    }
}

impl GritQLMatcher {
    /// Create a new GritQL matcher from a pattern
    pub fn new(pattern: String) -> Result<Self> {
        Self::new_with_rule_id(pattern, "unknown")
    }

    /// Create a new GritQL matcher with a specific rule ID
    pub fn new_with_rule_id(pattern: String, rule_id: &str) -> Result<Self> {
        if pattern.is_empty() {
            return Err(FshLintError::RuleError {
                rule_id: rule_id.to_string(),
                message: "GritQL pattern cannot be empty".to_string(),
            });
        }

        Ok(Self {
            pattern,
            rule_id: rule_id.to_string(),
        })
    }

    /// Get the original pattern
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Get the rule ID
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }
}

impl Default for RuleEngineConfig {
    fn default() -> Self {
        Self {
            rule_dirs: Vec::new(),
            rule_configs: HashMap::new(),
            fail_fast: false,
            max_diagnostics_per_rule: Some(100),
        }
    }
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            severity: None,
            options: HashMap::new(),
            enabled: true,
        }
    }
}

impl std::fmt::Display for RuleCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.slug())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_creation() {
        let rule = Rule::new(
            "lint/correctness/test-rule".to_string(),
            "Test Rule".to_string(),
            "A test rule for validation".to_string(),
            Severity::Warning,
            "some_pattern".to_string(),
        );

        assert_eq!(rule.id, "lint/correctness/test-rule");
        assert_eq!(rule.metadata.name, "Test Rule");
        assert_eq!(rule.severity, Severity::Warning);
    }

    #[test]
    fn test_rule_validation() {
        let valid_rule = Rule::new(
            "lint/correctness/valid-rule".to_string(),
            "Valid Rule".to_string(),
            "A valid rule".to_string(),
            Severity::Error,
            "valid_pattern".to_string(),
        );

        assert!(valid_rule.validate().is_ok());

        let invalid_rule = Rule::new(
            "invalid".to_string(),
            "Invalid Rule".to_string(),
            "An invalid rule".to_string(),
            Severity::Error,
            "pattern".to_string(),
        );

        assert!(invalid_rule.validate().is_err());
    }

    #[test]
    fn test_gritql_matcher_creation() {
        let matcher = GritQLMatcher::new("test_pattern".to_string());
        assert!(matcher.is_ok());

        let empty_matcher = GritQLMatcher::new("".to_string());
        assert!(empty_matcher.is_err());
    }

    #[test]
    fn test_compiled_rule_creation() {
        let metadata = RuleMetadata {
            id: "lint/style/test-rule".to_string(),
            name: "Test Rule".to_string(),
            description: "A test rule".to_string(),
            severity: Severity::Warning,
            category: RuleCategory::Style,
            tags: vec!["test".to_string()],
            version: Some("1.0.0".to_string()),
            docs_url: None,
        };

        let matcher = GritQLMatcher::new("test_pattern".to_string()).unwrap();
        let compiled_rule = CompiledRule::new(metadata, matcher);

        assert_eq!(compiled_rule.id(), "lint/style/test-rule");
        assert_eq!(compiled_rule.severity(), Severity::Warning);
        assert!(!compiled_rule.has_autofix());
    }

    #[test]
    fn test_rule_category_display() {
        assert_eq!(RuleCategory::Correctness.to_string(), "correctness");
        assert_eq!(RuleCategory::Documentation.to_string(), "documentation");
        assert_eq!(
            RuleCategory::Custom("custom".to_string()).to_string(),
            "custom"
        );
    }
}
