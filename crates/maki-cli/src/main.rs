//! MAKI CLI
//!
//! Command-line interface for the MAKI FSH tooling suite

mod commands; // Contains current commands: lint, format, rules, config
mod output;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use maki_core::{Result, init_tracing};
use std::io;
use std::path::PathBuf;
use tracing::error;

#[derive(Parser)]
#[command(name = "maki")]
#[command(about = "MAKI: High-performance FSH CLI for linting, formatting, and building")]
#[command(version = maki_core::VERSION)]
#[command(
    long_about = "MAKI is a fast, extensible toolkit for FHIR Shorthand (FSH) projects.\n\
It provides comprehensive linting, formatting, and (soon) build capabilities for Implementation Guides.\n\
\n\
Examples:\n  \
maki lint                    # Lint current directory\n  \
maki lint --fix src/         # Lint and fix files in src/\n  \
maki fmt --check .           # Check formatting without changes\n  \
maki rules --verbose         # List all available rules\n  \
maki config init             # Initialize configuration file"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Configuration file path
    #[arg(
        short,
        long,
        global = true,
        help = "Path to configuration file (.makirc.json/.makirc.toml)"
    )]
    config: Option<PathBuf>,

    /// Verbose output (can be used multiple times for increased verbosity)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    /// Number of threads to use for parallel processing
    #[arg(
        short = 'j',
        long,
        global = true,
        help = "Number of threads (default: number of CPU cores)"
    )]
    threads: Option<usize>,

    /// Generate shell completion script
    #[arg(
        long,
        value_enum,
        help = "Generate completion script for specified shell"
    )]
    generate_completion: Option<Shell>,
}

