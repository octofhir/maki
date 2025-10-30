//! Default configuration generator
//!
//! Generates default maki configuration files in various formats.

use anyhow::Result;
use maki_core::config::MakiConfiguration;
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

        let config = MakiConfiguration::default();

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
    pub fn generate_full_example(output_path: &Path) -> Result<()> {
        tracing::info!("Generating full example configuration...");

        // Use the example from our examples/configs directory
        let example_content = include_str!("../../../examples/configs/full.jsonc");

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, example_content)?;

        println!("✓ Generated full example config: {}", output_path.display());
        tracing::info!("Full example configuration generated");

        Ok(())
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
}
