//! Unified configuration structure for MAKI
//!
//! This module provides a clean section-based configuration format that maintains
//! 100% SUSHI field compatibility while organizing settings into logical sections:
//! - `build`: SUSHI-compatible build configuration (all sushi-config.yaml fields)
//! - `linter`: Linter rules and settings
//! - `formatter`: Code formatting preferences
//! - `files`: File discovery patterns
//!
//! MAKI uses only the unified format (`maki.yaml` or `maki.json`). There is no
//! backward compatibility with old formats since MAKI has no existing users.
//!
//! ## Example Configuration (maki.yaml)
//!
//! ```yaml
//! # === Shared Dependencies (available to build and linter) ===
//! dependencies:
//!   hl7.fhir.us.core: 6.1.0
//!   hl7.terminology.r4: 5.3.0
//!
//! # === Build Configuration (SUSHI-compatible) ===
//! build:
//!   id: my.example.ig
//!   canonical: http://example.org/fhir/my-ig
//!   name: MyIG
//!   title: My Implementation Guide
//!   status: draft
//!   version: 0.1.0
//!   fhirVersion: 4.0.1
//!
//!   publisher:
//!     name: Example Publisher
//!     url: http://example.org
//!
//! # === Linter Configuration ===
//! linter:
//!   enabled: true
//!   rules:
//!     recommended: true
//!     correctness:
//!       duplicate-definition: error
//!
//! # === Formatter Configuration ===
//! formatter:
//!   enabled: true
//!   indentSize: 2
//!   lineWidth: 100
//!
//! # === File Discovery ===
//! files:
//!   include:
//!     - "input/fsh/**/*.fsh"
//!   exclude:
//!     - "**/*.draft.fsh"
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::{
    DependencyVersion, FilesConfiguration, FormatterConfiguration, LinterConfiguration,
    SushiConfiguration,
};

/// Unified configuration with clean section-based structure
///
/// This is the new MAKI configuration format that organizes all settings
/// into logical sections while maintaining 100% SUSHI compatibility in the
/// `build` section.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedConfig {
    /// Top-level dependencies (shared across build and linter)
    ///
    /// These dependencies are available to both the build process and linter.
    /// Useful for sharing package definitions without duplication.
    /// Format: `package-id: version` or complex dependency specs.
    ///
    /// Example:
    /// ```yaml
    /// dependencies:
    ///   hl7.fhir.us.core: 6.1.0
    ///   hl7.terminology.r4: 5.3.0
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, DependencyVersion>>,

    /// Build configuration (SUSHI-compatible fields)
    ///
    /// This section contains all fields from sushi-config.yaml, allowing
    /// MAKI to act as a drop-in replacement for SUSHI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<SushiConfiguration>,

    /// Linter configuration
    ///
    /// Controls linting rules, severity levels, and rule directories.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linter: Option<LinterConfiguration>,

    /// Formatter configuration
    ///
    /// Controls code formatting preferences like indent size and line width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatter: Option<FormatterConfiguration>,

    /// File discovery configuration
    ///
    /// Specifies which FSH files to include/exclude from processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<FilesConfiguration>,
}

