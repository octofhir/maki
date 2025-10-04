//! CLI command implementations
//!
//! This module contains the implementation of all CLI commands

use fsh_lint_core::{
    CachedFshParser, DefaultExecutor, DefaultFileDiscovery, DefaultSemanticAnalyzer,
    ExecutionContext, Executor, FileDiscovery, Result, RuleEngine,
    config::{ConfigManager, DefaultConfigManager},
};
use fsh_lint_rules::{BuiltinRules, DefaultRuleEngine, init_builtin_rules};
use serde_json;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, error, info};

use crate::output::{LintSummary, OutputFormatter, ProgressReporter};
use crate::{ConfigFormat, OutputFormat, Severity};

/// Lint command implementation
pub async fn lint_command(
    paths: Vec<PathBuf>,
    format: OutputFormat,
    fix: bool,
    fix_dry_run: bool,
    fix_unsafe: bool,
    min_severity: Severity,
    include: Vec<String>,
    exclude: Vec<String>,
    error_on_warnings: bool,
    progress: bool,
    config_path: Option<PathBuf>,
) -> Result<()> {
    debug!("Running lint command on paths: {:?}", paths);

    // Load configuration
    let config_manager = DefaultConfigManager::new();
    let mut config = config_manager.load_config(config_path.as_ref().map(|p| p.as_path()))?;

    // Apply CLI overrides to configuration
    if !include.is_empty() {
        config.include_patterns = include.clone();
    }
    if !exclude.is_empty() {
        config.exclude_patterns = exclude.clone();
    }

    debug!("Loaded configuration with {} rules", config.rules.len());

    let start_time = Instant::now();

    // Determine which files to lint
    let fsh_files = if paths.is_empty() {
        // No paths specified - discover files based on config patterns
        let file_discovery = DefaultFileDiscovery::new(std::env::current_dir()?);
        file_discovery.discover_files(&config)?
    } else {
        // Paths specified - use them directly (respecting globs)
        let mut files = Vec::new();
        for path in paths {
            if path.is_file() {
                // Direct file path
                if path.extension().map_or(false, |ext| ext == "fsh") {
                    files.push(path);
                }
            } else if path.is_dir() {
                // Directory - find all .fsh files in it
                let file_discovery = DefaultFileDiscovery::new(&path);
                files.extend(file_discovery.discover_files(&config)?);
            } else if path.to_string_lossy().contains('*') {
                // Glob pattern - expand it
                let pattern_str = path.to_string_lossy();
                for entry in glob::glob(&pattern_str).map_err(|e| fsh_lint_core::FshLintError::IoError {
                    path: path.clone(),
                    source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
                })? {
                    match entry {
                        Ok(p) if p.extension().map_or(false, |ext| ext == "fsh") => files.push(p),
                        _ => {}
                    }
                }
            } else {
                // Path doesn't exist - try as glob pattern anyway
                let pattern_str = path.to_string_lossy();
                let mut found = false;
                for entry in glob::glob(&pattern_str).map_err(|e| fsh_lint_core::FshLintError::IoError {
                    path: path.clone(),
                    source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
                })? {
                    match entry {
                        Ok(p) if p.extension().map_or(false, |ext| ext == "fsh") => {
                            files.push(p);
                            found = true;
                        }
                        _ => {}
                    }
                }
                if !found {
                    error!("Path does not exist: {}", path.display());
                }
            }
        }
        files
    };

    if fsh_files.is_empty() {
        println!("No FSH files found in specified paths.");
        return Ok(());
    }

    debug!("Found {} FSH files to lint", fsh_files.len());

    // Initialize progress reporter
    let mut progress_reporter = ProgressReporter::new(progress, fsh_files.len());

    // Create rule engine
    let mut rule_engine = DefaultRuleEngine::new();

    // Collect and compile all built-in rules
    let all_builtin_rules = BuiltinRules::all_rules();
    let mut compiled_rules = Vec::new();

    for rule in all_builtin_rules {
        match rule_engine.compile_rule(&rule) {
            Ok(compiled_rule) => {
                rule_engine.registry_mut().register(compiled_rule.clone());
                compiled_rules.push(compiled_rule);
            }
            Err(e) => {
                error!("Failed to compile rule {}: {}", rule.id, e);
            }
        }
    }

    debug!("Loaded {} rules for execution", compiled_rules.len());

    // Create parser, semantic analyzer, and executor
    let parser = Box::new(CachedFshParser::new()?);
    let semantic_analyzer = Box::new(DefaultSemanticAnalyzer::new());

    // Create execution context
    let context = ExecutionContext::new(config.clone(), compiled_rules);

    // Create executor
    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, Box::new(rule_engine));

    // Execute linting in parallel
    if progress {
        progress_reporter.update(0, "Starting linting...");
    }

    let results = executor.execute_parallel(fsh_files)?;

    if progress {
        progress_reporter.finish("Linting");
    }

    // Collect diagnostics and build summary
    let mut all_diagnostics = Vec::new();
    let mut summary = LintSummary::new();
    summary.files_checked = results.len();

    for result in results {
        if let Some(error) = result.error {
            error!("Error processing {}: {}", result.file_path.display(), error);
            summary.errors += 1;
        } else {
            for diagnostic in &result.diagnostics {
                match diagnostic.severity {
                    fsh_lint_core::Severity::Error => summary.errors += 1,
                    fsh_lint_core::Severity::Warning => summary.warnings += 1,
                    fsh_lint_core::Severity::Info => summary.info += 1,
                    fsh_lint_core::Severity::Hint => summary.hints += 1,
                }
            }
            all_diagnostics.extend(result.diagnostics);
        }
    }

    // Format and print results
    let use_colors = !std::env::var("NO_COLOR").is_ok() && atty::is(atty::Stream::Stdout);
    let formatter = OutputFormatter::new(format, use_colors);
    formatter.print_results(&all_diagnostics, &summary, progress)?;

    let duration = start_time.elapsed();
    if progress {
        println!(
            "Completed in {}",
            crate::output::utils::format_duration(duration)
        );
    }

    // Determine exit code
    let has_errors = summary.errors > 0;
    let has_warnings = summary.warnings > 0;

    if has_errors || (error_on_warnings && has_warnings) {
        std::process::exit(1);
    }

    Ok(())
}

