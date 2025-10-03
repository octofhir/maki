//! Configuration management for FSH linting

use crate::error::FshLintError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Configuration files to extend from
    pub extends: Option<Vec<String>>,
    /// File patterns to include
    #[serde(alias = "include")]
    pub include_patterns: Vec<String>,
    /// File patterns to exclude  
    #[serde(alias = "exclude")]
    pub exclude_patterns: Vec<String>,
    /// Custom ignore files to respect (in addition to .gitignore)
    pub ignore_files: Vec<String>,
    /// Directories containing custom rules
    pub rules_dir: Vec<PathBuf>,
    /// Rule-specific configuration
    pub rules: HashMap<String, RuleConfig>,
    /// Directory-specific overrides
    pub overrides: Vec<Override>,
    /// Environment-specific settings
    pub env: Environment,
    /// Formatter configuration
    pub formatter: FormatterConfig,
    /// Autofix configuration
    pub autofix: AutofixConfig,
}

/// Rule-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuleConfig {
    /// Rule severity override
    pub severity: Option<crate::diagnostics::Severity>,
    /// Rule-specific options
    pub options: HashMap<String, serde_json::Value>,
}

/// Directory-specific configuration overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Override {
    /// Glob pattern for files to apply override to
    pub files: String,
    /// Configuration to apply
    pub config: Config,
}

/// Environment-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Environment {
    /// Target FHIR version
    pub fhir_version: Option<String>,
    /// Additional context paths
    pub context_paths: Vec<PathBuf>,
}

/// Formatter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FormatterConfig {
    /// Indentation size in spaces
    pub indent_size: usize,
    /// Maximum line width
    pub max_line_width: usize,
    /// Whether to align caret expressions
    pub align_carets: bool,
}

/// Autofix configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutofixConfig {
    /// Whether to enable safe autofixes
    pub enable_safe: bool,
    /// Whether to enable unsafe autofixes (requires explicit confirmation)
    pub enable_unsafe: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            extends: None,
            include_patterns: vec!["**/*.fsh".to_string()],
            exclude_patterns: vec!["node_modules/**".to_string(), "target/**".to_string()],
            ignore_files: vec![".fshlintignore".to_string()],
            rules_dir: Vec::new(),
            rules: HashMap::new(),
            overrides: Vec::new(),
            env: Environment::default(),
            formatter: FormatterConfig::default(),
            autofix: AutofixConfig::default(),
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            fhir_version: Some("4.0.1".to_string()),
            context_paths: Vec::new(),
        }
    }
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_size: 2,
            max_line_width: 100,
            align_carets: true,
        }
    }
}

impl Default for AutofixConfig {
    fn default() -> Self {
        Self {
            enable_safe: true,
            enable_unsafe: false,
        }
    }
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            severity: None,
            options: HashMap::new(),
        }
    }
}

/// Configuration manager trait for loading and merging configurations
pub trait ConfigManager {
    /// Load configuration from a specific path or discover it
    fn load_config(&self, path: Option<&Path>) -> Result<Config, FshLintError>;

    /// Validate a configuration
    fn validate_config(&self, config: &Config) -> Result<(), FshLintError>;

    /// Merge two configurations, with override taking precedence
    fn merge_configs(&self, base: Config, override_config: Config) -> Config;

    /// Discover configuration file by walking up directory tree
    fn discover_config(&self, start_path: &Path) -> Option<PathBuf>;
}

/// Default implementation of ConfigManager
#[derive(Debug, Default)]
pub struct DefaultConfigManager;

impl ConfigManager for DefaultConfigManager {
    fn load_config(&self, path: Option<&Path>) -> Result<Config, FshLintError> {
        let config_path = match path {
            Some(p) => p.to_path_buf(),
            None => {
                // Try to discover config file
                let current_dir =
                    std::env::current_dir().map_err(|e| FshLintError::ConfigError {
                        message: format!("Failed to get current directory: {}", e),
                    })?;

                match self.discover_config(&current_dir) {
                    Some(path) => path,
                    None => return Ok(Config::default()),
                }
            }
        };

        self.load_config_from_path(&config_path)
    }

    fn validate_config(&self, config: &Config) -> Result<(), FshLintError> {
        config.validate()
    }