impl UnifiedConfig {
    /// Load configuration from file
    ///
    /// Supports both YAML (maki.yaml) and JSON (maki.json) formats.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to configuration file
    ///
    /// # Returns
    ///
    /// * `Ok(UnifiedConfig)` - Successfully loaded configuration
    /// * `Err(ConfigError)` - Failed to load or parse configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maki_core::config::UnifiedConfig;
    /// use std::path::Path;
    ///
    /// let config = UnifiedConfig::load(Path::new("maki.yaml"))?;
    /// ```
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str());

        match ext {
            Some("yaml") | Some("yml") => Ok(serde_yaml::from_str(&content)?),
            Some("json") => Ok(serde_json::from_str(&content)?),
            _ => Err("Unsupported file extension (expected .yaml, .yml, or .json)".into()),
        }
    }

    /// Get build configuration (SUSHI-compatible)
    ///
    /// Returns the build section which contains all SUSHI fields.
    pub fn build_config(&self) -> Option<&SushiConfiguration> {
        self.build.as_ref()
    }

    /// Get linter configuration with defaults
    ///
    /// Returns linter configuration, or default if not specified.
    pub fn linter_config(&self) -> LinterConfiguration {
        self.linter.clone().unwrap_or_default()
    }

    /// Get formatter configuration with defaults
    ///
    /// Returns formatter configuration, or default if not specified.
    pub fn formatter_config(&self) -> FormatterConfiguration {
        self.formatter.clone().unwrap_or_default()
    }

    /// Get files configuration with defaults
    ///
    /// Returns files configuration, or default if not specified.
    pub fn files_config(&self) -> FilesConfiguration {
        self.files.clone().unwrap_or_default()
    }

    /// Check if build section exists
    pub fn has_build_config(&self) -> bool {
        self.build.is_some()
    }

    /// Check if this is a valid IG project configuration
    ///
    /// A valid IG project must have a build section with at least
    /// a canonical URL and FHIR version.
    pub fn is_valid_ig_config(&self) -> bool {
        self.build
            .as_ref()
            .map(|b| !b.canonical.is_empty() && !b.fhir_version.is_empty())
            .unwrap_or(false)
    }
}

impl Default for UnifiedConfig {
    fn default() -> Self {
        Self {
            dependencies: None,
            build: None,
            linter: Some(LinterConfiguration::default()),
            formatter: Some(FormatterConfiguration::default()),
            files: Some(FilesConfiguration::default()),
        }
    }
}

impl UnifiedConfig {
    /// Get all dependencies (top-level + build section merged)
    ///
    /// Returns all dependencies with top-level dependencies taking precedence
    /// over build-section dependencies in case of conflicts.
    pub fn all_dependencies(&self) -> HashMap<String, DependencyVersion> {
        let mut deps = HashMap::new();

        // First, add build section dependencies
        if let Some(build) = &self.build {
            if let Some(build_deps) = &build.dependencies {
                deps.extend(build_deps.clone());
            }
        }

        // Then, overlay top-level dependencies (these take precedence)
        if let Some(top_deps) = &self.dependencies {
            deps.extend(top_deps.clone());
        }

        deps
    }

