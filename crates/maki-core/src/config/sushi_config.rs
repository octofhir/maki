//! SUSHI-compatible configuration parser (sushi-config.yaml)
//!
//! This module provides full compatibility with SUSHI's sushi-config.yaml format,
//! allowing maki to read and validate FHIR Implementation Guide configurations.
//!
//! **FHIR IG specification**: <http://hl7.org/fhir/R4/implementationguide.html>
//! **SUSHI documentation**: <https://fshschool.org/docs/sushi/configuration/>
//! **NPM Package spec**: <https://confluence.hl7.org/display/FHIR/NPM+Package+Specification>
//!
//! ## Example Configuration
//!
//! ```yaml
//! canonical: http://example.org/fhir/example-ig
//! name: ExampleIG
//! id: example.fhir.ig
//! version: 1.0.0
//! fhirVersion: 4.0.1
//! status: draft
//! publisher: Example Organization
//! dependencies:
//!   hl7.fhir.us.core: 5.0.1
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main SUSHI configuration structure
///
/// This corresponds to the sushi-config.yaml file that defines
/// Implementation Guide metadata and build settings.
///
/// **Reference**: <https://fshschool.org/docs/sushi/configuration/>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SushiConfiguration {
    // === Core IG Metadata (Required) ===
    /// Canonical URL for the IG (required)
    pub canonical: String,

    /// FHIR version(s) - can be single string or array
    #[serde(deserialize_with = "deserialize_fhir_version")]
    pub fhir_version: Vec<String>,

    // === Basic Metadata ===
    /// Unique identifier for the IG
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Computer-friendly name (PascalCase recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Human-friendly title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Version string (semver recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Publication status (draft | active | retired | unknown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Whether this is experimental
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,

    /// Publication date (YYYY, YYYY-MM, or YYYY-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,

    /// Publisher name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    /// Contact details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Vec<ContactDetail>>,

    /// Description (markdown supported)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Use context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_context: Option<Vec<UsageContext>>,

    /// Jurisdiction (countries/regions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<Vec<CodeableConcept>>,

    /// Copyright statement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,

    /// Copyright label (R5+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright_label: Option<String>,

    /// Version algorithm string (R5+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_algorithm_string: Option<String>,

    /// Version algorithm coding (R5+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_algorithm_coding: Option<Coding>,

    // === Package Metadata ===
    /// NPM package ID (defaults to id if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,

    /// SPDX license identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    // === Dependencies ===
    /// IG dependencies (package-id: version)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, DependencyVersion>>,

    /// Global resource profiles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global: Option<Vec<GlobalProfile>>,

    // === Resource Grouping ===
    /// Resource groups for organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<ResourceGroup>>,

    /// Resources to include in IG
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<ResourceEntry>>,

    // === Pages ===
    /// IG page structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<PageDefinition>>,

    /// Content for generated index page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_page_content: Option<String>,

    // === IG Parameters ===
    /// IG build parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,

    // === Templates ===
    /// IG definition templates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<Vec<Template>>,

    // === Menu ===
    /// Navigation menu structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu: Option<Vec<MenuItem>>,

    // === SUSHI-specific Options ===
    /// FSH-only mode (no IG content generation)
    #[serde(rename = "FSHOnly", skip_serializing_if = "Option::is_none")]
    pub fsh_only: Option<bool>,

    /// Apply extension metadata to root element
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_extension_metadata_to_root: Option<bool>,

    /// Instance processing options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_options: Option<InstanceOptions>,

    // === Other IG.definition Properties ===
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit_rules: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contained: Option<Vec<serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Vec<serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier_extension: Option<Vec<serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Definition-level extensions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<DefinitionExtension>,
}

/// Custom deserializer for fhirVersion - handles both string and array
fn deserialize_fhir_version<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FhirVersionValue {
        Single(String),
        Multiple(Vec<String>),
    }

    match FhirVersionValue::deserialize(deserializer)? {
        FhirVersionValue::Single(s) => Ok(vec![s]),
        FhirVersionValue::Multiple(v) => {
            if v.is_empty() {
                Err(D::Error::custom("fhirVersion array cannot be empty"))
            } else {
                Ok(v)
            }
        }
    }
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub telecom: Option<Vec<ContactPoint>>,
}

/// Contact point (phone, email, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactPoint {
    pub system: String,  // phone | fax | email | pager | url | sms | other
    pub value: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_field: Option<String>,  // home | work | temp | old | mobile

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<serde_json::Value>,
}

/// Usage context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageContext {
    pub code: Coding,

    #[serde(flatten)]
    pub value: serde_json::Value,
}

/// CodeableConcept
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeableConcept {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<Coding>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Coding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Coding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_selected: Option<bool>,
}

/// Dependency version - can be string or object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum DependencyVersion {
    /// Simple version string
    Simple(String),

    /// Complex dependency with additional properties
    Complex {
        version: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        uri: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        extension: Option<Vec<serde_json::Value>>,
    },
}

/// Global resource profile
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalProfile {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub profile: String,
}

/// Resource group
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceGroup {
    pub id: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<String>>,
}

/// Resource entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceEntry {
    pub reference: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_boolean: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_canonical: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub omit: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Vec<serde_json::Value>>,
}

/// Page definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PageDefinition {
    #[serde(flatten)]
    pub name_or_url: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<Vec<PageDefinition>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Vec<serde_json::Value>>,
}

/// IG parameter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Parameter {
    pub code: String,
    pub value: String,
}

/// Template definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Template {
    pub code: String,
    pub source: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Menu item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MenuItem {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_in_new_tab: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_menu: Option<Vec<MenuItem>>,
}