    fn merge_configs(&self, mut base: Config, override_config: Config) -> Config {
        // Merge extends
        if let Some(extends) = override_config.extends {
            base.extends = Some(extends);
        }

        // Merge include patterns (override replaces base if different from default)
        if override_config.include_patterns != Config::default().include_patterns {
            base.include_patterns = override_config.include_patterns;
        }

        // Merge exclude patterns (override replaces base if different from default)
        if override_config.exclude_patterns != Config::default().exclude_patterns {
            base.exclude_patterns = override_config.exclude_patterns;
        }

        // Merge ignore files
        base.ignore_files.extend(override_config.ignore_files);

        // Merge rules directories
        base.rules_dir.extend(override_config.rules_dir);

        // Merge rules (override takes precedence)
        for (rule_id, rule_config) in override_config.rules {
            base.rules.insert(rule_id, rule_config);
        }

        // Merge overrides
        base.overrides.extend(override_config.overrides);

        // Merge environment settings
        if override_config.env.fhir_version.is_some() {
            base.env.fhir_version = override_config.env.fhir_version;
        }
        base.env
            .context_paths
            .extend(override_config.env.context_paths);

        // Merge formatter config (override takes precedence for each field)
        if override_config.formatter.indent_size != FormatterConfig::default().indent_size {
            base.formatter.indent_size = override_config.formatter.indent_size;
        }
        if override_config.formatter.max_line_width != FormatterConfig::default().max_line_width {
            base.formatter.max_line_width = override_config.formatter.max_line_width;
        }
        if override_config.formatter.align_carets != FormatterConfig::default().align_carets {
            base.formatter.align_carets = override_config.formatter.align_carets;
        }

        // Merge autofix config
        if override_config.autofix.enable_safe != AutofixConfig::default().enable_safe {
            base.autofix.enable_safe = override_config.autofix.enable_safe;
        }
        if override_config.autofix.enable_unsafe != AutofixConfig::default().enable_unsafe {
            base.autofix.enable_unsafe = override_config.autofix.enable_unsafe;
        }

        base
    }

    fn discover_config(&self, start_path: &Path) -> Option<PathBuf> {
        let config_names = [".fshlintrc", ".fshlintrc.json", ".fshlintrc.toml"];

        let mut current_path = start_path;

        loop {
            for config_name in &config_names {
                let config_path = current_path.join(config_name);
                if config_path.exists() {
                    return Some(config_path);
                }
            }

            // Move up one directory
            match current_path.parent() {
                Some(parent) => current_path = parent,
                None => break,
            }
        }

        None
    }
}

impl DefaultConfigManager {
    /// Create a new default config manager
    pub fn new() -> Self {
        Self
    }

    /// Load configuration from a specific file path
    fn load_config_from_path(&self, path: &Path) -> Result<Config, FshLintError> {
        let content = fs::read_to_string(path).map_err(|e| FshLintError::ConfigError {
            message: format!("Failed to read config file '{}': {}", path.display(), e),
        })?;

        let mut config = self.parse_config_content(&content, path)?;

        // Handle extends
        if let Some(extends) = &config.extends.clone() {
            config = self.resolve_extends(config, extends, path)?;
        }

        config.validate()?;
        Ok(config)
    }

    /// Parse configuration content based on file extension
    fn parse_config_content(&self, content: &str, path: &Path) -> Result<Config, FshLintError> {
        let extension = path.extension().and_then(|ext| ext.to_str());

        match extension {
            Some("toml") => Config::from_toml(content),
            Some("json") => Config::from_json(content),
            _ => {
                // Try to detect format by content
                if content.trim_start().starts_with('{') {
                    Config::from_json(content)
                } else {
                    Config::from_toml(content)
                }
            }
        }
    }

    /// Resolve extends configuration inheritance
    fn resolve_extends(
        &self,
        config: Config,
        extends: &[String],
        base_path: &Path,
    ) -> Result<Config, FshLintError> {
        let base_dir = base_path.parent().unwrap_or_else(|| Path::new("."));
        let mut result_config = Config::default();

        // First, load and merge all extended configs in order
        for extend_path in extends {
            let extended_config_path = if Path::new(extend_path).is_absolute() {
                PathBuf::from(extend_path)
            } else {
                base_dir.join(extend_path)
            };

            if !extended_config_path.exists() {
                return Err(FshLintError::ConfigError {
                    message: format!(
                        "Extended config file not found: {}",
                        extended_config_path.display()
                    ),
                });
            }

            let extended_config = self.load_config_from_path(&extended_config_path)?;
            result_config = self.merge_configs(result_config, extended_config);
        }

        // Finally, merge the current config on top
        result_config = self.merge_configs(result_config, config);

        Ok(result_config)
    }
}

impl Config {
    /// Validate the configuration for correctness
    pub fn validate(&self) -> Result<(), FshLintError> {
        // Validate include patterns are not empty
        if self.include_patterns.is_empty() {
            return Err(FshLintError::ConfigError {
                message: "Include patterns cannot be empty".to_string(),
            });
        }

        // Validate formatter configuration
        self.formatter.validate()?;

        // Validate rule configurations
        for (rule_id, rule_config) in &self.rules {
            rule_config.validate(rule_id)?;
        }

        // Validate overrides
        for (index, override_config) in self.overrides.iter().enumerate() {
            override_config.validate(index)?;
        }

        // Validate environment settings
        self.env.validate()?;

        Ok(())
    }