/// Rules list command implementation
pub async fn rules_list_command(
    detailed: bool,
    category: Option<String>,
    tag: Option<String>,
    config_path: Option<PathBuf>,
) -> Result<()> {
    debug!("Listing available rules");

    // Load configuration to get rule settings
    let config_manager = DefaultConfigManager::new();
    let config = config_manager.load_config(config_path.as_ref().map(|p| p.as_path()))?;

    // Collect all built-in rules
    let all_rules = BuiltinRules::all_rules();

    println!("Available Rules:");
    println!("================");

    let mut count = 0;
    for rule in all_rules {
        let rule_category = match &rule.metadata.category {
            fsh_lint_core::RuleCategory::Correctness => "correctness",
            fsh_lint_core::RuleCategory::Suspicious => "suspicious",
            fsh_lint_core::RuleCategory::Style => "style",
            fsh_lint_core::RuleCategory::Complexity => "complexity",
            fsh_lint_core::RuleCategory::Documentation => "documentation",
            fsh_lint_core::RuleCategory::Performance => "performance",
            fsh_lint_core::RuleCategory::Nursery => "nursery",
            fsh_lint_core::RuleCategory::Accessibility => "accessibility",
            fsh_lint_core::RuleCategory::Security => "security",
            fsh_lint_core::RuleCategory::Custom(s) => s.as_str(),
        };

        // Apply category filter
        if let Some(ref filter_cat) = category {
            if rule_category != filter_cat.as_str() {
                continue;
            }
        }

        // Apply tag filter (if tags exist)
        if let Some(ref filter_tag) = tag {
            // Skip if no tags match
            continue;
        }

        count += 1;

        if detailed {
            println!("\n{}", rule.id);
            println!("  Description: {}", rule.metadata.description);
            println!("  Category: {}", rule_category);
            println!(
                "  Status: {}",
                if config.rules.contains_key(&rule.id) {
                    "configured"
                } else {
                    "default"
                }
            );
            if rule.autofix.is_some() {
                println!("  Autofix: available");
            }
        } else {
            println!("  {} - {}", rule.id, rule.metadata.description);
        }
    }

    if count == 0 {
        println!("\nNo rules found matching the specified filters.");
    } else {
        println!("\nTotal: {} rules", count);
    }

    Ok(())
}

