//! Configuration migration command
//!
//! Migrates SUSHI's sushi-config.yaml to MAKI's unified maki.yaml format

use colored::Colorize;
use maki_core::config::{SushiConfiguration, UnifiedConfig};
use maki_core::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Run the config migration command
///
/// Dependencies are always lifted to the top-level since they're shared
/// between build and linter sections.
pub async fn migrate_command(
    auto_yes: bool,
    output: Option<PathBuf>,
) -> Result<()> {
    print_header();

    // 1. Detect existing configs
    println!("📋 {}", "Detecting configuration files...".bold());

    let sushi_path = detect_sushi_config()?;

    if let Some(ref path) = sushi_path {
        println!("   ✓ {} (SUSHI format)", path.display().to_string().green());
    } else {
        println!("   ✗ {}", "No sushi-config.yaml found".red());
        println!("\n{}", "Migration cancelled: No SUSHI configuration file found.".yellow());
        println!("Run {} to create a new MAKI configuration.", "maki config init".bright_blue());
        return Ok(());
    }

    // 2. Explain migration
    println!("\n🔄 {}", "Migration Plan:".bold());
    println!("   • Read SUSHI config → map to build section");
    println!("   • Lift dependencies to top-level (shared across build/linter)");
    println!("   • Add default linter and formatter sections");
    println!("   • Generate unified maki.yaml");

    println!("\n⚠️  {}", "The new config uses a different structure:".yellow());
    println!("   {}: Top-level SUSHI fields", "Old (SUSHI)".dimmed());
    println!("   {}: Structured sections (build, linter, formatter)", "New (MAKI)".green());

    // 3. Confirm
    if !auto_yes {
        print!("\n{} ", "? Proceed with migration? (Y/n)".bold());
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if !input.is_empty() && input != "y" && input != "yes" {
            println!("\n❌ Migration cancelled");
            return Ok(());
        }
    }

    // 4. Load and convert
    println!("\n⏳ {}", "Migrating configuration...".bright_blue());

    let sushi_path = sushi_path.unwrap();
    let unified = build_unified_config(&sushi_path)?;

    // 5. Write new config
    let target = output.unwrap_or_else(|| PathBuf::from("maki.yaml"));

    // Create backup
    info!("Creating backup of {}", sushi_path.display());
    create_backup(&sushi_path)?;

    // Write unified config
    info!("Writing unified config to {}", target.display());
    let yaml = serde_yaml::to_string(&unified)
        .map_err(|e| maki_core::MakiError::config_error(
            format!("Failed to serialize config: {}", e)
        ))?;
    fs::write(&target, yaml)?;

    // 6. Success message
    print_success(&target, &sushi_path);

    Ok(())
}

/// Print the header
fn print_header() {
    println!("\n╭───────────────────────────────────────────────────────────╮");
    println!("│{}│", "          MAKI Config Migration                    ".bright_cyan().bold());
    println!("╰───────────────────────────────────────────────────────────╯\n");
}

/// Print success message
fn print_success(target: &Path, sushi_path: &Path) {
    println!("\n✅ {}\n", "Configuration migrated successfully!".green().bold());

    println!("📁 {}:", "Files".bold());
    println!("   {}: {}", "Created".green(), target.display());
    println!("   {}: {}.backup", "Backup".dimmed(), sushi_path.display());

    println!("\n💡 {}:", "Next steps".bold());
    println!("   1. Review {} to verify settings", target.display().to_string().bright_blue());
    println!("   2. Update any CI/CD scripts to use new config");
    println!("   3. Test your build: {}", "maki build".bright_blue());
    println!("   4. Delete backup when ready: {}", format!("rm {}.backup", sushi_path.display()).dimmed());

    println!("\n📖 Documentation: {}", "https://octofhir.github.io/maki/config".bright_blue().underline());
}

/// Build unified config from SUSHI config
///
/// Always lifts dependencies to top-level since they're shared between
/// build and linter sections.
fn build_unified_config(sushi_path: &Path) -> Result<UnifiedConfig> {
    info!("Loading SUSHI config from {}", sushi_path.display());

    // Load SUSHI config
    let content = fs::read_to_string(sushi_path)?;
    let mut sushi_config: SushiConfiguration = serde_yaml::from_str(&content)
        .map_err(|e| maki_core::MakiError::config_error(
            format!("Failed to parse sushi-config.yaml: {}", e)
        ))?;

    // Validate SUSHI config
    if let Err(errors) = sushi_config.validate() {
        warn!("SUSHI config has validation errors: {:?}", errors);
    }

    // Lift dependencies to top-level (always, since they're shared)
    let top_level_deps = sushi_config.dependencies.clone();

    // Remove dependencies from build section since they're now at top-level
    sushi_config.dependencies = None;

    if let Some(ref deps) = top_level_deps {
        info!("Lifted {} dependencies to top-level", deps.len());
    }

    // Create unified config with dependencies at top-level
    let unified = UnifiedConfig {
        dependencies: top_level_deps,
        build: Some(sushi_config),
        linter: Some(maki_core::config::LinterConfiguration::default()),
        formatter: Some(maki_core::config::FormatterConfiguration::default()),
        files: Some(maki_core::config::FilesConfiguration::default()),
    };

    info!("Built unified config successfully");
    Ok(unified)
}

/// Create backup of a file
fn create_backup(path: &Path) -> Result<()> {
    let backup = PathBuf::from(format!("{}.backup", path.display()));
    fs::copy(path, &backup)?;
    info!("Created backup: {}", backup.display());
    Ok(())
}

/// Detect sushi-config.yaml in current directory
fn detect_sushi_config() -> Result<Option<PathBuf>> {
    let paths = ["sushi-config.yaml", "sushi-config.yml"];
    for path in &paths {
        let path_buf = PathBuf::from(path);
        if path_buf.exists() {
            return Ok(Some(path_buf));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_sushi_config() {
        // This would need test fixtures
        let result = detect_sushi_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_unified_config() {
        // Create a temporary SUSHI config for testing
        let temp_dir = std::env::temp_dir();
        let test_config = temp_dir.join("test-sushi-config.yaml");

        let sushi_yaml = r#"
canonical: http://example.org/fhir/test-ig
fhirVersion: 4.0.1
id: test.ig
name: TestIG
title: Test Implementation Guide
status: draft
version: 1.0.0
"#;

        fs::write(&test_config, sushi_yaml).unwrap();

        let unified = build_unified_config(&test_config).unwrap();

        assert!(unified.build.is_some());
        assert_eq!(unified.build.as_ref().unwrap().canonical, "http://example.org/fhir/test-ig");
        assert!(unified.linter.is_some());
        assert!(unified.formatter.is_some());

        // Cleanup
        fs::remove_file(&test_config).ok();
    }
}