    /// Generate JSON schema for configuration validation
    pub fn json_schema() -> serde_json::Value {
        serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "FSH Lint Configuration",
            "description": "Configuration schema for fsh-lint-rs",
            "type": "object",
            "properties": {
                "extends": {
                    "description": "Configuration files to extend from",
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "include_patterns": {
                    "description": "File patterns to include",
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "minItems": 1,
                    "default": ["**/*.fsh"]
                },
                "exclude_patterns": {
                    "description": "File patterns to exclude",
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "default": ["node_modules/**", "target/**"]
                },
                "ignore_files": {
                    "description": "Custom ignore files to respect",
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "default": [".fshlintignore"]
                },
                "rules_dir": {
                    "description": "Directories containing custom rules",
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "rules": {
                    "description": "Rule-specific configuration",
                    "type": "object",
                    "additionalProperties": {
                        "$ref": "#/definitions/RuleConfig"
                    }
                },
                "overrides": {
                    "description": "Directory-specific configuration overrides",
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Override"
                    }
                },
                "env": {
                    "$ref": "#/definitions/Environment"
                },
                "formatter": {
                    "$ref": "#/definitions/FormatterConfig"
                },
                "autofix": {
                    "$ref": "#/definitions/AutofixConfig"
                }
            },
            "definitions": {
                "RuleConfig": {
                    "type": "object",
                    "properties": {
                        "severity": {
                            "type": "string",
                            "enum": ["error", "warning", "info", "hint"]
                        },
                        "options": {
                            "type": "object",
                            "additionalProperties": true
                        }
                    }
                },
                "Override": {
                    "type": "object",
                    "properties": {
                        "files": {
                            "type": "string",
                            "description": "Glob pattern for files to apply override to"
                        },
                        "config": {
                            "$ref": "#"
                        }
                    },
                    "required": ["files", "config"]
                },
                "Environment": {
                    "type": "object",
                    "properties": {
                        "fhir_version": {
                            "type": "string",
                            "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+$"
                        },
                        "context_paths": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        }
                    }
                },
                "FormatterConfig": {
                    "type": "object",
                    "properties": {
                        "indent_size": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 8,
                            "default": 2
                        },
                        "max_line_width": {
                            "type": "integer",
                            "minimum": 40,
                            "maximum": 200,
                            "default": 100
                        },
                        "align_carets": {
                            "type": "boolean",
                            "default": true
                        }
                    }
                },
                "AutofixConfig": {
                    "type": "object",
                    "properties": {
                        "enable_safe": {
                            "type": "boolean",
                            "default": true
                        },
                        "enable_unsafe": {
                            "type": "boolean",
                            "default": false
                        }
                    }
                }
            }
        })
    }
}

impl RuleConfig {
    /// Validate rule configuration
    pub fn validate(&self, rule_id: &str) -> Result<(), FshLintError> {
        // Validate that rule_id is not empty
        if rule_id.is_empty() {
            return Err(FshLintError::ConfigError {
                message: "Rule ID cannot be empty".to_string(),
            });
        }

        // Validate severity if provided
        if let Some(severity) = &self.severity {
            // Severity validation is handled by the enum itself
            let _ = severity;
        }

        Ok(())
    }
}

impl Override {
    /// Validate override configuration
    pub fn validate(&self, index: usize) -> Result<(), FshLintError> {
        // Validate files pattern is not empty
        if self.files.is_empty() {
            return Err(FshLintError::ConfigError {
                message: format!("Override {} files pattern cannot be empty", index),
            });
        }

        // Validate the nested config
        self.config.validate().map_err(|e| match e {
            FshLintError::ConfigError { message } => FshLintError::ConfigError {
                message: format!("Override {}: {}", index, message),
            },
            other => other,
        })?;

        Ok(())
    }
}

impl Environment {
    /// Validate environment configuration
    pub fn validate(&self) -> Result<(), FshLintError> {
        // Validate FHIR version format if provided
        if let Some(version) = &self.fhir_version {
            if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return Err(FshLintError::ConfigError {
                    message: format!("Invalid FHIR version format: {}", version),
                });
            }

            let parts: Vec<&str> = version.split('.').collect();
            if parts.len() != 3 {
                return Err(FshLintError::ConfigError {
                    message: format!("FHIR version must be in format X.Y.Z: {}", version),
                });
            }

            for part in parts {
                if part.parse::<u32>().is_err() {
                    return Err(FshLintError::ConfigError {
                        message: format!("Invalid FHIR version format: {}", version),
                    });
                }
            }
        }