/// Rules explain command implementation
pub async fn rules_explain_command(rule_id: String, _config_path: Option<PathBuf>) -> Result<()> {
    debug!("Explaining rule: {}", rule_id);

    // Collect all built-in rules
    let all_rules = BuiltinRules::all_rules();

    // Find the rule
    let rule = all_rules.iter().find(|r| r.id == rule_id);

    match rule {
        Some(rule) => {
            let category = match &rule.metadata.category {
                fsh_lint_core::RuleCategory::Correctness => "Correctness",
                fsh_lint_core::RuleCategory::Suspicious => "Suspicious",
                fsh_lint_core::RuleCategory::Style => "Style",
                fsh_lint_core::RuleCategory::Complexity => "Complexity",
                fsh_lint_core::RuleCategory::Documentation => "Documentation",
                fsh_lint_core::RuleCategory::Performance => "Performance",
                fsh_lint_core::RuleCategory::Nursery => "Nursery",
                fsh_lint_core::RuleCategory::Accessibility => "Accessibility",
                fsh_lint_core::RuleCategory::Security => "Security",
                fsh_lint_core::RuleCategory::Custom(s) => s.as_str(),
            };

            println!("Rule: {}", rule.id);
            println!("{}", "=".repeat(rule.id.len() + 6));
            println!();
            println!("Category: {}", category);
            println!("Description: {}", rule.metadata.description);

            if let Some(autofix) = &rule.autofix {
                println!();
                println!("Autofix available: {}", autofix.description);
                println!("Safety: {:?}", autofix.safety);
            }

            println!();
            println!("GritQL Pattern:");
            println!("{}", rule.gritql_pattern);
        }
        None => {
            println!("Rule '{}' not found.", rule_id);
            println!();
            println!("Use 'fsh-lint rules' to list all available rules.");
        }
    }

    Ok(())
}

/// Rules search command implementation
pub async fn rules_search_command(query: String, _config_path: Option<PathBuf>) -> Result<()> {
    debug!("Searching rules for: {}", query);

    // Collect all built-in rules
    let all_rules = BuiltinRules::all_rules();

    let query_lower = query.to_lowercase();

    // Search in rule IDs and descriptions
    let matches: Vec<_> = all_rules
        .into_iter()
        .filter(|rule| {
            rule.id.to_lowercase().contains(&query_lower)
                || rule
                    .metadata
                    .description
                    .to_lowercase()
                    .contains(&query_lower)
        })
        .collect();

    if matches.is_empty() {
        println!("No rules found matching '{}'", query);
    } else {
        println!("Rules matching '{}':", query);
        println!("{}", "=".repeat(20 + query.len()));
        println!();
        for rule in matches {
            let category = match &rule.metadata.category {
                fsh_lint_core::RuleCategory::Correctness => "correctness",
                fsh_lint_core::RuleCategory::Suspicious => "suspicious",
                fsh_lint_core::RuleCategory::Style => "style",
                fsh_lint_core::RuleCategory::Complexity => "complexity",
                fsh_lint_core::RuleCategory::Documentation => "documentation",
                fsh_lint_core::RuleCategory::Performance => "performance",
                fsh_lint_core::RuleCategory::Nursery => "nursery",
                fsh_lint_core::RuleCategory::Accessibility => "accessibility",
                fsh_lint_core::RuleCategory::Security => "security",
                fsh_lint_core::RuleCategory::Custom(s) => s.as_str(),
            };
            println!(
                "  {} - {} ({})",
                rule.id, rule.metadata.description, category
            );
        }
    }

    Ok(())
}

