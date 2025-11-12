//! GoFSH Command - Convert FHIR resources to FSH
//!
//! This command replicates GoFSH functionality, converting FHIR JSON/XML resources
//! back into FHIR Shorthand (FSH) files. It's the reverse operation of `maki build`.
//!
//! # Features
//!
//! - Load FHIR resources from JSON files
//! - Process StructureDefinitions, ValueSets, CodeSystems, Instances
//! - Extract FSH rules and definitions
//! - Optimize FSH output (remove redundant rules)
//! - Organize FSH files by type or profile
//! - Generate configuration files (sushi-config.yaml, .makirc.json)
//!
//! # Example Usage
//!
//! ```sh
//! # Basic usage - convert current directory
//! maki gofsh .
//!
//! # Specify input and output
//! maki gofsh ./fsh-generated -o ./input/fsh
//!
//! # With dependencies
//! maki gofsh ./resources -d hl7.fhir.us.core@5.0.1 --fhir-version R4
//!
//! # With progress reporting
//! maki gofsh ./resources --progress
//! ```

use colored::Colorize;
use maki_core::{Result, MakiError};
use maki_decompiler::{
    ConfigGenerator, FileLoader, FileOrganizer, FshWriter, LakeStats, LoadStats,
    OrganizationStrategy, ResourceLake, create_lake_with_session, parse_cli_dependencies,
    setup_canonical_environment,
};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, error, info, warn};

/// Execute the gofsh command
///
/// # Arguments
///
/// * `input` - Input directory or file containing FHIR resources
/// * `output` - Output directory for FSH files
/// * `fhir_version` - FHIR version (R4 or R5)
/// * `dependencies` - Package dependencies (e.g., hl7.fhir.us.core@5.0.1)
/// * `strategy` - File organization strategy
/// * `progress` - Show progress during conversion
/// * `indent_size` - Number of spaces for indentation
/// * `line_width` - Maximum line width for formatting
#[allow(clippy::too_many_arguments)]
pub async fn gofsh_command(
    input: PathBuf,
    output: Option<PathBuf>,
    fhir_version: String,
    dependencies: Vec<String>,
    strategy: Option<String>,
    progress: bool,
    indent_size: Option<usize>,
    line_width: Option<usize>,
) -> Result<()> {
    let start_time = Instant::now();

    // Print header
    print_header();

    // Step 1: Validate input path
    if !input.exists() {
        error!("Input path does not exist: {}", input.display());
        return Err(MakiError::ConfigError {
            message: format!("Input path not found: {}", input.display()),
        });
    }

    let output_dir = output.unwrap_or_else(|| PathBuf::from("output"));
    info!("Converting FHIR resources to FSH");
    info!("  Input: {}", input.display());
    info!("  Output: {}", output_dir.display());
    info!("  FHIR Version: {}", fhir_version);

    if progress {
        println!("\n📦 {}", "Step 1: Setting up FHIR packages...".bold());
    }

    // Step 2: Parse dependencies and create ResourceLake
    if progress {
        println!("\n🗂️  {}", "Step 2: Loading FHIR resources...".bold());
    }

    let (release, deps) = if !dependencies.is_empty() || fhir_version != "R4" {
        parse_cli_dependencies(&fhir_version, &dependencies).map_err(|e| {
            MakiError::ConfigError {
                message: format!("Failed to parse dependencies: {}", e),
            }
        })?
    } else {
        parse_cli_dependencies("R4", &[]).map_err(|e| {
            MakiError::ConfigError {
                message: format!("Failed to parse default dependencies: {}", e),
            }
        })?
    };

    let mut lake = create_lake_with_session(release, deps).await.map_err(|e| {
        MakiError::ConfigError {
            message: format!("Failed to create resource lake: {}", e),
        }
    })?;

    // Step 4: Load files into lake
    info!("Loading files from: {}", input.display());
    let mut loader = FileLoader::new();

    let load_result = loader.load_into_lake(&input, &mut lake);

    match load_result {
        Ok(stats) => {
            info!("Loaded {} resources", stats.loaded);
            if progress {
                print_load_stats(&stats);
            }
        }
        Err(e) => {
            error!("Failed to load files: {}", e);
            return Err(MakiError::ConfigError {
                message: format!("Failed to load files: {}", e),
            });
        }
    }

    let lake_stats = lake.stats();

    let total_resources = lake_stats.structure_definitions
        + lake_stats.value_sets
        + lake_stats.code_systems
        + lake_stats.instances;

    if total_resources == 0 {
        println!("\n⚠️  {}", "No FHIR resources found in input directory".yellow());
        println!("   Make sure the input directory contains FHIR JSON files");
        return Ok(());
    }

    if progress {
        println!("\n📊 {}", "Resource Summary:".bold());
        print_lake_stats(&lake_stats);
    }

    // Step 5: Process resources and extract FSH
    if progress {
        println!("\n🔄 {}", "Step 3: Converting to FSH...".bold());
    }

    info!("Processing {} resources", total_resources);

    // TODO: Implement full processing pipeline
    // For now, we'll show what needs to be done:
    // - Process each resource type (Profiles, ValueSets, CodeSystems, Instances)
    // - Extract FSH rules using processor
    // - Apply optimizations to remove redundant rules
    // - Generate exportable objects

    println!("   ⚠️  {}", "Full processing pipeline not yet implemented".yellow());
    println!("   This is a placeholder showing the pipeline structure:");
    println!("   1. ✅ Load FHIR resources → ResourceLake");
    println!("   2. ⏳ Process StructureDefinitions → Profiles/Extensions");
    println!("   3. ⏳ Process ValueSets → FSH ValueSets");
    println!("   4. ⏳ Process CodeSystems → FSH CodeSystems");
    println!("   5. ⏳ Process Instances → FSH Instances");
    println!("   6. ⏳ Optimize FSH rules");
    println!("   7. ⏳ Write FSH files with organizer");
    println!("   8. ⏳ Generate config files");

    // Step 6: Create FSH writer with config
    let writer_indent = indent_size.unwrap_or(2);
    let writer_line_width = line_width.unwrap_or(100);
    let _writer = FshWriter::new(writer_indent, writer_line_width);

    // Step 7: Determine organization strategy
    let org_strategy = match strategy.as_deref() {
        Some("type") => OrganizationStrategy::GroupByFshType,
        Some("profile") => OrganizationStrategy::GroupByProfile,
        Some("single") => OrganizationStrategy::SingleFile,
        Some("file") | None => OrganizationStrategy::FilePerDefinition,
        Some(other) => {
            warn!("Unknown strategy '{}', using 'file'", other);
            OrganizationStrategy::FilePerDefinition
        }
    };

    info!("Organization strategy: {:?}", org_strategy);

    // Step 8: Generate config files
    if progress {
        println!("\n📝 {}", "Step 4: Generating configuration files...".bold());
    }

    let config_generator = ConfigGenerator::new();

    // Check if there's an ImplementationGuide in the lake
    let ig = None; // TODO: Extract IG from lake if present

    let config_result = config_generator.generate_all_configs(ig, &output_dir);

    match config_result {
        Ok(()) => {
            info!("Generated config files in: {}", output_dir.display());
            if progress {
                println!("   ✅ Created sushi-config.yaml");
                println!("   ✅ Created .makirc.json");
            }
        }
        Err(e) => {
            warn!("Failed to generate config files: {}", e);
            if progress {
                println!("   ⚠️  {}", "Config generation failed".yellow());
            }
        }
    }

    // Final summary
    let elapsed = start_time.elapsed();

    if progress {
        println!("\n{}", "═".repeat(60).dimmed());
        println!("\n✨ {}", "GoFSH conversion completed!".green().bold());
        println!("   📁 Output directory: {}", output_dir.display().to_string().cyan());
        println!("   ⏱️  Time: {:.2}s", elapsed.as_secs_f64());
        println!("\n💡 {}", "Next steps:".bold());
        println!("   1. Review generated FSH files in {}", output_dir.display().to_string().cyan());
        println!("   2. Verify sushi-config.yaml settings");
        println!("   3. Run {} to compile back to FHIR", "maki build".bright_blue());
    }

    Ok(())
}

