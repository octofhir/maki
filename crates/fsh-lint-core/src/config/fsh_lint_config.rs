//! Configuration types for fsh-lint-rs

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main fsh-lint configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FshLintConfiguration {
    /// JSON Schema reference for IDE support
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub schema: Option<String>,

    /// Mark this directory as the root (stop upward search)
    #[schemars(description = "Stop config file discovery at this directory")]
    pub root: Option<bool>,

    /// Extend from other configuration files
    #[schemars(description = "Inherit from other config files (relative or absolute paths)")]
    pub extends: Option<Vec<String>>,

    /// Linter configuration
    #[schemars(description = "Linter settings and rules")]
    pub linter: Option<LinterConfiguration>,

    /// Formatter configuration
    #[schemars(description = "Code formatter settings")]
    pub formatter: Option<FormatterConfiguration>,

    /// File pattern configuration
    #[schemars(description = "File inclusion/exclusion patterns")]
    pub files: Option<FilesConfiguration>,
}

/// Linter configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinterConfiguration {
    /// Enable/disable linter
    #[schemars(description = "Enable or disable the linter")]
    pub enabled: Option<bool>,

    /// Rule configuration
    #[schemars(description = "Rule severity configuration")]
    pub rules: Option<RulesConfiguration>,

    /// Directories containing custom GritQL rules
    #[schemars(description = "Paths to directories containing .grit rule files")]
    pub rule_directories: Option<Vec<String>>,
}

/// Rules configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RulesConfiguration {
    /// Enable all recommended rules
    #[schemars(description = "Enable all recommended rules")]
    pub recommended: Option<bool>,

    /// Enable all available rules
    #[schemars(description = "Enable all rules")]
    pub all: Option<bool>,

    /// Blocking rules (critical requirements)
    #[schemars(description = "Blocking rules configuration")]
    pub blocking: Option<HashMap<String, RuleSeverity>>,

    /// Correctness rules (errors in FSH logic)
    #[schemars(description = "Correctness rules configuration")]
    pub correctness: Option<HashMap<String, RuleSeverity>>,

    /// Suspicious rules (patterns that often indicate bugs)
    #[schemars(description = "Suspicious rules configuration")]
    pub suspicious: Option<HashMap<String, RuleSeverity>>,

    /// Style rules (formatting and conventions)
    #[schemars(description = "Style rules configuration")]
    pub style: Option<HashMap<String, RuleSeverity>>,

    /// Documentation rules
    #[schemars(description = "Documentation rules configuration")]
    pub documentation: Option<HashMap<String, RuleSeverity>>,
}

/// Rule severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    /// Disable the rule
    Off,
    /// Informational message
    Info,
    /// Warning (doesn't fail build)
    Warn,
    /// Error (fails build)
    Error,
}

/// Formatter configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FormatterConfiguration {
    /// Enable/disable formatter
    #[schemars(description = "Enable or disable the formatter")]
    pub enabled: Option<bool>,

    /// Indentation size in spaces
    #[schemars(description = "Number of spaces for indentation")]
    pub indent_size: Option<usize>,

    /// Maximum line width
    #[schemars(description = "Maximum line width before wrapping")]
    pub line_width: Option<usize>,

    /// Whether to align caret expressions
    #[schemars(description = "Align caret expressions for readability")]
    pub align_carets: Option<bool>,
}

/// Files configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FilesConfiguration {
    /// Glob patterns to include
    #[schemars(description = "Glob patterns for files to include")]
    pub include: Option<Vec<String>>,

    /// Glob patterns to exclude
    #[schemars(description = "Glob patterns for files to exclude")]
    pub exclude: Option<Vec<String>>,

    /// Custom ignore files to respect
    #[schemars(description = "Additional ignore files to respect (beyond .gitignore)")]
    pub ignore_files: Option<Vec<String>>,
}

/// Rule-specific configuration with options
///
/// This type is used for individual rule configuration, allowing
/// fine-grained control over rule behavior and options.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuleConfig {
    /// Rule severity override
    #[schemars(description = "Override the default severity for this rule")]
    pub severity: Option<RuleSeverity>,

    /// Rule-specific options
    #[schemars(description = "Custom options for this rule")]
    pub options: Option<serde_json::Value>,
}

impl Default for FshLintConfiguration {
    fn default() -> Self {
        Self {
            schema: Some("https://octofhir.github.io/fsh-lint-rs/schema/v1.json".to_string()),
            root: Some(false),
            extends: None,
            linter: Some(LinterConfiguration::default()),
            formatter: Some(FormatterConfiguration::default()),
            files: Some(FilesConfiguration::default()),
        }
    }
}

impl Default for LinterConfiguration {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            rules: Some(RulesConfiguration::default()),
            rule_directories: None,
        }
    }
}

impl Default for RulesConfiguration {
    fn default() -> Self {
        Self {
            recommended: Some(true),
            all: None,
            blocking: None,
            correctness: None,
            suspicious: None,
            style: None,
            documentation: None,
        }
    }
}

impl Default for FormatterConfiguration {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            indent_size: Some(2),
            line_width: Some(100),
            align_carets: Some(true),
        }
    }
}

impl Default for FilesConfiguration {
    fn default() -> Self {
        Self {
            include: Some(vec!["**/*.fsh".to_string()]),
            exclude: Some(vec![
                "**/node_modules/**".to_string(),
                "**/temp/**".to_string(),
                "**/*.generated.fsh".to_string(),
                "**/target/**".to_string(),
                "**/build/**".to_string(),
            ]),
            ignore_files: Some(vec![".fshlintignore".to_string()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FshLintConfiguration::default();
        assert!(config.linter.is_some());
        assert!(config.formatter.is_some());
        assert!(config.files.is_some());
    }

    #[test]
    fn test_rule_severity_serialization() {
        let severity = RuleSeverity::Error;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, r#""error""#);

        let severity = RuleSeverity::Off;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, r#""off""#);
    }

    #[test]
    fn test_config_serialization() {
        let config = FshLintConfiguration::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("linter"));
        assert!(json.contains("formatter"));
        assert!(json.contains("files"));
    }

    #[test]
    fn test_config_deserialization() {
        let json = r#"{
            "linter": {
                "enabled": true,
                "rules": {
                    "recommended": true
                }
            }
        }"#;

        let config: FshLintConfiguration = serde_json::from_str(json).unwrap();
        assert!(config.linter.is_some());
        assert_eq!(config.linter.unwrap().enabled, Some(true));
    }
}
