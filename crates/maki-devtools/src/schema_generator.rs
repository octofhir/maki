//! JSON Schema generator for maki configuration
//!
//! Generates JSON Schema from the Rust configuration types using schemars.

use anyhow::Result;
use maki_core::config::MakiConfiguration;
use schemars::schema_for;
use serde_json::json;
use std::fs;
use std::path::Path;

/// Schema generator for configuration files
pub struct SchemaGenerator;

impl SchemaGenerator {
    /// Generate JSON Schema and write to file
    ///
    /// Generates a complete JSON Schema from the `MakiConfiguration` type
    /// with metadata and writes it to the specified path.
    pub fn generate(output_path: &Path) -> Result<()> {
        tracing::info!("Generating JSON Schema for maki configuration...");

        // Generate schema from Rust types using schemars
        let schema = schema_for!(MakiConfiguration);

        // Convert to JSON value for manipulation
        let mut schema_json = serde_json::to_value(schema)?;

        // Add metadata
        schema_json["$id"] = json!("https://octofhir.github.io/maki-rs/schema/v1.json");
        schema_json["$schema"] = json!("http://json-schema.org/draft-07/schema#");
        schema_json["title"] = json!("FSH Lint Configuration");
        schema_json["description"] =
            json!("Configuration file schema for maki - validates maki.json and maki.jsonc files");

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to file with pretty formatting
        let output = serde_json::to_string_pretty(&schema_json)?;
        fs::write(output_path, output)?;

        println!("✓ Generated JSON Schema: {}", output_path.display());
        tracing::info!("Schema generation completed successfully");

        Ok(())
    }

    /// Validate that the schema can be generated without errors
    ///
    /// Useful for CI/CD to ensure schema generation doesn't break
    pub fn validate() -> Result<()> {
        tracing::info!("Validating schema generation...");

        let schema = schema_for!(MakiConfiguration);
        let _schema_json = serde_json::to_value(schema)?;

        println!("✓ Schema validation passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_schema_generation() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("schema.json");

        SchemaGenerator::generate(&output_path).unwrap();

        assert!(output_path.exists());

        // Verify it's valid JSON
        let content = fs::read_to_string(&output_path).unwrap();
        let schema: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify metadata
        assert_eq!(schema["title"], "FSH Lint Configuration");
        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    }

    #[test]
    fn test_schema_validation() {
        SchemaGenerator::validate().unwrap();
    }
}