#[derive(Subcommand)]
enum Commands {
    /// Build FSH project to FHIR resources (SUSHI-compatible)
    Build {
        /// Path to FSH project directory
        #[arg(help = "Path to FSH project (default: current directory)")]
        project_path: Option<PathBuf>,

        /// Output directory for generated resources
        #[arg(short, long, help = "Output directory (default: fsh-generated)")]
        output: Option<PathBuf>,

        /// Generate snapshots in StructureDefinitions
        #[arg(short, long, help = "Generate snapshots in StructureDefinitions")]
        snapshot: bool,

        /// Output preprocessed FSH for debugging
        #[arg(short, long, help = "Output preprocessed FSH")]
        preprocessed: bool,

        /// Clean output directory before building
        #[arg(long, help = "Clean output directory before building")]
        clean: bool,

        /// Show progress during build
        #[arg(long, help = "Show progress bar during build")]
        progress: bool,

        /// Run linter during build for real-time feedback
        #[arg(long, help = "Run linter during build (default: false)")]
        lint: bool,

        /// Strict mode: treat warnings as errors
        #[arg(long, help = "Treat warnings as errors (requires --lint)")]
        strict: bool,

        /// Format FSH files before building
        #[arg(long, help = "Auto-format FSH files before build (default: false)")]
        format: bool,

        /// Disable incremental compilation cache
        #[arg(long, help = "Disable build cache (default: cache enabled)")]
        no_cache: bool,

        /// Skip dependency installation (workaround for timeout issues)
        #[arg(long, help = "Skip installing FHIR package dependencies")]
        skip_deps: bool,

        /// Override configuration values (e.g., --config version:2.0.0)
        #[arg(
            short = 'c',
            long,
            help = "Override config values (version, status, releaselabel)",
            value_parser = parse_config_override
        )]
        config: Vec<(String, String)>,
    },

    /// Initialize a new FHIR Implementation Guide project
    Init {
        /// Project name
        #[arg(help = "Name of the project (default: MyIG)")]
        name: Option<String>,

        /// Use default values for all prompts (non-interactive)
        #[arg(long, help = "Use default values without prompting")]
        default: bool,
    },

    /// Lint FSH files for syntax errors, semantic issues, and best practice violations
    #[command(alias = "check")]
    Lint {
        /// Files or directories to lint
        #[arg(help = "Files or directories to process (default: current directory)")]
        paths: Vec<PathBuf>,

        /// Output format
        #[arg(
            short,
            long,
            default_value = "human",
            help = "Output format for diagnostics"
        )]
        format: OutputFormat,

        /// Write fixes to files (applies safe fixes by default)
        #[arg(long, help = "Write fixes to files")]
        write: bool,

        /// Show fixes without applying them
        #[arg(
            long,
            help = "Show proposed fixes without applying them (dry run)",
            conflicts_with = "write"
        )]
        dry_run: bool,

        /// Apply unsafe fixes (use with --write)
        #[arg(long, help = "Apply unsafe fixes (requires --write, use with caution)")]
        r#unsafe: bool,

        /// Interactive mode - confirm each unsafe fix
        #[arg(long, short = 'i', help = "Interactively confirm each unsafe fix")]
        interactive: bool,

        /// Minimum severity level to report
        #[arg(
            long,
            default_value = "warning",
            help = "Minimum severity level to report"
        )]
        min_severity: Severity,

        /// Include/exclude patterns (glob syntax)
        #[arg(
            long,
            help = "Include files matching pattern (can be used multiple times)"
        )]
        include: Vec<String>,

        /// Exclude patterns (glob syntax)
        #[arg(
            long,
            help = "Exclude files matching pattern (can be used multiple times)"
        )]
        exclude: Vec<String>,

        /// Exit with non-zero code only on errors (ignore warnings)
        #[arg(long, help = "Exit with non-zero code only on errors")]
        error_on_warnings: bool,

        /// Show progress for long-running operations
        #[arg(long, help = "Show progress bar for large projects")]
        progress: bool,
    },

    /// Format FSH files according to style guidelines
    #[command(alias = "format")]
    /// Format and fix FSH files
    Fmt {
        /// Files or directories to format
        #[arg(help = "Files or directories to format (default: current directory)")]
        paths: Vec<PathBuf>,

        /// Output format
        #[arg(
            short,
            long,
            default_value = "human",
            help = "Output format for diagnostics"
        )]
        format: OutputFormat,

        /// Write fixes to files (default behavior for format command)
        #[arg(long, help = "Write fixes to files")]
        write: bool,

        /// Check formatting without modifying files
        #[arg(
            long,
            help = "Check if files are formatted correctly without modifying them",
            conflicts_with = "write"
        )]
        check: bool,

        /// Show diff of proposed changes without applying them (dry run)
        #[arg(long, help = "Show diff of proposed formatting changes")]
        diff: bool,

        /// Apply unsafe fixes (use with --write)
        #[arg(long, help = "Apply unsafe fixes (use with caution)")]
        r#unsafe: bool,

        /// Include/exclude patterns (glob syntax)
        #[arg(
            long,
            help = "Include files matching pattern (can be used multiple times)"
        )]
        include: Vec<String>,

        /// Exclude patterns (glob syntax)
        #[arg(
            long,
            help = "Exclude files matching pattern (can be used multiple times)"
        )]
        exclude: Vec<String>,

        /// Line width for formatting
        #[arg(long, help = "Maximum line width for formatting")]
        line_width: Option<usize>,

        /// Indentation size
        #[arg(long, help = "Number of spaces for indentation")]
        indent_size: Option<usize>,
    },

    /// Manage and inspect linting rules
    Rules {
        #[command(subcommand)]
        action: Option<RulesAction>,

        /// Show detailed rule information
        #[arg(long, help = "Show detailed information for each rule")]
        detailed: bool,

        /// Filter rules by category
        #[arg(
            long,
            help = "Filter rules by category (syntax, semantic, style, etc.)"
        )]
        category: Option<String>,

        /// Filter rules by tag
        #[arg(long, help = "Filter rules by tag")]
        tag: Option<String>,
    },

    /// Configuration file management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show version information
    #[command(alias = "ver")]
    Version {
        /// Show detailed version information
        #[arg(long, help = "Show detailed version and build information")]
        detailed: bool,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Initialize a new configuration file
    Init {
        /// Configuration file format
        #[arg(long, default_value = "json", help = "Configuration file format")]
        format: ConfigFormat,

        /// Overwrite existing configuration file
        #[arg(long, help = "Overwrite existing configuration file")]
        force: bool,

        /// Include example rules and settings
        #[arg(long, help = "Include example rules and settings")]
        with_examples: bool,
    },

    /// Migrate SUSHI config to MAKI unified format
    Migrate {
        /// Output file path (default: maki.yaml)
        #[arg(short, long, help = "Output file path for migrated config")]
        output: Option<PathBuf>,

        /// Skip confirmation prompts
        #[arg(short, long, help = "Skip confirmation prompts")]
        yes: bool,
    },

    /// Validate configuration file
    Validate {
        /// Path to configuration file to validate
        #[arg(help = "Path to configuration file (default: search for .makirc)")]
        path: Option<PathBuf>,
    },

    /// Show current configuration
    Show {
        /// Show resolved configuration (after inheritance and merging)
        #[arg(long, help = "Show resolved configuration after inheritance")]
        resolved: bool,
    },
}

#[derive(Subcommand)]
enum RulesAction {
    /// List all available rules
    List,

    /// Show detailed information about a specific rule
    Explain {
        /// Rule ID to explain
        #[arg(help = "Rule ID to show detailed information for")]
        rule_id: String,
    },

    /// Search rules by name or description
    Search {
        /// Search query
        #[arg(help = "Search query for rule names or descriptions")]
        query: String,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    /// Human-readable output with colors and context
    Human,
    /// JSON format for programmatic consumption
    Json,
    /// SARIF format for CI/CD integration
    Sarif,
    /// Compact human-readable format
    Compact,
    /// GitHub Actions format
    Github,
}

#[derive(ValueEnum, Clone, Debug)]
enum Severity {
    /// Only show errors
    Error,
    /// Show warnings and errors
    Warning,
    /// Show info, warnings, and errors
    Info,
    /// Show all diagnostics including hints
    Hint,
}

#[derive(ValueEnum, Clone, Debug)]
enum ConfigFormat {
    /// JSON configuration format
    Json,
    /// TOML configuration format
    Toml,
}

/// Parse config override in the format key:value
fn parse_config_override(s: &str) -> std::result::Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid config override format '{}'. Expected 'key:value'",
            s
        ));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn main() -> Result<()> {
    // Configure tokio runtime with enough threads to handle concurrent database operations
    // Each concurrent export may spawn blocking tasks for DB queries (via tokio-rusqlite)
    // With 4 concurrent exports × 3-4 DB queries each = ~16 blocking operations
    // So we need: worker_threads (CPU-bound) + max_blocking_threads (I/O-bound)
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get()) // CPU-bound async tasks
        .max_blocking_threads(512) // I/O-bound blocking operations (DB queries) - high to prevent deadlock
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    runtime.block_on(async_main())
}