/// Print header
fn print_header() {
    println!("\n╭───────────────────────────────────────────────────────────╮");
    println!(
        "│{}│",
        "          GoFSH - FHIR to FSH Converter             "
            .bright_cyan()
            .bold()
    );
    println!("╰───────────────────────────────────────────────────────────╯\n");
}

/// Print file loading statistics
fn print_load_stats(stats: &LoadStats) {
    println!("   ✅ Loaded {} resources", stats.loaded);

    if stats.errors == 0 {
        println!("      • {} errors", "0".green());
    } else {
        println!("      • {} errors", stats.errors.to_string().red());
        for error in &stats.error_details {
            println!("        - {}: {}", error.file_path.display(), error.error_message.red());
        }
    }
}

/// Print ResourceLake statistics
fn print_lake_stats(stats: &LakeStats) {
    let total = stats.structure_definitions
        + stats.value_sets
        + stats.code_systems
        + stats.instances;
    println!("   📦 Total resources: {}", total.to_string().cyan().bold());

    if stats.structure_definitions > 0 {
        println!("      • {} StructureDefinitions", stats.structure_definitions);
    }
    if stats.value_sets > 0 {
        println!("      • {} ValueSets", stats.value_sets);
    }
    if stats.code_systems > 0 {
        println!("      • {} CodeSystems", stats.code_systems);
    }
    if stats.instances > 0 {
        println!("      • {} Instances", stats.instances);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_gofsh_command_no_input() {
        let result = gofsh_command(
            PathBuf::from("/nonexistent/path"),
            None,
            "R4".to_string(),
            vec![],
            None,
            false,
            None,
            None,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_gofsh_command_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let result = gofsh_command(
            temp_dir.path().to_path_buf(),
            Some(temp_dir.path().join("output")),
            "R4".to_string(),
            vec![],
            None,
            false,
            None,
            None,
        )
        .await;

        // Should succeed but warn about no resources
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_gofsh_command_with_fhir_resource() {
        let temp_dir = TempDir::new().unwrap();

        // Create a sample FHIR resource
        let patient_json = r#"{
            "resourceType": "Patient",
            "id": "example",
            "name": [{
                "family": "Test",
                "given": ["John"]
            }]
        }"#;

        fs::write(temp_dir.path().join("patient.json"), patient_json).unwrap();

        let result = gofsh_command(
            temp_dir.path().to_path_buf(),
            Some(temp_dir.path().join("output")),
            "R4".to_string(),
            vec![],
            None,
            false,
            None,
            None,
        )
        .await;

        assert!(result.is_ok());

        // Verify config files were created
        assert!(temp_dir.path().join("output/sushi-config.yaml").exists());
        assert!(temp_dir.path().join("output/.makirc.json").exists());
    }
}
