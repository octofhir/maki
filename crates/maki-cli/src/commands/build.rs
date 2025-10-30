//! Build command implementation
//!
//! SUSHI-compatible build command for compiling FSH to FHIR resources.

use colored::Colorize;
use maki_core::config::SushiConfiguration;
use maki_core::export::{BuildError, BuildOptions, BuildOrchestrator, BuildStats};
use maki_core::{MakiError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{error, info};

/// Build FSH files to FHIR resources (SUSHI-compatible)
///
/// This command orchestrates the complete FSH → FHIR build pipeline:
/// - Parse FSH files from input/fsh/
/// - Build semantic model
/// - Export all resource types (Profiles, Extensions, Instances, ValueSets, CodeSystems)
/// - Generate ImplementationGuide resource
/// - Write package.json
/// - Load predefined resources
/// - Generate FSH index
pub async fn build_command(
    project_path: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    snapshot: bool,
    preprocessed: bool,
    clean: bool,
    progress: bool,
    config_overrides: HashMap<String, String>,
) -> Result<()> {
    let start_time = Instant::now();

    // Resolve project path
    let project_path = project_path.unwrap_or_else(|| PathBuf::from("."));
    info!("Building FSH project at: {:?}", project_path);

    // Load configuration
    let config = load_configuration(&project_path, &config_overrides)?;

    // Validate configuration
    if let Err(errors) = config.validate() {
        error!("Configuration validation failed:");
        for err in errors {
            error!("  - {}", err);
        }
        return Err(MakiError::ConfigError {
            message: "Invalid configuration".to_string(),
        });
    }

    info!(
        "Configuration loaded: {} v{}",
        config.name.as_deref().unwrap_or("Unknown"),
        config.version.as_deref().unwrap_or("0.1.0")
    );
    info!("Canonical: {}", config.canonical);
    info!("FHIR Version: {}", config.fhir_version.join(", "));

    // Determine input and output directories
    let input_dir = project_path.join("input").join("fsh");
    let output_dir = if let Some(out) = output_dir {
        out
    } else {
        project_path.join("fsh-generated")
    };

    // Create build options
    let options = BuildOptions {
        input_dir,
        output_dir: output_dir.clone(),
        generate_snapshots: snapshot,
        write_preprocessed: preprocessed,
        clean_output: clean,
        show_progress: progress,
        fhir_version: None,
        config_overrides,
    };

    // Print build info
    print_build_header(&config, &options);

    // Create orchestrator and run build
    let orchestrator = BuildOrchestrator::new(config.clone(), options);
    let result = orchestrator.build().await.map_err(|e| MakiError::ConfigError {
        message: format!("Build failed: {}", e),
    })?;

    let elapsed = start_time.elapsed();

    // Print results
    print_build_results(&result.stats, elapsed);

    // Exit with error code if there were errors
    if result.stats.has_errors() {
        std::process::exit(result.stats.errors.min(255) as i32);
    }

    Ok(())
}

/// Load configuration from project directory
fn load_configuration(
    project_path: &Path,
    overrides: &HashMap<String, String>,
) -> Result<SushiConfiguration> {
    // Try to load sushi-config.yaml or sushi-config.yml
    let yaml_path = project_path.join("sushi-config.yaml");
    let yml_path = project_path.join("sushi-config.yml");

    let config_path = if yaml_path.exists() {
        yaml_path
    } else if yml_path.exists() {
        yml_path
    } else {
        return Err(MakiError::ConfigError {
            message: "No sushi-config.yaml found in project directory".to_string(),
        });
    };

    info!("Loading configuration from: {:?}", config_path);

    let content = std::fs::read_to_string(&config_path).map_err(|e| MakiError::ConfigError {
        message: format!("Failed to read configuration file: {}", e),
    })?;

    let mut config: SushiConfiguration =
        serde_yaml::from_str(&content).map_err(|e| MakiError::ConfigError {
            message: format!("Failed to parse configuration: {}", e),
        })?;

    // Apply overrides
    apply_config_overrides(&mut config, overrides);

    Ok(config)
}

/// Apply configuration overrides from CLI
fn apply_config_overrides(config: &mut SushiConfiguration, overrides: &HashMap<String, String>) {
    for (key, value) in overrides {
        match key.as_str() {
            "version" => config.version = Some(value.clone()),
            "status" => config.status = Some(value.clone()),
            _ => {
                tracing::warn!("Unknown config override: {}", key);
            }
        }
    }
}

/// Print build header with configuration info
fn print_build_header(config: &SushiConfiguration, options: &BuildOptions) {
    println!();
    println!(
        "{}",
        "╔════════════════════════════════════════════════════════════════╗"
            .bright_cyan()
    );
    println!(
        "{}",
        "║                    MAKI Build Pipeline                         ║"
            .bright_cyan()
    );
    println!(
        "{}",
        "╚════════════════════════════════════════════════════════════════╝"
            .bright_cyan()
    );
    println!();
    println!(
        "  {} {}",
        "Project:".bold(),
        config.name.as_deref().unwrap_or("Unknown")
    );
    println!(
        "  {} {}",
        "Version:".bold(),
        config.version.as_deref().unwrap_or("0.1.0")
    );
    println!(
        "  {} {}",
        "Status:".bold(),
        config.status.as_deref().unwrap_or("draft")
    );
    println!("  {} {}", "Canonical:".bold(), config.canonical);
    println!(
        "  {} {}",
        "FHIR Version:".bold(),
        config.fhir_version.join(", ")
    );
    println!();
    println!("  {} {:?}", "Input:".bold(), options.input_dir);
    println!("  {} {:?}", "Output:".bold(), options.output_dir);

    if options.generate_snapshots {
        println!("  {} Enabled", "Snapshots:".bold());
    }
    if options.write_preprocessed {
        println!("  {} Enabled", "Preprocessed FSH:".bold());
    }
    if options.clean_output {
        println!("  {} Enabled", "Clean Output:".bold());
    }

    println!();
    println!("{}", "Starting build...".bright_blue());
    println!();
}

/// Print build results in SUSHI-compatible format
fn print_build_results(stats: &BuildStats, elapsed: std::time::Duration) {
    println!();

    let color = if stats.has_errors() {
        colored::Color::Red
    } else if stats.has_warnings() {
        colored::Color::Yellow
    } else {
        colored::Color::Green
    };

    println!(
        "{}",
        "╔════════════════════════════════════════════════════════════════╗"
            .color(color)
    );
    println!(
        "{}",
        "║                        BUILD RESULTS                           ║"
            .color(color)
    );
    println!(
        "{}",
        "╠════════════════════════════════════════════════════════════════╣"
            .color(color)
    );

    // Resource counts
    println!(
        "{} │ {:^14} │ {:^14} │ {:^14} │ {}",
        "║".color(color),
        "Profiles",
        "Extensions",
        "Logicals",
        "║".color(color)
    );
    println!(
        "{} │ {:^14} │ {:^14} │ {:^14} │ {}",
        "║".color(color),
        stats.profiles,
        stats.extensions,
        stats.logicals,
        "║".color(color)
    );
    println!(
        "{}",
        "╠════════════════════════════════════════════════════════════════╣"
            .color(color)
    );

    println!(
        "{} │ {:^14} │ {:^14} │ {:^14} │ {}",
        "║".color(color),
        "ValueSets",
        "CodeSystems",
        "Instances",
        "║".color(color)
    );
    println!(
        "{} │ {:^14} │ {:^14} │ {:^14} │ {}",
        "║".color(color),
        stats.value_sets,
        stats.code_systems,
        stats.instances,
        "║".color(color)
    );
    println!(
        "{}",
        "╠════════════════════════════════════════════════════════════════╣"
            .color(color)
    );

    // Summary
    let total = stats.total_resources();
    let summary = format!(
        "{} resources generated in {:.2}s",
        total,
        elapsed.as_secs_f64()
    );
    let errors_msg = if stats.errors > 0 {
        format!(
            "{} error{}",
            stats.errors,
            if stats.errors != 1 { "s" } else { "" }
        )
        .red()
        .to_string()
    } else {
        "0 errors".green().to_string()
    };
    let warnings_msg = if stats.warnings > 0 {
        format!(
            "{} warning{}",
            stats.warnings,
            if stats.warnings != 1 { "s" } else { "" }
        )
        .yellow()
        .to_string()
    } else {
        "0 warnings".green().to_string()
    };

    println!(
        "{} {:^62} {}",
        "║".color(color),
        summary,
        "║".color(color)
    );
    println!(
        "{} {:^62} {}",
        "║".color(color),
        format!("{} | {}", errors_msg, warnings_msg),
        "║".color(color)
    );
    println!(
        "{}",
        "╚════════════════════════════════════════════════════════════════╝"
            .color(color)
    );

    if stats.has_errors() {
        println!();
        println!("{}", "Build failed with errors!".red().bold());
    } else if stats.has_warnings() {
        println!();
        println!("{}", "Build completed with warnings.".yellow());
    } else {
        println!();
        println!("{}", "Build completed successfully!".green().bold());
        println!();
        println!(
            "{}",
            "Ready for IG Publisher. Run ./_genonce.sh to publish."
                .bright_blue()
        );
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_apply_config_overrides() {
        let mut config = SushiConfiguration {
            id: Some("test.ig".to_string()),
            canonical: "http://example.org/fhir/test".to_string(),
            name: Some("TestIG".to_string()),
            status: Some("draft".to_string()),
            version: Some("1.0.0".to_string()),
            fhir_version: vec!["4.0.1".to_string()],
            ..Default::default()
        };

        let mut overrides = HashMap::new();
        overrides.insert("version".to_string(), "2.0.0".to_string());
        overrides.insert("status".to_string(), "active".to_string());

        apply_config_overrides(&mut config, &overrides);

        assert_eq!(config.version, Some("2.0.0".to_string()));
        assert_eq!(config.status, Some("active".to_string()));
    }

    #[test]
    fn test_load_configuration_missing() {
        let temp_dir = TempDir::new().unwrap();
        let overrides = HashMap::new();

        let result = load_configuration(temp_dir.path(), &overrides);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_configuration_yaml() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal sushi-config.yaml
        let config_content = r#"
id: test.ig
canonical: http://example.org/fhir/test
name: TestIG
status: draft
version: 1.0.0
fhirVersion: 4.0.1
"#;
        std::fs::write(temp_dir.path().join("sushi-config.yaml"), config_content).unwrap();

        let overrides = HashMap::new();
        let config = load_configuration(temp_dir.path(), &overrides).unwrap();

        assert_eq!(config.id, Some("test.ig".to_string()));
        assert_eq!(config.canonical, "http://example.org/fhir/test");
        assert_eq!(config.version, Some("1.0.0".to_string()));
    }
}
