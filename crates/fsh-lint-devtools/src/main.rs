//! FSH Lint Developer Tools
//!
//! Command-line tools for fsh-lint developers:
//! - Generate JSON Schema for configuration files
//! - Generate default configuration files
//! - Generate rule documentation
//! - Validate configuration setup

mod config_generator;
mod rule_doc_generator;
mod schema_generator;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fsh-lint-devtools")]
#[command(about = "Developer tools for fsh-lint", version)]
#[command(author = "OctoFHIR Team")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate JSON Schema for configuration file
    GenerateSchema {
        /// Output path for schema file
        #[arg(short, long, default_value = "docs/public/schema/v1.json")]
        output: PathBuf,
    },

    /// Generate a default configuration file
    GenerateConfig {
        /// Output path for config file
        #[arg(short, long, default_value = "fsh-lint.json")]
        output: PathBuf,

        /// Generate full example with all options
        #[arg(short, long)]
        full: bool,
    },

    /// Generate rule documentation
    GenerateRuleDocs {
        /// Output directory for rule docs
        #[arg(short, long, default_value = "docs/src/content/docs/rules")]
        output: PathBuf,
    },

    /// Validate schema generation (CI/CD helper)
    Validate,

    /// Generate all artifacts (schema + example configs + rule docs)
    GenerateAll {
        /// Base directory for output
        #[arg(short, long, default_value = "docs")]
        output_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity
    let subscriber = tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false);

    if cli.verbose {
        subscriber.with_max_level(tracing::Level::DEBUG).init();
    } else {
        subscriber.with_max_level(tracing::Level::INFO).init();
    }

    match cli.command {
        Commands::GenerateSchema { output } => {
            schema_generator::SchemaGenerator::generate(&output)?;
        }

        Commands::GenerateConfig { output, full } => {
            if full {
                config_generator::ConfigGenerator::generate_full_example(&output)?;
            } else {
                config_generator::ConfigGenerator::generate_default(&output)?;
            }
        }

        Commands::GenerateRuleDocs { output } => {
            rule_doc_generator::RuleDocGenerator::generate(&output)?;
        }

        Commands::Validate => {
            println!("Validating schema generation...");
            schema_generator::SchemaGenerator::validate()?;
            println!("✓ All validations passed");
        }

        Commands::GenerateAll { output_dir } => {
            println!("🔨 Generating all development artifacts...\n");

            // Generate JSON Schema
            let schema_path = output_dir.join("public/schema/v1.json");
            println!("📋 Generating JSON Schema...");
            schema_generator::SchemaGenerator::generate(&schema_path)?;

            // Generate example configs
            let examples_dir = output_dir.join("examples");
            println!("\n📝 Generating example configurations...");

            let minimal_path = examples_dir.join("minimal.json");
            config_generator::ConfigGenerator::generate_default(&minimal_path)?;

            let full_path = examples_dir.join("full.jsonc");
            config_generator::ConfigGenerator::generate_full_example(&full_path)?;

            // Generate rule documentation
            let rules_dir = output_dir.join("src/content/docs/rules");
            println!("\n📚 Generating rule documentation...");
            rule_doc_generator::RuleDocGenerator::generate(&rules_dir)?;

            println!("\n✅ All artifacts generated successfully!");
            println!("\nGenerated files:");
            println!("  📋 Schema:        {}", schema_path.display());
            println!("  📝 Minimal config: {}", minimal_path.display());
            println!("  📝 Full example:   {}", full_path.display());
            println!("  📚 Rule docs:      {}", rules_dir.display());
        }
    }

    Ok(())
}