    /// Get top-level dependencies only
    pub fn top_level_dependencies(&self) -> Option<&HashMap<String, DependencyVersion>> {
        self.dependencies.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_config_serialization() {
        let config = UnifiedConfig {
            dependencies: None,
            build: Some(SushiConfiguration {
                canonical: "http://example.org/fhir/my-ig".to_string(),
                fhir_version: vec!["4.0.1".to_string()],
                id: Some("my.example.ig".to_string()),
                name: Some("MyIG".to_string()),
                ..Default::default()
            }),
            linter: Some(LinterConfiguration::default()),
            formatter: Some(FormatterConfiguration::default()),
            files: Some(FilesConfiguration::default()),
        };

        // Test YAML serialization
        let yaml = serde_yaml::to_string(&config).expect("Failed to serialize to YAML");
        assert!(yaml.contains("build:"));
        assert!(yaml.contains("linter:"));
        assert!(yaml.contains("formatter:"));

        // Test JSON serialization
        let json = serde_json::to_string_pretty(&config).expect("Failed to serialize to JSON");
        assert!(json.contains("\"build\""));
        assert!(json.contains("\"linter\""));
    }

    #[test]
    fn test_unified_config_deserialization_yaml() {
        let yaml = r#"
build:
  canonical: http://example.org/fhir/test
  fhirVersion: 4.0.1
  name: TestIG
linter:
  enabled: true
formatter:
  enabled: true
  indentSize: 2
"#;

        let config: UnifiedConfig =
            serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");

        assert!(config.build.is_some());
        let build = config.build.unwrap();
        assert_eq!(build.canonical, "http://example.org/fhir/test");
        assert_eq!(build.name, Some("TestIG".to_string()));
    }

    #[test]
    fn test_unified_config_deserialization_json() {
        let json = r#"
{
  "build": {
    "canonical": "http://example.org/fhir/test",
    "fhirVersion": "4.0.1",
    "name": "TestIG"
  },
  "linter": {
    "enabled": true
  },
  "formatter": {
    "enabled": true,
    "indentSize": 2
  }
}
"#;

        let config: UnifiedConfig =
            serde_json::from_str(json).expect("Failed to deserialize JSON");

        assert!(config.build.is_some());
        assert_eq!(
            config.build.as_ref().unwrap().canonical,
            "http://example.org/fhir/test"
        );
    }

    #[test]
    fn test_default_config() {
        let config = UnifiedConfig::default();

        assert!(config.build.is_none());
        assert!(config.linter.is_some());
        assert!(config.formatter.is_some());
        assert!(config.files.is_some());
    }

    #[test]
    fn test_config_accessors() {
        let config = UnifiedConfig {
            build: Some(SushiConfiguration {
                canonical: "http://test.org".to_string(),
                fhir_version: vec!["4.0.1".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(config.has_build_config());
        assert!(config.is_valid_ig_config());
        assert_eq!(
            config.build_config().unwrap().canonical,
            "http://test.org"
        );
    }

    #[test]
    fn test_invalid_ig_config() {
        // No build section
        let config1 = UnifiedConfig::default();
        assert!(!config1.is_valid_ig_config());

        // Empty canonical
        let config2 = UnifiedConfig {
            build: Some(SushiConfiguration {
                canonical: "".to_string(),
                fhir_version: vec!["4.0.1".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(!config2.is_valid_ig_config());

        // Empty fhir_version
        let config3 = UnifiedConfig {
            build: Some(SushiConfiguration {
                canonical: "http://test.org".to_string(),
                fhir_version: vec![],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(!config3.is_valid_ig_config());
    }

    #[test]
    fn test_dependencies_merging() {
        use super::DependencyVersion;

        let mut top_deps = HashMap::new();
        top_deps.insert(
            "hl7.fhir.us.core".to_string(),
            DependencyVersion::Simple("6.1.0".to_string()),
        );
        top_deps.insert(
            "hl7.terminology.r4".to_string(),
            DependencyVersion::Simple("5.3.0".to_string()),
        );

        let mut build_deps = HashMap::new();
        build_deps.insert(
            "hl7.fhir.us.core".to_string(),
            DependencyVersion::Simple("5.0.0".to_string()), // Will be overridden by top-level
        );
        build_deps.insert(
            "hl7.fhir.us.mcode".to_string(),
            DependencyVersion::Simple("3.0.0".to_string()),
        );

        let config = UnifiedConfig {
            dependencies: Some(top_deps),
            build: Some(SushiConfiguration {
                canonical: "http://test.org".to_string(),
                fhir_version: vec!["4.0.1".to_string()],
                dependencies: Some(build_deps),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Test all_dependencies merges correctly with top-level taking precedence
        let all_deps = config.all_dependencies();
        assert_eq!(all_deps.len(), 3);
        assert_eq!(
            all_deps.get("hl7.fhir.us.core"),
            Some(&DependencyVersion::Simple("6.1.0".to_string())) // Top-level wins
        );
        assert_eq!(
            all_deps.get("hl7.terminology.r4"),
            Some(&DependencyVersion::Simple("5.3.0".to_string()))
        );
        assert_eq!(
            all_deps.get("hl7.fhir.us.mcode"),
            Some(&DependencyVersion::Simple("3.0.0".to_string()))
        );
    }

    #[test]
    fn test_top_level_dependencies_yaml() {
        let yaml = r#"
dependencies:
  hl7.fhir.us.core: 6.1.0
  hl7.terminology.r4: 5.3.0

build:
  canonical: http://example.org/fhir/test
  fhirVersion: 4.0.1
"#;

        let config: UnifiedConfig =
            serde_yaml::from_str(yaml).expect("Failed to deserialize YAML with dependencies");

        assert!(config.dependencies.is_some());
        let deps = config.dependencies.unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains_key("hl7.fhir.us.core"));
        assert!(deps.contains_key("hl7.terminology.r4"));
    }
}
