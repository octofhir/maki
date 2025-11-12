//! Config Generator - Generate SUSHI and Maki configuration files
//!
//! This module handles generation of both:
//! - `sushi-config.yaml` - SUSHI/FSH build configuration
//! - `.makirc.json` - Maki linter/formatter configuration
//!
//! # Examples
//!
//! ## Generate from ImplementationGuide
//!
//! ```no_run
//! use maki_decompiler::config_generator::ConfigGenerator;
//! use maki_decompiler::models::ImplementationGuide;
//! use std::path::Path;
//!
//! let generator = ConfigGenerator::new();
//! // let ig = ...; // Load ImplementationGuide
//! // generator.generate_sushi_config(Some(&ig), Path::new("sushi-config.yaml"))?;
//! ```
//!
//! ## Generate Minimal Configs
//!
//! ```no_run
//! use maki_decompiler::config_generator::ConfigGenerator;
//! use std::path::Path;
//!
//! let generator = ConfigGenerator::new();
//! generator.generate_minimal_sushi_config(Path::new("sushi-config.yaml")).unwrap();
//! generator.generate_maki_config(Path::new(".makirc.json")).unwrap();
//! ```

use crate::error::Result;
use crate::models::ImplementationGuide;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Configuration generator for SUSHI and Maki config files
pub struct ConfigGenerator {
    /// Default FHIR version to use
    default_fhir_version: String,
}

impl ConfigGenerator {
    /// Create a new ConfigGenerator
    pub fn new() -> Self {
        Self {
            default_fhir_version: "4.0.1".to_string(),
        }
    }

    /// Create a ConfigGenerator with custom default FHIR version
    pub fn with_fhir_version(fhir_version: String) -> Self {
        Self {
            default_fhir_version: fhir_version,
        }
    }

    /// Generate SUSHI config from ImplementationGuide or create minimal config
    ///
    /// # Arguments
    ///
    /// * `ig` - Optional ImplementationGuide resource
    /// * `output_path` - Path to write sushi-config.yaml
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if file writing fails
    pub fn generate_sushi_config(
        &self,
        ig: Option<&ImplementationGuide>,
        output_path: &Path,
    ) -> Result<()> {
        let yaml = if let Some(ig) = ig {
            self.sushi_config_from_ig(ig)
        } else {
            self.minimal_sushi_config()
        };

        fs::write(output_path, yaml)?;
        Ok(())
    }

    /// Generate minimal SUSHI config
    pub fn generate_minimal_sushi_config(&self, output_path: &Path) -> Result<()> {
        let yaml = self.minimal_sushi_config();
        fs::write(output_path, yaml)?;
        Ok(())
    }

    /// Generate Maki configuration file
    pub fn generate_maki_config(&self, output_path: &Path) -> Result<()> {
        let config = MakiConfig::default();
        let json = serde_json::to_string_pretty(&config)?;
        fs::write(output_path, json)?;
        Ok(())
    }

    /// Generate both SUSHI and Maki configs
    pub fn generate_all_configs(
        &self,
        ig: Option<&ImplementationGuide>,
        output_dir: &Path,
    ) -> Result<()> {
        // Create output directory if needed
        fs::create_dir_all(output_dir)?;

        // Generate SUSHI config
        self.generate_sushi_config(ig, &output_dir.join("sushi-config.yaml"))?;

        // Generate Maki config
        self.generate_maki_config(&output_dir.join(".makirc.json"))?;

        Ok(())
    }