/// Config init command implementation
pub async fn config_init_command(
    format: ConfigFormat,
    force: bool,
    with_examples: bool,
) -> Result<()> {
    debug!("Initializing configuration file with format: {:?}", format);

    let filename = match format {
        ConfigFormat::Json => ".fshlintrc.json",
        ConfigFormat::Toml => ".fshlintrc.toml",
    };

    let config_path = PathBuf::from(filename);

    // Check if file already exists
    if config_path.exists() && !force {
        error!(
            "Configuration file '{}' already exists. Use --force to overwrite.",
            filename
        );
        return Err(fsh_lint_core::FshLintError::ConfigError {
            message: format!("Configuration file '{}' already exists", filename),
        });
    }

    // Create default configuration
    let default_config = if with_examples {
        create_example_config()
    } else {
        create_minimal_config()
    };

    // Write configuration file
    let config_content = match format {
        ConfigFormat::Json => serde_json::to_string_pretty(&default_config).map_err(|e| {
            fsh_lint_core::FshLintError::ConfigError {
                message: format!("Failed to serialize JSON: {}", e),
            }
        })?,
        ConfigFormat::Toml => toml::to_string_pretty(&default_config).map_err(|e| {
            fsh_lint_core::FshLintError::ConfigError {
                message: format!("Failed to serialize TOML: {}", e),
            }
        })?,
    };

    std::fs::write(&config_path, config_content)?;

    println!("✅ Created configuration file: {}", filename);
    if with_examples {
        println!("   The file includes example rules and settings.");
    }
    println!("   Edit the file to customize your linting rules.");

    Ok(())
}

/// Config validate command implementation
pub async fn config_validate_command(path: Option<PathBuf>) -> Result<()> {
    debug!("Validating configuration file: {:?}", path);

    let config_manager = DefaultConfigManager::new();

    match config_manager.load_config(path.as_ref().map(|p| p.as_path())) {
        Ok(config) => {
            println!("✅ Configuration is valid");
            println!("   Rules configured: {}", config.rules.len());
            println!("   Include patterns: {}", config.include_patterns.len());
            println!("   Exclude patterns: {}", config.exclude_patterns.len());
        }
        Err(e) => {
            error!("❌ Configuration validation failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Config show command implementation
pub async fn config_show_command(resolved: bool, config_path: Option<PathBuf>) -> Result<()> {
    debug!("Showing configuration (resolved: {})", resolved);

    let config_manager = DefaultConfigManager::new();
    let config = config_manager.load_config(config_path.as_ref().map(|p| p.as_path()))?;

    if resolved {
        println!("Resolved Configuration:");
        println!("======================");
    } else {
        println!("Configuration:");
        println!("==============");
    }

    let config_json = serde_json::to_string_pretty(&config).map_err(|e| {
        fsh_lint_core::FshLintError::ConfigError {
            message: format!("Failed to serialize config: {}", e),
        }
    })?;
    println!("{}", config_json);

    Ok(())
}

/// Create a minimal default configuration
fn create_minimal_config() -> serde_json::Value {
    serde_json::json!({
        "include": ["**/*.fsh"],
        "exclude": ["node_modules/**", "target/**"],
        "rules": {},
        "formatter": {
            "indent_size": 2,
            "max_line_width": 100,
            "caret_alignment": "consistent"
        }
    })
}

/// Create an example configuration with sample rules
fn create_example_config() -> serde_json::Value {
    serde_json::json!({
        "include": ["**/*.fsh"],
        "exclude": ["node_modules/**", "target/**", "build/**"],
        "rules": {
            "builtin/correctness/invalid-keyword": "error",
            "builtin/correctness/invalid-constraint": "error",
            "builtin/suspicious/trailing-text": "warning",
            "builtin/style/profile-naming-convention": "warning",
            "builtin/documentation/missing-description": "info"
        },
        "formatter": {
            "indent_size": 2,
            "max_line_width": 100,
            "caret_alignment": "consistent"
        },
        "autofix": {
            "enable_safe_fixes": true,
            "enable_unsafe_fixes": false
        }
    })
}