        Ok(())
    }
}

impl FormatterConfig {
    /// Validate formatter configuration
    pub fn validate(&self) -> Result<(), FshLintError> {
        // Validate indent size
        if !(1..=8).contains(&self.indent_size) {
            return Err(FshLintError::ConfigError {
                message: format!(
                    "Indent size must be between 1 and 8, got {}",
                    self.indent_size
                ),
            });
        }

        // Validate max line width
        if !(40..=200).contains(&self.max_line_width) {
            return Err(FshLintError::ConfigError {
                message: format!(
                    "Max line width must be between 40 and 200, got {}",
                    self.max_line_width
                ),
            });
        }

        Ok(())
    }
}

/// Serialization utilities for configuration
impl Config {
    /// Serialize configuration to JSON string
    pub fn to_json(&self) -> Result<String, FshLintError> {
        serde_json::to_string_pretty(self).map_err(|e| FshLintError::ConfigError {
            message: format!("Failed to serialize config to JSON: {}", e),
        })
    }

    /// Serialize configuration to TOML string
    pub fn to_toml(&self) -> Result<String, FshLintError> {
        toml::to_string_pretty(self).map_err(|e| FshLintError::ConfigError {
            message: format!("Failed to serialize config to TOML: {}", e),
        })
    }

    /// Deserialize configuration from JSON string
    pub fn from_json(json: &str) -> Result<Self, FshLintError> {
        let config: Config = serde_json::from_str(json).map_err(|e| FshLintError::ConfigError {
            message: format!("Failed to parse JSON config: {}", e),
        })?;

        config.validate()?;
        Ok(config)
    }

    /// Deserialize configuration from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, FshLintError> {
        let config: Config = toml::from_str(toml_str).map_err(|e| FshLintError::ConfigError {
            message: format!("Failed to parse TOML config: {}", e),
        })?;

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration against JSON schema
    pub fn validate_against_schema(&self) -> Result<(), FshLintError> {
        // For now, we'll use our built-in validation
        // In the future, this could use a proper JSON schema validator
        self.validate()
    }

