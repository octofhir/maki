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

use crate::output::IndicatifProgressReporter;
use colored::Colorize;
use maki_core::{MakiError, Result};
use maki_decompiler::{
    ConfigGenerator, FileLoader, FshWriter, GoFshSummary, LakeStats, LoadStats,
    OrganizationStrategy, ProcessingStats, WriteStats, create_lake_with_session,
    parse_cli_dependencies,
};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{error, info, warn};

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

    // Phase 1: Setup canonical environment
    let setup_progress = IndicatifProgressReporter::new_spinner(
        progress,
        "{spinner:.green} Setting up FHIR packages...",
    );
    setup_progress.set_message("Setting up FHIR packages...");

    // Step 2: Parse dependencies and create ResourceLake

    let (release, deps) = if !dependencies.is_empty() || fhir_version != "R4" {
        parse_cli_dependencies(&fhir_version, &dependencies).map_err(|e| {
            MakiError::ConfigError {
                message: format!("Failed to parse dependencies: {}", e),
            }
        })?
    } else {
        parse_cli_dependencies("R4", &[]).map_err(|e| MakiError::ConfigError {
            message: format!("Failed to parse default dependencies: {}", e),
        })?
    };

    let mut lake =
        create_lake_with_session(release, deps)
            .await
            .map_err(|e| MakiError::ConfigError {
                message: format!("Failed to create resource lake: {}", e),
            })?;

    setup_progress.finish_with_message("✓ FHIR packages ready");

    // Phase 2: Load FHIR resources
    let load_progress = IndicatifProgressReporter::new_spinner(
        progress,
        "{spinner:.green} Loading FHIR resources...",
    );

    info!("Loading files from: {}", input.display());
    let mut loader = FileLoader::new();
    let load_stats = loader.load_into_lake(&input, &mut lake).map_err(|e| {
        error!("Failed to load files: {}", e);
        MakiError::ConfigError {
            message: format!("Failed to load files: {}", e),
        }
    })?;

    load_progress.finish_with_message(format!("✓ Loaded {} resources", load_stats.loaded));

    // Display detailed load stats if errors occurred
    if progress && load_stats.errors > 0 {
        print_load_errors(&load_stats);
    }

    let lake_stats = lake.stats();
    let total_resources = lake_stats.structure_definitions
        + lake_stats.value_sets
        + lake_stats.code_systems
        + lake_stats.instances;

    let has_resources = total_resources > 0;

    if progress {
        print_lake_summary(&lake_stats);
    }

    // Phase 3: Process resources and extract FSH
    let mut processing_stats = ProcessingStats::new();
    let mut exportables: Vec<Box<dyn maki_decompiler::Exportable>> = Vec::new();

    if !has_resources {
        if progress {
            println!(
                "\n⚠️  {}",
                "No FHIR resources found in input directory".yellow()
            );
            println!("   Skipping processing and writing phases");
            println!("   Config files will still be generated");
        }
    } else {
        // Create processor instances
        use maki_decompiler::{
            CodeSystemProcessor, InstanceProcessor, StructureDefinitionProcessor, ValueSetProcessor,
        };

        let sd_processor = StructureDefinitionProcessor::new(&lake);
        let vs_processor = ValueSetProcessor::new(&lake);
        let cs_processor = CodeSystemProcessor::new(&lake);
        let inst_processor = InstanceProcessor::new(&lake);

        // Process StructureDefinitions with progress
        if lake_stats.structure_definitions > 0 {
            let sd_progress = IndicatifProgressReporter::new(
                progress,
                lake_stats.structure_definitions as u64,
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} Processing StructureDefinitions...",
            );

            for (url, sd) in lake.structure_definitions() {
                match sd_processor.process(sd).await {
                    Ok(exportable) => {
                        exportables.push(exportable);
                        processing_stats.profiles_processed += 1; // TODO: distinguish profiles/extensions/resources/logicals
                    }
                    Err(e) => {
                        warn!("Failed to process StructureDefinition {}: {}", url, e);
                        processing_stats.errors += 1;
                    }
                }
                sd_progress.inc();
            }
            sd_progress.finish_with_message(format!(
                "✓ Processed {} StructureDefinitions",
                lake_stats.structure_definitions
            ));
        }

        // Process ValueSets with progress
        if lake_stats.value_sets > 0 {
            let vs_progress = IndicatifProgressReporter::new(
                progress,
                lake_stats.value_sets as u64,
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} Processing ValueSets...",
            );

            for (url, vs) in lake.value_sets() {
                match vs_processor.process(vs) {
                    Ok(exportable) => {
                        exportables.push(Box::new(exportable));
                        processing_stats.value_sets_processed += 1;
                    }
                    Err(e) => {
                        warn!("Failed to process ValueSet {}: {}", url, e);
                        processing_stats.errors += 1;
                    }
                }
                vs_progress.inc();
            }
            vs_progress
                .finish_with_message(format!("✓ Processed {} ValueSets", lake_stats.value_sets));
        }

        // Process CodeSystems with progress
        if lake_stats.code_systems > 0 {
            let cs_progress = IndicatifProgressReporter::new(
                progress,
                lake_stats.code_systems as u64,
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} Processing CodeSystems...",
            );

            for (url, cs) in lake.code_systems() {
                match cs_processor.process(cs) {
                    Ok(exportable) => {
                        exportables.push(Box::new(exportable));
                        processing_stats.code_systems_processed += 1;
                    }
                    Err(e) => {
                        warn!("Failed to process CodeSystem {}: {}", url, e);
                        processing_stats.errors += 1;
                    }
                }
                cs_progress.inc();
            }
            cs_progress.finish_with_message(format!(
                "✓ Processed {} CodeSystems",
                lake_stats.code_systems
            ));
        }

        // Process Instances with progress
        if lake_stats.instances > 0 {
            let inst_progress = IndicatifProgressReporter::new(
                progress,
                lake_stats.instances as u64,
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} Processing Instances...",
            );

            for (_id, resource) in lake.instances() {
                match inst_processor.process(resource) {
                    Ok(exportable) => {
                        exportables.push(Box::new(exportable));
                        processing_stats.instances_processed += 1;
                    }
                    Err(e) => {
                        warn!("Failed to process Instance: {}", e);
                        processing_stats.errors += 1;
                    }
                }
                inst_progress.inc();
            }
            inst_progress
                .finish_with_message(format!("✓ Processed {} Instances", lake_stats.instances));
        }

        info!(
            "Processed {} resources with {} errors",
            processing_stats.total_processed(),
            processing_stats.errors
        );
    }

    // Phase 4: Optimize FSH rules
    if !exportables.is_empty() {
        let opt_progress = IndicatifProgressReporter::new_spinner(
            progress,
            "{spinner:.green} Optimizing FSH rules...",
        );

        use maki_decompiler::{
            AddReferenceKeywordOptimizer, CombineAssignmentsOptimizer,
            CombineCardAndFlagRulesOptimizer, CombineContainsRulesOptimizer, OptimizerRegistry,
            RemoveChoiceSlicingRulesOptimizer, RemoveDuplicateRulesOptimizer,
            RemoveExtensionURLAssignmentOptimizer, RemoveGeneratedTextRulesOptimizer,
            RemoveImpliedCardinalityOptimizer, RemoveZeroZeroCardRulesOptimizer,
            SimplifyArrayIndexingOptimizer, SimplifyCardinalityOptimizer,
        };

        // Create optimizer registry and register all plugins
        let mut registry = OptimizerRegistry::new();
        registry.add(Box::new(RemoveDuplicateRulesOptimizer));
        registry.add(Box::new(CombineAssignmentsOptimizer));
        registry.add(Box::new(SimplifyCardinalityOptimizer));
        registry.add(Box::new(AddReferenceKeywordOptimizer));
        registry.add(Box::new(CombineCardAndFlagRulesOptimizer));
        registry.add(Box::new(RemoveZeroZeroCardRulesOptimizer));
        registry.add(Box::new(CombineContainsRulesOptimizer));
        registry.add(Box::new(RemoveGeneratedTextRulesOptimizer));
        registry.add(Box::new(RemoveExtensionURLAssignmentOptimizer));
        registry.add(Box::new(SimplifyArrayIndexingOptimizer));
        registry.add(Box::new(RemoveChoiceSlicingRulesOptimizer));
        registry.add(Box::new(RemoveImpliedCardinalityOptimizer));

        // Apply optimizations to all exportables
        for exportable in exportables.iter_mut() {
            match registry.optimize_all(exportable.as_mut(), &lake) {
                Ok(stats) => {
                    processing_stats.optimization_stats.merge(&stats);
                }
                Err(e) => {
                    warn!("Failed to optimize {}: {}", exportable.name(), e);
                    processing_stats.errors += 1;
                }
            }
        }

        if progress {
            if processing_stats.optimization_stats.has_changes() {
                opt_progress.finish_with_message(format!(
                    "✓ Optimized {} exportables ({})",
                    exportables.len(),
                    processing_stats.optimization_stats
                ));
            } else {
                opt_progress.finish_with_message("✓ No optimizations needed");
            }
        }
    }

    let mut write_stats = WriteStats::new();

    // Phase 5: Determine organization strategy
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

    // Phase 6: Write FSH files
    if !exportables.is_empty() {
        let write_progress = IndicatifProgressReporter::new_spinner(
            progress,
            "{spinner:.green} Writing FSH files...",
        );

        let writer_indent = indent_size.unwrap_or(2);
        let writer_line_width = line_width.unwrap_or(100);
        let writer = FshWriter::new(writer_indent, writer_line_width);

        use maki_decompiler::FileOrganizer;
        let organizer = FileOrganizer::with_writer(org_strategy, writer);

        match organizer.organize(&exportables, &output_dir) {
            Ok(()) => {
                // TODO: Track actual file count and bytes written
                write_stats.files_written = exportables.len();
                write_progress
                    .finish_with_message(format!("✓ Wrote {} FSH files", exportables.len()));
            }
            Err(e) => {
                write_progress.finish_with_message("⚠ FSH writing failed");
                warn!("Failed to write FSH files: {}", e);
                write_stats.errors += 1;
            }
        }
    }

    // Phase 7: Generate config files
    let config_progress = IndicatifProgressReporter::new_spinner(
        progress,
        "{spinner:.green} Generating configuration files...",
    );

    let config_generator = ConfigGenerator::new();
    let ig = None; // TODO: Extract IG from lake if present
    let config_generated = config_generator
        .generate_all_configs(ig, &output_dir)
        .is_ok();

    if config_generated {
        config_progress.finish_with_message("✓ Configuration files generated");
    } else {
        config_progress.finish_with_message("⚠ Config generation failed");
    }

    // Final summary
    let duration = start_time.elapsed();

    if progress {
        let summary = GoFshSummary::new(
            load_stats,
            processing_stats,
            write_stats,
            duration,
            config_generated,
        );
        print_final_summary(&summary, &output_dir);
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

/// Print load errors if any occurred
fn print_load_errors(stats: &LoadStats) {
    if stats.errors > 0 {
        println!("\n⚠️  {} load errors:", stats.errors.to_string().yellow());
        for error in &stats.error_details {
            println!(
                "   • {}: {}",
                error.file_path.display().to_string().dimmed(),
                error.error_message.red()
            );
        }
    }
}

/// Print ResourceLake summary
fn print_lake_summary(stats: &LakeStats) {
    let total =
        stats.structure_definitions + stats.value_sets + stats.code_systems + stats.instances;

    println!("\n📊 {}", "Resource Summary:".bold());
    println!(
        "   {} {}",
        "Total:".dimmed(),
        total.to_string().cyan().bold()
    );

    if stats.structure_definitions > 0 {
        println!(
            "   {} {} StructureDefinitions",
            "•".cyan(),
            stats.structure_definitions.to_string().bold()
        );
    }
    if stats.value_sets > 0 {
        println!(
            "   {} {} ValueSets",
            "•".cyan(),
            stats.value_sets.to_string().bold()
        );
    }
    if stats.code_systems > 0 {
        println!(
            "   {} {} CodeSystems",
            "•".cyan(),
            stats.code_systems.to_string().bold()
        );
    }
    if stats.instances > 0 {
        println!(
            "   {} {} Instances",
            "•".cyan(),
            stats.instances.to_string().bold()
        );
    }
}

/// Print final summary with all statistics
fn print_final_summary(summary: &GoFshSummary, output_dir: &Path) {
    println!("\n{}", "═".repeat(60).dimmed());

    if summary.is_success() {
        println!("\n✨ {}", "GoFSH conversion completed!".green().bold());
    } else {
        println!(
            "\n⚠️  {}",
            "GoFSH conversion completed with errors".yellow().bold()
        );
    }

    println!("\n📦 {}", "Summary:".bold());
    println!(
        "   {} {} resources loaded",
        "•".cyan(),
        summary.load_stats.loaded
    );

    if summary.processing_stats.has_processed() {
        println!(
            "   {} {} resources processed",
            "•".cyan(),
            summary.processing_stats.total_processed()
        );
        if summary.processing_stats.rules_extracted > 0 {
            println!(
                "   {} {} FSH rules extracted",
                "•".cyan(),
                summary.processing_stats.rules_extracted
            );
        }
    }

    if summary.write_stats.has_written() {
        println!(
            "   {} {} FSH files written ({})",
            "•".cyan(),
            summary.write_stats.files_written,
            summary.write_stats.human_size()
        );
    }

    if summary.config_generated {
        println!("   {} Configuration files generated", "•".cyan());
    }

    if summary.total_errors() > 0 {
        println!(
            "\n⚠️  {} total errors",
            summary.total_errors().to_string().red()
        );
    }

    println!(
        "\n⏱️  {} {:.2}s",
        "Time:".dimmed(),
        summary.duration.as_secs_f64()
    );
    println!(
        "📁 {} {}",
        "Output:".dimmed(),
        output_dir.display().to_string().cyan()
    );

    println!("\n💡 {}", "Next steps:".bold());
    println!(
        "   1. Review generated FSH files in {}",
        output_dir.display().to_string().cyan()
    );
    println!("   2. Verify sushi-config.yaml settings");
    println!(
        "   3. Run {} to compile back to FHIR",
        "maki build".bright_blue()
    );

    println!("\n{}", "═".repeat(60).dimmed());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

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
    #[serial]
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
    #[serial]
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

        if let Err(e) = &result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok(), "Expected command to succeed but got error");

        // Verify config files were created
        let sushi_config = temp_dir.path().join("output/sushi-config.yaml");
        let makirc = temp_dir.path().join("output/.makirc.json");

        assert!(
            sushi_config.exists(),
            "sushi-config.yaml not found at {:?}",
            sushi_config
        );
        assert!(makirc.exists(), ".makirc.json not found at {:?}", makirc);
    }
}