    /// Create SUSHI config from ImplementationGuide
    fn sushi_config_from_ig(&self, ig: &ImplementationGuide) -> String {
        let mut yaml = String::new();

        // Basic metadata
        if let Some(id) = &ig.id {
            yaml.push_str(&format!("id: {}\n", id));
        }

        yaml.push_str(&format!("canonical: {}\n", ig.url));
        yaml.push_str(&format!("name: {}\n", ig.name));

        if let Some(title) = &ig.title {
            yaml.push_str(&format!("title: \"{}\"\n", escape_yaml_string(title)));
        }

        if let Some(desc) = &ig.description {
            yaml.push_str(&format!("description: \"{}\"\n", escape_yaml_string(desc)));
        }

        yaml.push_str(&format!("status: {}\n", ig.status));

        if let Some(version) = &ig.version {
            yaml.push_str(&format!("version: {}\n", version));
        }

        // FHIR version
        if let Some(fhir_versions) = &ig.fhir_version {
            if !fhir_versions.is_empty() {
                yaml.push_str("fhirVersion:\n");
                for version in fhir_versions {
                    yaml.push_str(&format!("  - {}\n", version));
                }
            } else {
                yaml.push_str(&format!("fhirVersion: {}\n", self.default_fhir_version));
            }
        } else {
            yaml.push_str(&format!("fhirVersion: {}\n", self.default_fhir_version));
        }

        // Publisher
        if let Some(publisher) = &ig.publisher {
            yaml.push_str(&format!(
                "publisher:\n  name: \"{}\"\n",
                escape_yaml_string(publisher)
            ));
        }

        // Contact
        if let Some(contacts) = &ig.contact
            && !contacts.is_empty()
        {
            yaml.push_str("contact:\n");
            for contact in contacts {
                if let Some(name) = &contact.name {
                    yaml.push_str(&format!("  - name: \"{}\"\n", escape_yaml_string(name)));
                }
                if let Some(telecom_list) = &contact.telecom
                    && !telecom_list.is_empty()
                {
                    yaml.push_str("    telecom:\n");
                    for telecom in telecom_list {
                        if let Some(system) = &telecom.system {
                            yaml.push_str(&format!("      - system: {}\n", system));
                        }
                        if let Some(value) = &telecom.value {
                            yaml.push_str(&format!("        value: {}\n", value));
                        }
                    }
                }
            }
        }

        // License
        if let Some(license) = &ig.license {
            yaml.push_str(&format!("license: {}\n", license));
        }

        // Dependencies
        if let Some(deps) = &ig.depends_on
            && !deps.is_empty()
        {
            yaml.push_str("dependencies:\n");
            for dep in deps {
                if let Some(package_id) = &dep.package_id {
                    yaml.push_str(&format!("  {}:\n", package_id));
                    if let Some(version) = &dep.version {
                        yaml.push_str(&format!("    version: {}\n", version));
                    }
                }
            }
        }

        yaml
    }

    /// Create minimal SUSHI config
    fn minimal_sushi_config(&self) -> String {
        format!(
            r#"# SUSHI Configuration
# Generated by Maki

id: example.fhir.ig
canonical: http://example.org/fhir
name: ExampleIG
title: "Example Implementation Guide"
description: "An example FHIR Implementation Guide"
status: draft
version: 0.1.0
fhirVersion: {}
publisher:
  name: "Example Publisher"
contact:
  - name: "Example Contact"
    telecom:
      - system: email
        value: contact@example.org
"#,
            self.default_fhir_version
        )
    }
}

impl Default for ConfigGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Maki configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakiConfig {
    pub files: FileConfig,
    pub formatter: FormatterConfig,
    pub linter: LinterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    pub exclude: Vec<String>,
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatterConfig {
    pub align_carets: bool,
    pub enabled: bool,
    pub indent_size: u32,
    pub line_width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinterConfig {
    pub enabled: bool,
    pub rule_directories: Vec<String>,
    pub rules: RuleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    pub recommended: bool,
}

impl Default for MakiConfig {
    fn default() -> Self {
        Self {
            files: FileConfig {
                exclude: vec![
                    "node_modules/**".to_string(),
                    "target/**".to_string(),
                    "build/**".to_string(),
                    "fsh-generated/**".to_string(),
                ],
                include: vec!["**/*.fsh".to_string()],
            },
            formatter: FormatterConfig {
                align_carets: true,
                enabled: true,
                indent_size: 2,
                line_width: 100,
            },
            linter: LinterConfig {
                enabled: true,
                rule_directories: vec![],
                rules: RuleConfig { recommended: true },
            },
        }
    }
}

/// Escape special characters in YAML strings
fn escape_yaml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ContactDetail, ContactPoint};
    use tempfile::TempDir;

    #[test]
    fn test_generator_new() {
        let generator = ConfigGenerator::new();
        assert_eq!(generator.default_fhir_version, "4.0.1");
    }

    #[test]
    fn test_generator_with_fhir_version() {
        let generator = ConfigGenerator::with_fhir_version("4.3.0".to_string());
        assert_eq!(generator.default_fhir_version, "4.3.0");
    }