    /// Get the JSON schema as a formatted string
    pub fn json_schema_string() -> Result<String, FshLintError> {
        serde_json::to_string_pretty(&Self::json_schema()).map_err(|e| FshLintError::ConfigError {
            message: format!("Failed to serialize JSON schema: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // Helper function to create a temporary config file
    fn create_temp_config(content: &str, filename: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(filename);
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        (temp_dir, config_path)
    }

    // Helper function to create a temporary directory structure
    fn create_temp_dir_structure() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Create nested directories
        fs::create_dir_all(temp_dir.path().join("project/src")).unwrap();
        fs::create_dir_all(temp_dir.path().join("project/test")).unwrap();

        temp_dir
    }

    #[test]
    fn test_default_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_json_serialization() {
        let config = Config::default();
        let json = config.to_json().unwrap();
        let deserialized = Config::from_json(&json).unwrap();
        assert_eq!(config.include_patterns, deserialized.include_patterns);
        assert_eq!(config.exclude_patterns, deserialized.exclude_patterns);
    }

    #[test]
    fn test_config_toml_serialization() {
        let config = Config::default();
        let toml_str = config.to_toml().unwrap();
        let deserialized = Config::from_toml(&toml_str).unwrap();
        assert_eq!(config.include_patterns, deserialized.include_patterns);
        assert_eq!(config.exclude_patterns, deserialized.exclude_patterns);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let mut config = Config::default();
        config.formatter.indent_size = 4;
        config.formatter.max_line_width = 120;
        config.autofix.enable_unsafe = true;
        config.rules.insert(
            "test-rule".to_string(),
            RuleConfig {
                severity: Some(crate::diagnostics::Severity::Warning),
                options: {
                    let mut opts = HashMap::new();
                    opts.insert(
                        "test_option".to_string(),
                        serde_json::Value::String("test_value".to_string()),
                    );
                    opts
                },
            },
        );

        // Test JSON roundtrip
        let json = config.to_json().unwrap();
        let json_deserialized = Config::from_json(&json).unwrap();
        assert_eq!(
            config.formatter.indent_size,
            json_deserialized.formatter.indent_size
        );
        assert_eq!(
            config.autofix.enable_unsafe,
            json_deserialized.autofix.enable_unsafe
        );
        assert!(json_deserialized.rules.contains_key("test-rule"));

        // Test TOML roundtrip
        let toml_str = config.to_toml().unwrap();
        let toml_deserialized = Config::from_toml(&toml_str).unwrap();
        assert_eq!(
            config.formatter.indent_size,
            toml_deserialized.formatter.indent_size
        );
        assert_eq!(
            config.autofix.enable_unsafe,
            toml_deserialized.autofix.enable_unsafe
        );
        assert!(toml_deserialized.rules.contains_key("test-rule"));
    }

    #[test]
    fn test_formatter_config_validation() {
        let mut config = FormatterConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid indent size
        config.indent_size = 0;
        assert!(config.validate().is_err());

        config.indent_size = 10;
        assert!(config.validate().is_err());

        // Test valid indent size
        config.indent_size = 4;
        assert!(config.validate().is_ok());

        // Test invalid line width
        config.max_line_width = 30;
        assert!(config.validate().is_err());

        config.max_line_width = 250;
        assert!(config.validate().is_err());

        // Test valid line width
        config.max_line_width = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_environment_validation() {
        let mut env = Environment::default();
        assert!(env.validate().is_ok());

        // Test invalid FHIR version formats
        env.fhir_version = Some("invalid".to_string());
        assert!(env.validate().is_err());

        env.fhir_version = Some("4.0".to_string());
        assert!(env.validate().is_err());

        env.fhir_version = Some("4.0.1.2".to_string());
        assert!(env.validate().is_err());

        env.fhir_version = Some("4.a.1".to_string());
        assert!(env.validate().is_err());

        // Test valid FHIR version
        env.fhir_version = Some("4.0.1".to_string());
        assert!(env.validate().is_ok());

        env.fhir_version = Some("5.0.0".to_string());
        assert!(env.validate().is_ok());
    }

    #[test]
    fn test_json_schema_generation() {
        let schema = Config::json_schema();
        assert!(schema.is_object());
        assert!(schema["properties"].is_object());
        assert!(schema["definitions"].is_object());

        // Verify specific schema properties
        let properties = &schema["properties"];
        assert!(properties["include_patterns"].is_object());
        assert!(properties["exclude_patterns"].is_object());
        assert!(properties["rules"].is_object());
        assert!(properties["formatter"].is_object());
        assert!(properties["autofix"].is_object());

        // Verify definitions
        let definitions = &schema["definitions"];
        assert!(definitions["RuleConfig"].is_object());
        assert!(definitions["FormatterConfig"].is_object());
        assert!(definitions["AutofixConfig"].is_object());
    }

    #[test]
    fn test_config_validation_errors() {
        // Test empty include patterns
        let mut config = Config::default();
        config.include_patterns.clear();
        let result = config.validate();
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Include patterns cannot be empty"));
        }

        // Test invalid formatter config
        config = Config::default();
        config.formatter.indent_size = 0;
        let result = config.validate();
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Indent size must be between 1 and 8"));
        }

        // Test invalid environment config
        config = Config::default();
        config.env.fhir_version = Some("invalid".to_string());
        let result = config.validate();
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Invalid FHIR version format"));
        }
    }

    #[test]
    fn test_rule_config_validation() {
        let rule_config = RuleConfig {
            severity: Some(crate::diagnostics::Severity::Error),
            options: HashMap::new(),
        };
        assert!(rule_config.validate("test-rule").is_ok());

        // Test empty rule ID
        let result = rule_config.validate("");
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Rule ID cannot be empty"));
        }
    }

    #[test]
    fn test_override_validation() {
        let mut config = Config::default();

        // Test valid override
        let override_config = Override {
            files: "src/**/*.fsh".to_string(),
            config: Config::default(),
        };
        config.overrides.push(override_config);
        assert!(config.validate().is_ok());

        // Test invalid override with empty files pattern
        let invalid_override = Override {
            files: "".to_string(),
            config: Config::default(),
        };
        config.overrides.clear();
        config.overrides.push(invalid_override);
        let result = config.validate();
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Override 0 files pattern cannot be empty"));
        }