/// Instance processing options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceOptions {
    /// When to set meta.profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_meta_profile: Option<MetaProfileSetting>,

    /// When to set id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_id: Option<IdSetting>,

    /// Require manual slice ordering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_slice_ordering: Option<bool>,
}

/// Meta profile setting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MetaProfileSetting {
    Always,
    Never,
    InlineOnly,
    StandaloneOnly,
}

/// ID setting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum IdSetting {
    Always,
    StandaloneOnly,
}

/// Definition extension container
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefinitionExtension {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Vec<serde_json::Value>>,
}

impl SushiConfiguration {
    /// Parse sushi-config.yaml from a file
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(path.to_path_buf(), e))?;

        Self::from_yaml(&contents)
            .map_err(|e| ConfigError::ParseError(path.to_path_buf(), e.to_string()))
    }

    /// Parse sushi-config.yaml from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Required fields
        if self.canonical.is_empty() {
            errors.push("canonical is required".to_string());
        }

        if self.fhir_version.is_empty() {
            errors.push("fhirVersion is required".to_string());
        }

        // Validate canonical URL format
        if !self.canonical.starts_with("http://") && !self.canonical.starts_with("https://") {
            errors.push(format!("canonical must be a valid URL: {}", self.canonical));
        }

        // Validate FHIR versions
        for version in &self.fhir_version {
            if !is_valid_fhir_version(version) {
                errors.push(format!("invalid FHIR version: {}", version));
            }
        }

        // Validate status if present
        if let Some(ref status) = self.status {
            if !matches!(status.as_str(), "draft" | "active" | "retired" | "unknown") {
                errors.push(format!("invalid status: {} (must be draft, active, retired, or unknown)", status));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get the package ID (id or packageId)
    pub fn package_id(&self) -> Option<&str> {
        self.package_id.as_deref().or(self.id.as_deref())
    }
}

/// Validate FHIR version string
fn is_valid_fhir_version(version: &str) -> bool {
    // Accept major.minor or major.minor.patch
    // Common versions: 4.0.1, 4.3.0, 5.0.0, etc.
    let parts: Vec<&str> = version.split('.').collect();
    matches!(parts.len(), 2 | 3)
        && parts.iter().all(|p| p.parse::<u32>().is_ok())
}

/// Configuration error types
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error reading {0}: {1}")]
    IoError(std::path::PathBuf, std::io::Error),

    #[error("Parse error in {0}: {1}")]
    ParseError(std::path::PathBuf, String),

    #[error("Validation errors: {0:?}")]
    ValidationError(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_config() {
        let yaml = r#"
canonical: http://example.org/fhir/example-ig
fhirVersion: 4.0.1
"#;

        let config = SushiConfiguration::from_yaml(yaml).unwrap();
        assert_eq!(config.canonical, "http://example.org/fhir/example-ig");
        assert_eq!(config.fhir_version, vec!["4.0.1"]);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_full_config() {
        let yaml = r#"
canonical: http://example.org/fhir/example-ig
fhirVersion: 4.0.1
id: example.fhir.ig
name: ExampleIG
title: Example Implementation Guide
version: 1.0.0
status: draft
experimental: true
date: 2024-01-01
publisher: Example Organization
description: An example IG for testing
license: CC0-1.0
dependencies:
  hl7.fhir.us.core: 5.0.1
"#;

        let config = SushiConfiguration::from_yaml(yaml).unwrap();
        assert_eq!(config.id, Some("example.fhir.ig".to_string()));
        assert_eq!(config.name, Some("ExampleIG".to_string()));
        assert_eq!(config.status, Some("draft".to_string()));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_multiple_fhir_versions() {
        let yaml = r#"
canonical: http://example.org/fhir/example-ig
fhirVersion:
  - 4.0.1
  - 4.3.0
"#;

        let config = SushiConfiguration::from_yaml(yaml).unwrap();
        assert_eq!(config.fhir_version, vec!["4.0.1", "4.3.0"]);
    }

    #[test]
    fn test_validation_missing_canonical() {
        let config = SushiConfiguration {
            canonical: String::new(),
            fhir_version: vec!["4.0.1".to_string()],
            id: None,
            name: None,
            title: None,
            version: None,
            status: None,
            experimental: None,
            date: None,
            publisher: None,
            contact: None,
            description: None,
            use_context: None,
            jurisdiction: None,
            copyright: None,
            copyright_label: None,
            version_algorithm_string: None,
            version_algorithm_coding: None,
            package_id: None,
            license: None,
            dependencies: None,
            global: None,
            groups: None,
            resources: None,
            pages: None,
            index_page_content: None,
            parameters: None,
            templates: None,
            menu: None,
            fsh_only: None,
            apply_extension_metadata_to_root: None,
            instance_options: None,
            meta: None,
            implicit_rules: None,
            language: None,
            text: None,
            contained: None,
            extension: None,
            modifier_extension: None,
            url: None,
            definition: None,
        };

        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("canonical")));
    }

    #[test]
    fn test_validation_invalid_status() {
        let yaml = r#"
canonical: http://example.org/fhir/example-ig
fhirVersion: 4.0.1
status: invalid-status
"#;

        let config = SushiConfiguration::from_yaml(yaml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("status")));
    }

    #[test]
    fn test_is_valid_fhir_version() {
        assert!(is_valid_fhir_version("4.0.1"));
        assert!(is_valid_fhir_version("4.3.0"));
        assert!(is_valid_fhir_version("5.0.0"));
        assert!(is_valid_fhir_version("4.0"));
        assert!(!is_valid_fhir_version("4"));
        assert!(!is_valid_fhir_version("invalid"));
        assert!(!is_valid_fhir_version("4.0.1.2"));
    }
}
