//! CLI command implementations
//!
//! This module contains the implementation of all CLI commands

use fsh_lint_core::{
    AstFormatter, CachedFshParser, ConfigLoader, DefaultExecutor, DefaultFileDiscovery,
    DefaultSemanticAnalyzer, ExecutionContext, Executor, FileDiscovery, Formatter,
    FormatterConfiguration, FshLintConfiguration, Result, Rule, RuleCategory, RuleEngine,
    RuleMetadata,
};
use fsh_lint_rules::gritql::GritQLRuleLoader;
use fsh_lint_rules::{BuiltinRules, DefaultRuleEngine};
use serde_json;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info};

use crate::output::{LintSummary, OutputFormatter, ProgressReporter};
use crate::{ConfigFormat, OutputFormat, Severity};

/// Lint command implementation
pub async fn lint_command(
    paths: Vec<PathBuf>,
    format: OutputFormat,
    _write: bool,
    _dry_run: bool,
    _unsafe: bool,
    _min_severity: Severity,
    include: Vec<String>,
    exclude: Vec<String>,
    error_on_warnings: bool,
    progress: bool,
    config_path: Option<PathBuf>,
) -> Result<()> {
    debug!("Running lint command on paths: {:?}", paths);

    // Load configuration
    let mut config = if let Some(path) = config_path {
        ConfigLoader::load_from_file(&path)?
    } else {
        // Auto-discover config or use default
        let start_path = if !paths.is_empty() && paths[0].is_file() {
            // Get parent directory of the file, or current directory if no parent
            match paths[0].parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent,
                _ => std::path::Path::new("."),
            }
        } else if !paths.is_empty() {
            &paths[0]
        } else {
            std::path::Path::new(".")
        };

        if let Some(discovered_path) = ConfigLoader::auto_discover(start_path)? {
            ConfigLoader::load_from_file(&discovered_path)?
        } else {
            FshLintConfiguration::default()
        }
    };

    // Apply CLI overrides to configuration
    if !include.is_empty() {
        config.files.get_or_insert_with(Default::default).include = Some(include.clone());
    }
    if !exclude.is_empty() {
        config.files.get_or_insert_with(Default::default).exclude = Some(exclude.clone());
    }

    debug!("Loaded configuration");

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
                for entry in
                    glob::glob(&pattern_str).map_err(|e| fsh_lint_core::FshLintError::IoError {
                        path: path.clone(),
                        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
                    })?
                {
                    match entry {
                        Ok(p) if p.extension().map_or(false, |ext| ext == "fsh") => files.push(p),
                        _ => {}
                    }
                }
            } else {
                // Path doesn't exist - try as glob pattern anyway
                let pattern_str = path.to_string_lossy();
                let mut found = false;
                for entry in
                    glob::glob(&pattern_str).map_err(|e| fsh_lint_core::FshLintError::IoError {
                        path: path.clone(),
                        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
                    })?
                {
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

    debug!("Loaded {} built-in rules", compiled_rules.len());

    // Load custom GritQL rules from configured directories
    if let Some(linter_config) = &config.linter {
        if let Some(rule_dirs) = &linter_config.rule_directories {
            if !rule_dirs.is_empty() {
                info!(
                    "Loading custom GritQL rules from {} directories",
                    rule_dirs.len()
                );

                // Resolve paths (handle both absolute and relative)
                let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let rule_paths: Vec<PathBuf> = rule_dirs
                    .iter()
                    .map(|s| {
                        let path = PathBuf::from(s);
                        if path.is_absolute() {
                            path
                        } else {
                            current_dir.join(path)
                        }
                    })
                    .collect();

                let rule_path_refs: Vec<&Path> = rule_paths.iter().map(|p| p.as_path()).collect();

                match GritQLRuleLoader::load_from_directories(&rule_path_refs) {
                    Ok(gritql_loader) => {
                        info!("Loaded {} custom GritQL rules", gritql_loader.len());

                        // Convert GritQL rules to Rule objects and compile them
                        for loaded_rule in gritql_loader.all_rules() {
                            let custom_rule = Rule {
                                id: loaded_rule.id().to_string(),
                                severity: fsh_lint_core::Severity::Warning, // Default severity
                                description: format!(
                                    "Custom GritQL rule from {}",
                                    loaded_rule.source_path().display()
                                ),
                                gritql_pattern: loaded_rule.pattern().pattern.clone(),
                                autofix: None,
                                metadata: RuleMetadata {
                                    id: loaded_rule.id().to_string(),
                                    name: loaded_rule.id().to_string(),
                                    description: format!(
                                        "Custom GritQL rule: {}",
                                        loaded_rule.id()
                                    ),
                                    severity: fsh_lint_core::Severity::Warning,
                                    category: RuleCategory::Custom("gritql".to_string()),
                                    tags: vec!["custom".to_string(), "gritql".to_string()],
                                    version: Some("1.0.0".to_string()),
                                    docs_url: None,
                                },
                                is_ast_rule: false,
                            };

                            match rule_engine.compile_rule(&custom_rule) {
                                Ok(compiled_rule) => {
                                    rule_engine.registry_mut().register(compiled_rule.clone());
                                    compiled_rules.push(compiled_rule);
                                    debug!("Compiled custom rule: {}", loaded_rule.id());
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to compile custom rule {}: {}",
                                        loaded_rule.id(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to load GritQL rules: {}", e);
                    }
                }
            }
        }
    }

    debug!("Total rules loaded: {}", compiled_rules.len());

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
    use std::io::IsTerminal;
    let use_colors = std::env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal();
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

/// Format command implementation
///
/// Dedicated formatter that only formats code without running lint rules.
/// This is separate from linting - it only handles code formatting/prettifying.
pub async fn format_command(
    paths: Vec<PathBuf>,
    write: bool,
    check: bool,
    diff: bool,
    #[allow(unused_variables)] r#unsafe: bool, // Currently unused - formatter doesn't have unsafe fixes
    include: Vec<String>,
    exclude: Vec<String>,
    line_width: Option<usize>,
    indent_size: Option<usize>,
    config_path: Option<PathBuf>,
) -> Result<()> {
    debug!("Running format command on paths: {:?}", paths);

    // Load configuration
    let mut config = if let Some(path) = config_path {
        ConfigLoader::load_from_file(&path)?
    } else {
        let start_path = if !paths.is_empty() && paths[0].is_file() {
            // Get parent directory of the file, or current directory if no parent
            match paths[0].parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent,
                _ => std::path::Path::new("."),
            }
        } else if !paths.is_empty() {
            &paths[0]
        } else {
            std::path::Path::new(".")
        };

        if let Some(discovered_path) = ConfigLoader::auto_discover(start_path)? {
            ConfigLoader::load_from_file(&discovered_path)?
        } else {
            FshLintConfiguration::default()
        }
    };

    // Apply CLI overrides to configuration
    if !include.is_empty() {
        config.files.get_or_insert_with(Default::default).include = Some(include.clone());
    }
    if !exclude.is_empty() {
        config.files.get_or_insert_with(Default::default).exclude = Some(exclude.clone());
    }

    // Override formatter settings from CLI
    if let Some(width) = line_width {
        config
            .formatter
            .get_or_insert_with(Default::default)
            .line_width = Some(width);
    }
    if let Some(indent) = indent_size {
        config
            .formatter
            .get_or_insert_with(Default::default)
            .indent_size = Some(indent);
    }

    let formatter_config = config
        .formatter
        .as_ref()
        .cloned()
        .unwrap_or_else(FormatterConfiguration::default);

    debug!("Loaded configuration");

    let start_time = Instant::now();

    // Determine which files to format
    let fsh_files = if paths.is_empty() {
        let file_discovery = DefaultFileDiscovery::new(std::env::current_dir()?);
        file_discovery.discover_files(&config)?
    } else {
        let mut files = Vec::new();
        for path in paths {
            if path.is_file() {
                if path.extension().map_or(false, |ext| ext == "fsh") {
                    files.push(path);
                }
            } else if path.is_dir() {
                let file_discovery = DefaultFileDiscovery::new(&path);
                files.extend(file_discovery.discover_files(&config)?);
            } else if path.to_string_lossy().contains('*') {
                let pattern_str = path.to_string_lossy();
                for entry in
                    glob::glob(&pattern_str).map_err(|e| fsh_lint_core::FshLintError::IoError {
                        path: path.clone(),
                        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
                    })?
                {
                    match entry {
                        Ok(p) if p.extension().map_or(false, |ext| ext == "fsh") => files.push(p),
                        _ => {}
                    }
                }
            }
        }
        files
    };

    if fsh_files.is_empty() {
        println!("No FSH files found in specified paths.");
        return Ok(());
    }

    debug!("Found {} FSH files to format", fsh_files.len());

    // Create formatter with parser
    let parser = CachedFshParser::new()?;
    let mut formatter = AstFormatter::new(parser);

    let mut files_formatted = 0;
    let mut files_already_formatted = 0;
    let mut errors = 0;

    use std::io::IsTerminal;
    let use_colors = std::env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal();

    for file_path in &fsh_files {
        match formatter.format_file(file_path, &formatter_config) {
            Ok(format_result) => {
                if format_result.changed {
                    if check {
                        // Check mode - report files that need formatting
                        if use_colors {
                            println!("\x1b[33mWould format:\x1b[0m {}", file_path.display());
                        } else {
                            println!("Would format: {}", file_path.display());
                        }
                        files_formatted += 1;
                    } else if diff {
                        // Show diff
                        println!("\n{}", file_path.display());
                        use fsh_lint_core::DiffRenderer;
                        let diff_renderer = DiffRenderer::new();
                        println!(
                            "{}",
                            diff_renderer
                                .render_diff(&format_result.original, &format_result.content)
                        );
                        files_formatted += 1;
                    } else if write {
                        // Write formatted content
                        std::fs::write(file_path, &format_result.content).map_err(|e| {
                            fsh_lint_core::FshLintError::IoError {
                                path: file_path.clone(),
                                source: e,
                            }
                        })?;
                        if use_colors {
                            println!("\x1b[32mFormatted:\x1b[0m {}", file_path.display());
                        } else {
                            println!("Formatted: {}", file_path.display());
                        }
                        files_formatted += 1;
                    } else {
                        // Default: show what would be formatted
                        if use_colors {
                            println!("\x1b[33mWould format:\x1b[0m {}", file_path.display());
                        } else {
                            println!("Would format: {}", file_path.display());
                        }
                        files_formatted += 1;
                    }
                } else {
                    files_already_formatted += 1;
                }
            }
            Err(e) => {
                error!("Error formatting {}: {}", file_path.display(), e);
                errors += 1;
            }
        }
    }

    let duration = start_time.elapsed();

    // Print summary
    println!();
    if write {
        if use_colors {
            println!(
                "\x1b[1mFormatted {} file{}\x1b[0m ({} already formatted, {} error{})",
                files_formatted,
                if files_formatted == 1 { "" } else { "s" },
                files_already_formatted,
                errors,
                if errors == 1 { "" } else { "s" }
            );
        } else {
            println!(
                "Formatted {} file{} ({} already formatted, {} error{})",
                files_formatted,
                if files_formatted == 1 { "" } else { "s" },
                files_already_formatted,
                errors,
                if errors == 1 { "" } else { "s" }
            );
        }
    } else if check {
        if use_colors {
            if files_formatted > 0 {
                println!(
                    "\x1b[31m{} file{} need formatting\x1b[0m ({} already formatted, {} error{})",
                    files_formatted,
                    if files_formatted == 1 { "" } else { "s" },
                    files_already_formatted,
                    errors,
                    if errors == 1 { "" } else { "s" }
                );
            } else {
                println!("\x1b[32mAll files are formatted correctly!\x1b[0m");
            }
        } else {
            if files_formatted > 0 {
                println!(
                    "{} file{} need formatting ({} already formatted, {} error{})",
                    files_formatted,
                    if files_formatted == 1 { "" } else { "s" },
                    files_already_formatted,
                    errors,
                    if errors == 1 { "" } else { "s" }
                );
            } else {
                println!("All files are formatted correctly!");
            }
        }
    } else {
        if use_colors {
            println!(
                "\x1b[33m{} file{} would be formatted\x1b[0m ({} already formatted, {} error{})",
                files_formatted,
                if files_formatted == 1 { "" } else { "s" },
                files_already_formatted,
                errors,
                if errors == 1 { "" } else { "s" }
            );
        } else {
            println!(
                "{} file{} would be formatted ({} already formatted, {} error{})",
                files_formatted,
                if files_formatted == 1 { "" } else { "s" },
                files_already_formatted,
                errors,
                if errors == 1 { "" } else { "s" }
            );
        }
    }

    println!(
        "Completed in {}",
        crate::output::utils::format_duration(duration)
    );

    // Exit with error code if check mode and files need formatting
    if check && files_formatted > 0 {
        std::process::exit(1);
    }

    // Exit with error if there were parse/format errors
    if errors > 0 {
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
    let config = if let Some(path) = config_path {
        ConfigLoader::load_from_file(&path)?
    } else {
        FshLintConfiguration::default()
    };

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
        if let Some(ref _filter_tag) = tag {
            // Skip if no tags match
            continue;
        }

        count += 1;

        if detailed {
            println!("\n{}", rule.id);
            println!("  Description: {}", rule.metadata.description);
            println!("  Category: {}", rule_category);
            // Check if rule is configured
            let is_configured = config
                .linter
                .as_ref()
                .and_then(|l| l.rules.as_ref())
                .map(|_rules| {
                    // For now, just check if recommended is set
                    true
                })
                .unwrap_or(false);

            println!(
                "  Status: {}",
                if is_configured {
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

    match if let Some(p) = path {
        ConfigLoader::load_from_file(&p)
    } else {
        ConfigLoader::load_from_file(&std::env::current_dir()?.join("fsh-lint.json"))
    } {
        Ok(config) => {
            println!("✅ Configuration is valid");
            let linter_enabled = config
                .linter
                .as_ref()
                .and_then(|l| l.enabled)
                .unwrap_or(true);
            let formatter_enabled = config
                .formatter
                .as_ref()
                .and_then(|f| f.enabled)
                .unwrap_or(true);
            println!("   Linter enabled: {}", linter_enabled);
            println!("   Formatter enabled: {}", formatter_enabled);
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

    let config = if let Some(path) = config_path {
        ConfigLoader::load_from_file(&path)?
    } else {
        FshLintConfiguration::default()
    };

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
        "linter": {
            "enabled": true,
            "ruleDirectories": [],
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
            "exclude": ["node_modules/**", "target/**", "build/**"]
        }
    })
}

/// Create an example configuration with sample rules
fn create_example_config() -> serde_json::Value {
    serde_json::json!({
        "linter": {
            "enabled": true,
            "ruleDirectories": ["rules/custom"],
            "rules": {
                "recommended": true,
                "all": false,
                "correctness": {
                    "invalid-keyword": "error",
                    "invalid-constraint": "error"
                },
                "suspicious": {
                    "trailing-text": "warning"
                },
                "style": {
                    "profile-naming-convention": "warning",
                    "naming-convention": "warning"
                },
                "documentation": {
                    "missing-description": "info",
                    "missing-title": "info"
                }
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
            "exclude": ["node_modules/**", "target/**", "build/**", "fsh-generated/**"]
        }
    })
}