async fn async_main() -> Result<()> {
    let cli = Cli::parse();

    // Handle shell completion generation
    if let Some(shell) = cli.generate_completion {
        generate_completion_script(shell);
        return Ok(());
    }

    // Initialize colored output
    if !cli.no_color && std::env::var("NO_COLOR").is_err() {
        colored::control::set_override(true);
    } else {
        colored::control::set_override(false);
    }

    // Initialize tracing based on verbosity
    let log_level = match cli.verbose {
        0 => "maki=error", // Only errors by default
        1 => "maki=warn",  // Warnings on first -v
        2 => "maki=info",  // Info on -vv
        3 => "maki=debug", // Debug on -vvv
        _ => "maki=trace", // Trace on -vvvv+
    };
    unsafe {
        std::env::set_var("RUST_LOG", log_level);
    }
    init_tracing();

    // Set thread pool size if specified
    if let Some(threads) = cli.threads
        && let Err(e) = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
    {
        error!("Failed to set thread pool size: {}", e);
        std::process::exit(1);
    }

    match run_command(cli).await {
        Ok(()) => Ok(()),
        Err(e) => {
            error!("FSH Lint failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn generate_completion_script(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}

async fn run_command(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Build {
            project_path,
            output,
            snapshot,
            preprocessed,
            clean,
            progress,
            lint,
            strict,
            format,
            no_cache,
            skip_deps,
            config,
        }) => {
            let config_overrides: std::collections::HashMap<String, String> =
                config.into_iter().collect();
            commands::build::build_command(
                project_path,
                output,
                snapshot,
                preprocessed,
                clean,
                progress,
                lint,
                strict,
                format,
                no_cache,
                skip_deps,
                config_overrides,
            )
            .await
        }

        Some(Commands::Init { name, default }) => commands::init::init_command(name, default).await,

        Some(Commands::Lint {
            paths,
            format,
            write,
            dry_run,
            r#unsafe,
            interactive,
            min_severity,
            include,
            exclude,
            error_on_warnings,
            progress,
        }) => {
            let paths = if paths.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                paths
            };
            commands::lint_command(
                paths,
                format,
                write,
                dry_run,
                r#unsafe,
                interactive,
                min_severity,
                include,
                exclude,
                error_on_warnings,
                progress,
                cli.config,
            )
            .await
        }

        Some(Commands::Fmt {
            paths,
            format,
            write,
            check,
            diff,
            r#unsafe,
            include,
            exclude,
            line_width,
            indent_size,
        }) => {
            let paths = if paths.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                paths
            };

            commands::format_command(
                paths,
                format,
                write,
                check,
                diff,
                r#unsafe,
                include,
                exclude,
                line_width,
                indent_size,
                cli.config,
            )
            .await
        }

        Some(Commands::Rules {
            action,
            detailed,
            category,
            tag,
        }) => match action {
            Some(RulesAction::List) | None => {
                commands::rules_list_command(detailed, category, tag, cli.config).await
            }
            Some(RulesAction::Explain { rule_id }) => {
                commands::rules_explain_command(rule_id, cli.config).await
            }
            Some(RulesAction::Search { query }) => {
                commands::rules_search_command(query, cli.config).await
            }
        },

        Some(Commands::Config { action }) => match action {
            ConfigAction::Init {
                format,
                force,
                with_examples,
            } => commands::config_init_command(format, force, with_examples).await,
            ConfigAction::Migrate { output, yes } => {
                commands::config::migrate_command(yes, output).await
            }
            ConfigAction::Validate { path } => commands::config_validate_command(path).await,
            ConfigAction::Show { resolved } => {
                commands::config_show_command(resolved, cli.config).await
            }
        },

        Some(Commands::Version { detailed }) => {
            if detailed {
                println!("maki {}", maki_core::VERSION);
                println!("Build information:");
                println!("  Target: {}", std::env::consts::ARCH);
                println!("  OS: {}", std::env::consts::OS);
                println!(
                    "  Rust version: {}",
                    env!("CARGO_PKG_RUST_VERSION", "unknown")
                );
                if let Ok(profile) = std::env::var("PROFILE") {
                    println!("  Profile: {profile}");
                }
            } else {
                println!("{}", maki_core::VERSION);
            }
            Ok(())
        }

        None => {
            // No subcommand provided, show help
            let mut cmd = Cli::command();
            cmd.print_help()?;
            Ok(())
        }
    }
}