        // Test override with invalid nested config
        let mut invalid_nested_config = Config::default();
        invalid_nested_config.include_patterns.clear();
        let override_with_invalid_config = Override {
            files: "src/**/*.fsh".to_string(),
            config: invalid_nested_config,
        };
        config.overrides.clear();
        config.overrides.push(override_with_invalid_config);
        let result = config.validate();
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Override 0: Include patterns cannot be empty"));
        }
    }

    #[test]
    fn test_config_manager_merge_comprehensive() {
        let manager = DefaultConfigManager::new();

        // Create base config
        let mut base = Config::default();
        base.include_patterns = vec!["**/*.fsh".to_string()];
        base.exclude_patterns = vec!["node_modules/**".to_string()];
        base.formatter.indent_size = 2;
        base.formatter.max_line_width = 80;
        base.formatter.align_carets = true;
        base.autofix.enable_safe = true;
        base.autofix.enable_unsafe = false;
        base.env.fhir_version = Some("4.0.1".to_string());
        base.rules.insert(
            "rule1".to_string(),
            RuleConfig {
                severity: Some(crate::diagnostics::Severity::Warning),
                options: HashMap::new(),
            },
        );

        // Create override config
        let mut override_config = Config::default();
        override_config.include_patterns = vec!["src/**/*.fsh".to_string()];
        override_config.exclude_patterns = vec!["build/**".to_string()];
        override_config.formatter.indent_size = 4;
        override_config.formatter.max_line_width = 120;
        override_config.autofix.enable_unsafe = true;
        override_config.env.fhir_version = Some("5.0.0".to_string());
        override_config.rules.insert(
            "rule2".to_string(),
            RuleConfig {
                severity: Some(crate::diagnostics::Severity::Error),
                options: HashMap::new(),
            },
        );
        override_config.rules.insert(
            "rule1".to_string(),
            RuleConfig {
                severity: Some(crate::diagnostics::Severity::Error),
                options: HashMap::new(),
            },
        );

        let merged = manager.merge_configs(base, override_config);

        // Verify merging behavior
        assert_eq!(merged.include_patterns, vec!["src/**/*.fsh".to_string()]);
        assert_eq!(merged.exclude_patterns, vec!["build/**".to_string()]);
        assert_eq!(merged.formatter.indent_size, 4);
        assert_eq!(merged.formatter.max_line_width, 120);
        assert!(merged.formatter.align_carets); // Should keep base value
        assert!(merged.autofix.enable_safe); // Should keep base value
        assert!(merged.autofix.enable_unsafe); // Should use override value
        assert_eq!(merged.env.fhir_version, Some("5.0.0".to_string()));

        // Verify rule merging
        assert!(merged.rules.contains_key("rule1"));
        assert!(merged.rules.contains_key("rule2"));
        assert_eq!(
            merged.rules.get("rule1").unwrap().severity,
            Some(crate::diagnostics::Severity::Error)
        );
        assert_eq!(
            merged.rules.get("rule2").unwrap().severity,
            Some(crate::diagnostics::Severity::Error)
        );
    }

    #[test]
    fn test_config_discovery_integration() {
        let temp_dir = create_temp_dir_structure();
        let manager = DefaultConfigManager::new();

        // Test discovery when no config exists
        let project_dir = temp_dir.path().join("project");
        let discovered = manager.discover_config(&project_dir);
        assert!(discovered.is_none());

        // Create config in project root
        let config_content = r#"{"include_patterns": ["**/*.fsh"]}"#;
        let config_path = project_dir.join(".fshlintrc.json");
        fs::write(&config_path, config_content).unwrap();

        // Test discovery from project root
        let discovered = manager.discover_config(&project_dir);
        assert!(discovered.is_some());
        assert_eq!(discovered.unwrap(), config_path);

        // Test discovery from subdirectory
        let src_dir = project_dir.join("src");
        let discovered = manager.discover_config(&src_dir);
        assert!(discovered.is_some());
        assert_eq!(discovered.unwrap(), config_path);

        // Test discovery with different config file names
        fs::remove_file(&config_path).unwrap();
        let toml_config_path = project_dir.join(".fshlintrc.toml");
        fs::write(&toml_config_path, "include_patterns = [\"**/*.fsh\"]").unwrap();

        let discovered = manager.discover_config(&src_dir);
        assert!(discovered.is_some());
        assert_eq!(discovered.unwrap(), toml_config_path);
    }

    #[test]
    fn test_config_loading_with_extends() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        // Create base config
        let base_config_content = r#"
        {
            "include_patterns": ["**/*.fsh"],
            "formatter": {
                "indent_size": 2,
                "max_line_width": 80
            },
            "rules": {
                "rule1": {"severity": "Warning"}
            }
        }
        "#;
        let base_config_path = base_dir.join("base.json");
        fs::write(&base_config_path, base_config_content).unwrap();

        // Create extending config
        let extending_config_content = format!(
            r#"
        {{
            "extends": ["{}"],
            "include_patterns": ["src/**/*.fsh"],
            "formatter": {{
                "indent_size": 4
            }},
            "rules": {{
                "rule2": {{"severity": "Error"}}
            }}
        }}
        "#,
            base_config_path.display()
        );
        let extending_config_path = base_dir.join("extending.json");
        fs::write(&extending_config_path, extending_config_content).unwrap();

        let manager = DefaultConfigManager::new();
        let config = manager.load_config(Some(&extending_config_path)).unwrap();

        // Verify inheritance
        assert_eq!(config.include_patterns, vec!["src/**/*.fsh".to_string()]);
        assert_eq!(config.formatter.indent_size, 4);
        assert_eq!(config.formatter.max_line_width, 80); // Inherited
        assert!(config.rules.contains_key("rule1")); // Inherited
        assert!(config.rules.contains_key("rule2")); // Added
    }

    #[test]
    fn test_config_loading_error_handling() {
        let manager = DefaultConfigManager::new();

        // Test loading non-existent file
        let result = manager.load_config(Some(Path::new("non_existent.json")));
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Failed to read config file"));
        }

        // Test loading invalid JSON
        let (_temp_dir, invalid_json_path) =
            create_temp_config(r#"{"invalid": json}"#, "invalid.json");
        let result = manager.load_config(Some(&invalid_json_path));
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Failed to parse JSON config"));
        }

        // Test loading invalid TOML
        let (_temp_dir, invalid_toml_path) = create_temp_config(r#"[invalid toml"#, "invalid.toml");
        let result = manager.load_config(Some(&invalid_toml_path));
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Failed to parse TOML config"));
        }

        // Test loading config with validation errors
        let (_temp_dir, invalid_config_path) =
            create_temp_config(r#"{"include_patterns": []}"#, "invalid_config.json");
        let result = manager.load_config(Some(&invalid_config_path));
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Include patterns cannot be empty"));
        }
    }

    #[test]
    fn test_config_extends_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        // Test extending non-existent config
        let extending_config_content = r#"
        {
            "extends": ["non_existent.json"],
            "include_patterns": ["**/*.fsh"]
        }
        "#;
        let extending_config_path = base_dir.join("extending.json");
        fs::write(&extending_config_path, extending_config_content).unwrap();

        let manager = DefaultConfigManager::new();
        let result = manager.load_config(Some(&extending_config_path));
        assert!(result.is_err());
        if let Err(FshLintError::ConfigError { message }) = result {
            assert!(message.contains("Extended config file not found"));
        }
    }

    #[test]
    fn test_parse_config_content_format_detection() {
        let manager = DefaultConfigManager::new();

        // Test JSON detection by content
        let json_content = r#"
        {
            "include_patterns": ["test/**/*.fsh"],
            "formatter": {
                "indent_size": 4
            }
        }
        "#;
        let path = Path::new("config"); // No extension
        let config = manager.parse_config_content(json_content, path).unwrap();
        assert_eq!(config.include_patterns, vec!["test/**/*.fsh".to_string()]);
        assert_eq!(config.formatter.indent_size, 4);

        // Test TOML detection by content
        let toml_content = r#"
        include_patterns = ["test/**/*.fsh"]
        
        [formatter]
        indent_size = 4
        "#;
        let config = manager.parse_config_content(toml_content, path).unwrap();
        assert_eq!(config.include_patterns, vec!["test/**/*.fsh".to_string()]);
        assert_eq!(config.formatter.indent_size, 4);
    }

    #[test]
    fn test_config_manager_load_default() {
        let manager = DefaultConfigManager::new();

        // Loading without a path should return default config
        let config = manager.load_config(None).unwrap();
        assert_eq!(config.include_patterns, Config::default().include_patterns);
        assert_eq!(config.exclude_patterns, Config::default().exclude_patterns);
        assert_eq!(
            config.formatter.indent_size,
            Config::default().formatter.indent_size
        );
    }

    #[test]
    fn test_config_loading_from_existing_test_files() {
        let manager = DefaultConfigManager::new();

        // Test loading from test.fshlintrc.json
        let config_path = Path::new("test_configs/test.fshlintrc.json");
        if config_path.exists() {
            let config = manager.load_config(Some(config_path)).unwrap();

            assert_eq!(
                config.include_patterns,
                vec!["src/**/*.fsh", "test/**/*.fsh"]
            );
            assert_eq!(config.exclude_patterns, vec!["node_modules/**", "build/**"]);
            assert_eq!(config.formatter.indent_size, 4);
            assert_eq!(config.formatter.max_line_width, 120);
            assert!(config.autofix.enable_safe);
            assert!(!config.autofix.enable_unsafe);
            assert_eq!(config.env.fhir_version, Some("4.0.1".to_string()));

            // Check rules
            assert!(config.rules.contains_key("no-trailing-whitespace"));
            assert!(config.rules.contains_key("require-description"));

            if let Some(rule_config) = config.rules.get("no-trailing-whitespace") {
                assert_eq!(
                    rule_config.severity,
                    Some(crate::diagnostics::Severity::Warning)
                );
            }

            if let Some(rule_config) = config.rules.get("require-description") {
                assert_eq!(
                    rule_config.severity,
                    Some(crate::diagnostics::Severity::Error)
                );
                assert!(rule_config.options.contains_key("min_length"));
            }
        }
    }

    #[test]
    fn test_config_extends_functionality() {
        let manager = DefaultConfigManager::new();
        let config_path = Path::new("test_configs/extended.fshlintrc.json");

        if config_path.exists() {
            let config = manager.load_config(Some(config_path)).unwrap();

            // Should inherit from base config
            assert_eq!(config.include_patterns, vec!["src/**/*.fsh"]); // Overridden
            assert_eq!(config.formatter.indent_size, 4); // Overridden
            assert_eq!(config.formatter.max_line_width, 100); // Inherited from base

            // Check rule merging
            assert!(config.rules.contains_key("basic-syntax")); // Inherited
            assert!(config.rules.contains_key("naming-convention")); // Overridden
            assert!(config.rules.contains_key("require-title")); // Added

            // Check that naming-convention severity was overridden
            if let Some(rule_config) = config.rules.get("naming-convention") {
                assert_eq!(
                    rule_config.severity,
                    Some(crate::diagnostics::Severity::Error)
                );
            }

            // Check that basic-syntax was inherited
            if let Some(rule_config) = config.rules.get("basic-syntax") {
                assert_eq!(
                    rule_config.severity,
                    Some(crate::diagnostics::Severity::Error)
                );
            }
        }
    }

    #[test]
    fn test_config_schema_validation() {
        let config = Config::default();

        // Test that validate_against_schema works
        assert!(config.validate_against_schema().is_ok());

        // Test with invalid config
        let mut invalid_config = Config::default();
        invalid_config.include_patterns.clear();
        assert!(invalid_config.validate_against_schema().is_err());
    }

    #[test]
    fn test_json_schema_string_generation() {
        let schema_string = Config::json_schema_string().unwrap();
        assert!(schema_string.contains("FSH Lint Configuration"));
        assert!(schema_string.contains("properties"));
        assert!(schema_string.contains("definitions"));

        // Verify it's valid JSON
        let _: serde_json::Value = serde_json::from_str(&schema_string).unwrap();
    }

    #[test]
    fn test_config_merge_with_extends() {
        let manager = DefaultConfigManager::new();

        // Test merging with extends field
        let mut base = Config::default();
        let mut override_config = Config::default();
        override_config.extends = Some(vec!["base.json".to_string()]);

        let merged = manager.merge_configs(base, override_config);
        assert_eq!(merged.extends, Some(vec!["base.json".to_string()]));

        // Test merging rules directories
        base = Config::default();
        base.rules_dir = vec![PathBuf::from("rules1")];
        override_config = Config::default();
        override_config.rules_dir = vec![PathBuf::from("rules2")];

        let merged = manager.merge_configs(base, override_config);
        assert_eq!(merged.rules_dir.len(), 2);
        assert!(merged.rules_dir.contains(&PathBuf::from("rules1")));
        assert!(merged.rules_dir.contains(&PathBuf::from("rules2")));

        // Test merging overrides
        base = Config::default();
        base.overrides = vec![Override {
            files: "src/**/*.fsh".to_string(),
            config: Config::default(),
        }];
        override_config = Config::default();
        override_config.overrides = vec![Override {
            files: "test/**/*.fsh".to_string(),
            config: Config::default(),
        }];

        let merged = manager.merge_configs(base, override_config);
        assert_eq!(merged.overrides.len(), 2);

        // Test merging environment context paths
        base = Config::default();
        base.env.context_paths = vec![PathBuf::from("context1")];
        override_config = Config::default();
        override_config.env.context_paths = vec![PathBuf::from("context2")];

        let merged = manager.merge_configs(base, override_config);
        assert_eq!(merged.env.context_paths.len(), 2);
        assert!(
            merged
                .env
                .context_paths
                .contains(&PathBuf::from("context1"))
        );
        assert!(
            merged
                .env
                .context_paths
                .contains(&PathBuf::from("context2"))
        );
    }

    #[test]
    fn test_config_merge_default_value_handling() {
        let manager = DefaultConfigManager::new();

        // Test that default values are not overridden unless explicitly changed
        let mut base = Config::default();
        base.formatter.indent_size = 4; // Non-default value

        let override_config = Config::default(); // All default values

        let merged = manager.merge_configs(base, override_config);

        // Should keep the base non-default value since override has default
        assert_eq!(merged.formatter.indent_size, 4);
        assert_eq!(
            merged.formatter.max_line_width,
            FormatterConfig::default().max_line_width
        );
        assert_eq!(
            merged.formatter.align_carets,
            FormatterConfig::default().align_carets
        );
    }
}
