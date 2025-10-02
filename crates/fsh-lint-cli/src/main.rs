//! FSH Lint CLI
//! 
//! Command-line interface for the FSH linter

use clap::{Parser, Subcommand};
use fsh_lint_core::{init_tracing, Result};
use std::path::PathBuf;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "fsh-lint")]
#[command(about = "A high-performance linter for FHIR Shorthand (FSH) files")]
#[command(version = fsh_lint_core::VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,
    
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint FSH files
    Lint {
        /// Files or directories to lint
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        
        /// Output format
        #[arg(short, long, default_value = "human")]
        format: OutputFormat,
        
        /// Apply safe automatic fixes
        #[arg(long)]
        fix: bool,
        
        /// Show fixes without applying them
        #[arg(long)]
        fix_dry_run: bool,
    },
    
    /// Format FSH files
    Fmt {
        /// Files or directories to format
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        
        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
        
        /// Show diff of formatting changes
        #[arg(long)]
        diff: bool,
    },
    
    /// List available rules
    Rules {
        /// Show detailed rule information
        #[arg(long)]
        verbose: bool,
    },
    
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Initialize a new configuration file
    Init,
    /// Validate configuration
    Validate,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Human,
    Json,
    Sarif,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize tracing based on verbosity
    if cli.verbose {
        std::env::set_var("RUST_LOG", "fsh_lint=debug");
    }
    init_tracing();
    
    info!("Starting FSH Lint v{}", fsh_lint_core::VERSION);
    
    match run_command(cli).await {
        Ok(()) => {
            info!("FSH Lint completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("FSH Lint failed: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Lint { paths, format, fix, fix_dry_run } => {
            info!("Linting paths: {:?}", paths);
            // TODO: Implement linting logic in later tasks
            println!("Linting functionality will be implemented in task 12.2");
            Ok(())
        }
        
        Commands::Fmt { paths, check, diff } => {
            info!("Formatting paths: {:?}", paths);
            // TODO: Implement formatting logic in later tasks
            println!("Formatting functionality will be implemented in task 11");
            Ok(())
        }
        
        Commands::Rules { verbose } => {
            info!("Listing rules");
            // TODO: Implement rules listing in later tasks
            println!("Rules listing will be implemented in task 12.2");
            Ok(())
        }
        
        Commands::Config { action } => {
            match action {
                ConfigAction::Init => {
                    info!("Initializing configuration");
                    // TODO: Implement config initialization in later tasks
                    println!("Config initialization will be implemented in task 12.2");
                    Ok(())
                }
                ConfigAction::Validate => {
                    info!("Validating configuration");
                    // TODO: Implement config validation in later tasks
                    println!("Config validation will be implemented in task 12.2");
                    Ok(())
                }
            }
        }
    }
}