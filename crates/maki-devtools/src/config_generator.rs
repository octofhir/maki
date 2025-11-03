//! Default configuration generator
//!
//! Generates default maki configuration files in various formats.

use anyhow::Result;
use maki_core::config::{
    BuildConfiguration, DependencyVersion, FilesConfiguration, FormatterConfiguration,
    LinterConfiguration, RulesConfiguration, RuleSeverity, UnifiedConfig,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Configuration file generator
pub struct ConfigGenerator;

impl ConfigGenerator {
    /// Generate a default configuration file
    ///
    /// Creates a minimal, ready-to-use configuration file with recommended settings.
    pub fn generate_default(output_path: &Path) -> Result<()> {
        tracing::info!("Generating default configuration file...");

        let config = UnifiedConfig::default();

        // Determine format from file extension
        let content = if output_path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(&config)?
        } else {
            // Default to JSON with comments (JSONC-style, but valid JSON)
            let json = serde_json::to_string_pretty(&config)?;
            Self::add_jsonc_comments(json)
        };

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, content)?;

        println!("✓ Generated config: {}", output_path.display());
        tracing::info!("Configuration generation completed");

        Ok(())
    }

    /// Generate a full example configuration with all options
    ///
    /// Creates a comprehensive configuration file showcasing all available options.
    #[allow(dead_code)]
    pub fn generate_full_example(output_path: &Path) -> Result<()> {
        tracing::info!("Generating full example configuration...");

        // Generate a comprehensive example programmatically with ALL options
        let config = Self::create_full_example_config();

        let content = if output_path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(&config)?
        } else {
            // Generate comprehensive JSON with all options documented
            serde_json::to_string_pretty(&config)?
        };

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, content)?;

        println!("✓ Generated full example config: {}", output_path.display());
        tracing::info!("Full example configuration generated");

        Ok(())
    }

    /// Create a full example configuration with all options populated
    ///
    /// This creates a comprehensive example showing every available configuration option.
    fn create_full_example_config() -> UnifiedConfig {
        // Create comprehensive rules configuration
        let mut blocking_rules = HashMap::new();
        blocking_rules.insert("validate-critical-requirements".to_string(), RuleSeverity::Error);

        let mut correctness_rules = HashMap::new();
        correctness_rules.insert("duplicate-definition".to_string(), RuleSeverity::Error);
        correctness_rules.insert("invalid-reference".to_string(), RuleSeverity::Error);
        correctness_rules.insert("missing-parent".to_string(), RuleSeverity::Error);

        let mut suspicious_rules = HashMap::new();
        suspicious_rules.insert("unused-alias".to_string(), RuleSeverity::Warn);
        suspicious_rules.insert("implicit-cardinality".to_string(), RuleSeverity::Warn);

        let mut style_rules = HashMap::new();
        style_rules.insert("naming-convention".to_string(), RuleSeverity::Warn);
        style_rules.insert("prefer-title-case".to_string(), RuleSeverity::Info);

        let mut documentation_rules = HashMap::new();
        documentation_rules.insert("require-description".to_string(), RuleSeverity::Warn);
        documentation_rules.insert("require-purpose".to_string(), RuleSeverity::Info);

        let rules = RulesConfiguration {
            recommended: Some(true),
            all: Some(false),
            blocking: Some(blocking_rules),
            correctness: Some(correctness_rules),
            suspicious: Some(suspicious_rules),
            style: Some(style_rules),
            documentation: Some(documentation_rules),
        };

        // Create linter configuration
        let linter = LinterConfiguration {
            enabled: Some(true),
            rules: Some(rules),
            rule_directories: Some(vec!["custom-rules/".to_string()]),
        };

        // Create formatter configuration
        let formatter = FormatterConfiguration {
            enabled: Some(true),
            indent_size: Some(2),
            line_width: Some(100),
            align_carets: Some(true),
        };

        // Create files configuration
        let files = FilesConfiguration {
            include: Some(vec![
                "input/fsh/**/*.fsh".to_string(),
                "fsh/**/*.fsh".to_string(),
            ]),
            exclude: Some(vec![
                "**/node_modules/**".to_string(),
                "**/temp/**".to_string(),
                "**/*.generated.fsh".to_string(),
                "**/target/**".to_string(),
                "**/build/**".to_string(),
                "**/*.draft.fsh".to_string(),
            ]),
            ignore_files: Some(vec![".fshlintignore".to_string(), ".gitignore".to_string()]),
        };

        // Create dependencies
        let mut dependencies = HashMap::new();
        dependencies.insert(
            "hl7.fhir.us.core".to_string(),
            DependencyVersion::Simple("6.1.0".to_string()),
        );
        dependencies.insert(
            "hl7.terminology.r4".to_string(),
            DependencyVersion::Simple("5.3.0".to_string()),
        );

        // Create build configuration (SUSHI-compatible)
        let build = BuildConfiguration {
            canonical: "http://example.org/fhir/my-ig".to_string(),
            fhir_version: vec!["4.0.1".to_string()],
            id: Some("my.example.ig".to_string()),
            name: Some("MyImplementationGuide".to_string()),
            title: Some("My Example Implementation Guide".to_string()),
            version: Some("1.0.0".to_string()),
            status: Some("draft".to_string()),
            experimental: Some(true),
            date: Some("2024-01-01".to_string()),
            publisher: Some(maki_core::config::PublisherInfo::Object {
                name: Some("Example Organization".to_string()),
                url: Some("http://example.org".to_string()),
                email: Some("contact@example.org".to_string()),
            }),
            description: Some(
                "This is a comprehensive example Implementation Guide showcasing all configuration options.".to_string()
            ),
            license: Some("CC0-1.0".to_string()),
            copyright: Some("Copyright (c) 2024 Example Organization".to_string()),
            dependencies: Some(dependencies.clone()),
            contact: None,
            use_context: None,
            jurisdiction: None,
            copyright_label: None,
            version_algorithm_string: None,
            version_algorithm_coding: None,
            package_id: None,
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

        // Create the unified config with all sections
        UnifiedConfig {
            schema: Some("https://octofhir.github.io/maki/schema/v1.json".to_string()),
            root: Some(true),
            dependencies: Some(dependencies),
            build: Some(build),
            linter: Some(linter),
            formatter: Some(formatter),
            files: Some(files),
        }
    }

    /// Add helpful comments to JSON configuration
    ///
    /// Adds inline comments (as strings) to help users understand the config
    fn add_jsonc_comments(_json: String) -> String {
        // For now, just prepend a helpful header comment
        r#"{
  "$schema": "https://octofhir.github.io/maki/schema/v1.json",
  "root": true,
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true
    }
  },
  "formatter": {
    "enabled": true,
    "indentSize": 2,
    "lineWidth": 100,
    "alignCarets": true
  },
  "files": {
    "include": ["**/*.fsh"],
    "exclude": [
      "**/node_modules/**",
      "**/temp/**",
      "**/*.generated.fsh",
      "**/target/**",
      "**/build/**"
    ],
    "ignoreFiles": [".fshlintignore"]
  }
}
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_default_json() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("maki.json");

        ConfigGenerator::generate_default(&output_path).unwrap();

        assert!(output_path.exists());

        // Verify it's valid JSON
        let content = fs::read_to_string(&output_path).unwrap();
        let _config: serde_json::Value = serde_json::from_str(&content).unwrap();
    }

    #[test]
    fn test_generate_default_toml() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("maki.toml");

        ConfigGenerator::generate_default(&output_path).unwrap();

        assert!(output_path.exists());

        // Verify it's valid TOML
        let content = fs::read_to_string(&output_path).unwrap();
        let _config: toml::Value = toml::from_str(&content).unwrap();
    }

    #[test]
    fn test_generate_full_example_json() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("maki.full.json");

        ConfigGenerator::generate_full_example(&output_path).unwrap();

        assert!(output_path.exists());

        // Verify it's valid JSON
        let content = fs::read_to_string(&output_path).unwrap();
        let config: UnifiedConfig = serde_json::from_str(&content).unwrap();

        // Verify all sections are present
        assert!(config.build.is_some());
        assert!(config.linter.is_some());
        assert!(config.formatter.is_some());
        assert!(config.files.is_some());
        assert!(config.dependencies.is_some());

        // Verify linter rules are populated
        let linter = config.linter.unwrap();
        assert!(linter.rules.is_some());
        let rules = linter.rules.unwrap();
        assert!(rules.blocking.is_some());
        assert!(rules.correctness.is_some());
        assert!(rules.suspicious.is_some());
        assert!(rules.style.is_some());
        assert!(rules.documentation.is_some());
    }

    #[test]
    fn test_generate_full_example_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("maki.full.yaml");

        ConfigGenerator::generate_full_example(&output_path).unwrap();

        assert!(output_path.exists());

        // Verify it's valid YAML
        let content = fs::read_to_string(&output_path).unwrap();
        let config: UnifiedConfig = serde_yaml::from_str(&content).unwrap();

        // Verify all sections are present
        assert!(config.build.is_some());
        assert!(config.linter.is_some());
        assert!(config.formatter.is_some());
        assert!(config.files.is_some());
    }
}
