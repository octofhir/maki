//! Configuration types for maki
//!
//! This module contains the sub-configuration types used by UnifiedConfig.
//! The main configuration structure is UnifiedConfig in unified_config.rs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Linter configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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

/// Indent style for formatting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IndentStyle {
    /// Use spaces for indentation
    Spaces,
    /// Use tabs for indentation
    Tabs,
}

/// Formatter configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FormatterConfiguration {
    /// Enable/disable formatter
    #[schemars(description = "Enable or disable the formatter")]
    pub enabled: Option<bool>,

    /// Indent style (spaces or tabs)
    #[schemars(description = "Indentation style: 'spaces' or 'tabs'")]
    pub indent_style: Option<IndentStyle>,

    /// Indentation size in spaces (when indent_style is 'spaces')
    #[schemars(description = "Number of spaces for indentation")]
    pub indent_size: Option<usize>,

    /// Maximum line width
    #[schemars(description = "Maximum line width before wrapping")]
    pub line_width: Option<usize>,

    /// Whether to align caret expressions
    #[schemars(description = "Align caret expressions for readability")]
    pub align_carets: Option<bool>,

    /// Whether to add blank line before rules
    #[schemars(description = "Add blank line before rule definitions")]
    pub blank_line_before_rules: Option<bool>,

    /// Whether to preserve existing blank lines
    #[schemars(description = "Preserve blank lines from original source")]
    pub preserve_blank_lines: Option<bool>,

    /// Maximum consecutive blank lines to keep
    #[schemars(description = "Maximum number of consecutive blank lines")]
    pub max_blank_lines: Option<usize>,

    /// Group rules by type (metadata, constraints, flags)
    #[schemars(description = "Group rules by type for better organization")]
    pub group_rules: Option<bool>,

    /// Sort rules within groups
    #[schemars(description = "Sort rules alphabetically within groups")]
    pub sort_rules: Option<bool>,

    /// Blank lines between rule groups
    #[schemars(description = "Number of blank lines between rule groups")]
    pub blank_lines_between_groups: Option<usize>,

    /// Normalize spacing around operators (: and =)
    #[schemars(description = "Normalize spacing around operators")]
    pub normalize_spacing: Option<bool>,
}

/// Files configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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
            indent_style: Some(IndentStyle::Spaces),
            indent_size: Some(2),
            line_width: Some(100),
            align_carets: Some(true),
            blank_line_before_rules: Some(true),
            preserve_blank_lines: Some(true),
            max_blank_lines: Some(2),
            group_rules: Some(false),
            sort_rules: Some(false),
            blank_lines_between_groups: Some(1),
            normalize_spacing: Some(true),
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
    fn test_rule_severity_serialization() {
        let severity = RuleSeverity::Error;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, r#""error""#);

        let severity = RuleSeverity::Off;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, r#""off""#);
    }
}