    #[test]
    fn test_minimal_sushi_config() {
        let generator = ConfigGenerator::new();
        let yaml = generator.minimal_sushi_config();

        assert!(yaml.contains("id: example.fhir.ig"));
        assert!(yaml.contains("canonical: http://example.org/fhir"));
        assert!(yaml.contains("name: ExampleIG"));
        assert!(yaml.contains("fhirVersion: 4.0.1"));
        assert!(yaml.contains("status: draft"));
    }

    #[test]
    fn test_sushi_config_from_ig() {
        let generator = ConfigGenerator::new();

        let ig = ImplementationGuide {
            resource_type: Some("ImplementationGuide".to_string()),
            id: Some("my.example.ig".to_string()),
            url: "http://example.org/fhir/ig".to_string(),
            name: "MyIG".to_string(),
            title: Some("My Implementation Guide".to_string()),
            description: Some("An example IG".to_string()),
            status: "active".to_string(),
            version: Some("1.0.0".to_string()),
            fhir_version: Some(vec!["4.0.1".to_string()]),
            publisher: Some("Example Publisher".to_string()),
            contact: Some(vec![ContactDetail {
                name: Some("John Doe".to_string()),
                telecom: Some(vec![ContactPoint {
                    system: Some("email".to_string()),
                    value: Some("john@example.org".to_string()),
                }]),
            }]),
            package_id: Some("my.example.ig".to_string()),
            license: Some("CC0-1.0".to_string()),
            depends_on: None,
            definition: None,
        };

        let yaml = generator.sushi_config_from_ig(&ig);

        assert!(yaml.contains("id: my.example.ig"));
        assert!(yaml.contains("canonical: http://example.org/fhir/ig"));
        assert!(yaml.contains("name: MyIG"));
        assert!(yaml.contains("title: \"My Implementation Guide\""));
        assert!(yaml.contains("status: active"));
        assert!(yaml.contains("version: 1.0.0"));
        assert!(yaml.contains("publisher:"));
        assert!(yaml.contains("name: \"Example Publisher\""));
        assert!(yaml.contains("license: CC0-1.0"));
    }

    #[test]
    fn test_generate_sushi_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("sushi-config.yaml");

        let generator = ConfigGenerator::new();
        generator
            .generate_minimal_sushi_config(&config_path)
            .unwrap();

        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("id: example.fhir.ig"));
        assert!(content.contains("canonical: http://example.org/fhir"));
    }

    #[test]
    fn test_generate_maki_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".makirc.json");

        let generator = ConfigGenerator::new();
        generator.generate_maki_config(&config_path).unwrap();

        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("files"));
        assert!(content.contains("formatter"));
        assert!(content.contains("linter"));
        assert!(content.contains("**/*.fsh"));
    }

    #[test]
    fn test_maki_config_default() {
        let config = MakiConfig::default();

        assert_eq!(config.formatter.indent_size, 2);
        assert_eq!(config.formatter.line_width, 100);
        assert!(config.formatter.enabled);
        assert!(config.linter.enabled);
        assert!(config.linter.rules.recommended);
    }

    #[test]
    fn test_maki_config_serialization() {
        let config = MakiConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();

        assert!(json.contains("alignCarets"));
        assert!(json.contains("indentSize"));
        assert!(json.contains("lineWidth"));
        assert!(json.contains("ruleDirectories"));
    }

    #[test]
    fn test_generate_all_configs() {
        let temp_dir = TempDir::new().unwrap();

        let generator = ConfigGenerator::new();
        generator
            .generate_all_configs(None, temp_dir.path())
            .unwrap();

        let sushi_config = temp_dir.path().join("sushi-config.yaml");
        let maki_config = temp_dir.path().join(".makirc.json");

        assert!(sushi_config.exists());
        assert!(maki_config.exists());

        let sushi_content = fs::read_to_string(&sushi_config).unwrap();
        assert!(sushi_content.contains("canonical:"));

        let maki_content = fs::read_to_string(&maki_config).unwrap();
        assert!(maki_content.contains("formatter"));
    }

    #[test]
    fn test_escape_yaml_string() {
        assert_eq!(escape_yaml_string("hello"), "hello");
        assert_eq!(escape_yaml_string("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(escape_yaml_string("line1\nline2"), "line1\\nline2");
    }
}
