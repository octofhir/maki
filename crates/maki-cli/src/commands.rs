//! CLI command implementations
//!
//! This module contains the implementation of all CLI commands.
//!
//! Commands are organized hierarchically:
//! - Top-level commands (lint, format, rules) are implemented in this file
//! - Subcommands with multiple actions are in subdirectories:
//!   - commands/config/ - Configuration management (init, migrate, validate, show)
//!   - commands/build/ - Build command (future: SUSHI-compatible build)
//!   - commands/init/ - Init command (future: project initialization)

// Command modules organized hierarchically
pub mod build;
pub mod config;
pub mod gofsh;
pub mod init;

use maki_core::config::{DependencyVersion, UnifiedConfig};
use maki_core::{
    AstFormatter, AutofixEngine, CachedFshParser, CanonicalFacade, CanonicalOptions, ConfigLoader,
    DefaultAutofixEngine, DefaultExecutor, DefaultFileDiscovery, DefaultSemanticAnalyzer,
    DefinitionSession, ExecutionContext, Executor, FhirRelease, FileDiscovery, FixConfig,
    FormatterConfiguration, PackageCoordinate, Result, Rule, RuleCategory, RuleEngine,
    RuleMetadata,
};
use maki_rules::gritql::GritQLRuleLoader;
use maki_rules::{BuiltinRules, DefaultRuleEngine};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info};

use crate::output::{LintSummary, OutputFormatter, ProgressReporter};
use crate::{ConfigFormat, OutputFormat, Severity};

/// Lint command implementation
#[allow(clippy::too_many_arguments)]
pub async fn lint_command(
    paths: Vec<PathBuf>,
    format: OutputFormat,
    write: bool,
    dry_run: bool,
    r#unsafe: bool,
    interactive: bool,
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
            UnifiedConfig::default()
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

    // Step 1: Set up canonical manager if dependencies exist (top-level or build config)
    // Note: session is prepared but not yet passed to rules - future enhancement
    let _session: Option<Arc<DefinitionSession>> = if config.dependencies.is_some()
        || (config.build.is_some() && config.build.as_ref().unwrap().dependencies.is_some())
    {
        info!("Setting up FHIR package dependencies from configuration...");

        // Create canonical facade
        let canonical_options = CanonicalOptions {
            quick_init: true,
            ..Default::default()
        };

        let facade = CanonicalFacade::new(canonical_options).await.map_err(|e| {
            maki_core::MakiError::ConfigError {
                message: format!("Failed to create CanonicalFacade: {}", e),
            }
        })?;

        // Get FHIR versions from build config
        let fhir_releases: Vec<FhirRelease> = if let Some(build_config) = &config.build {
            build_config
                .fhir_version
                .iter()
                .filter_map(|v| match v.as_str() {
                    "4.0.1" => Some(FhirRelease::R4),
                    "4.3.0" => Some(FhirRelease::R4B),
                    "5.0.0" => Some(FhirRelease::R5),
                    _ => {
                        error!("Unsupported FHIR version: {}", v);
                        None
                    }
                })
                .collect()
        } else {
            // Default to R4 if no build config
            vec![FhirRelease::R4]
        };

        if fhir_releases.is_empty() {
            error!("No valid FHIR versions found in configuration, using R4 as default");
        }

        let fhir_releases = if fhir_releases.is_empty() {
            vec![FhirRelease::R4]
        } else {
            fhir_releases
        };

        // Create session
        let session = Arc::new(facade.session(fhir_releases.clone()).await.map_err(|e| {
            maki_core::MakiError::ConfigError {
                message: format!("Failed to create DefinitionSession: {}", e),
            }
        })?);

        // Get all dependencies (top-level takes precedence over build section)
        let all_deps = config.all_dependencies();

        if !all_deps.is_empty() {
            let coords: Vec<PackageCoordinate> = all_deps
                .iter()
                .map(|(name, dep_version)| {
                    let version = match dep_version {
                        DependencyVersion::Simple(v) => v.clone(),
                        DependencyVersion::Complex { version, .. } => version.clone(),
                    };
                    PackageCoordinate::new(name, version)
                })
                .collect();

            info!("Installing {} FHIR package dependencies...", coords.len());
            session.ensure_packages(coords).await.map_err(|e| {
                maki_core::MakiError::ConfigError {
                    message: format!("Failed to install FHIR packages: {}", e),
                }
            })?;
            info!("✓ Dependencies installed successfully");
        }

        Some(session)
    } else {
        debug!("No dependencies found in configuration, skipping FHIR package setup");
        None
    };

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
                if path.extension().is_some_and(|ext| ext == "fsh") {
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
                    glob::glob(&pattern_str).map_err(|e| maki_core::MakiError::IoError {
                        path: path.clone(),
                        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
                    })?
                {
                    match entry {
                        Ok(p) if p.extension().is_some_and(|ext| ext == "fsh") => files.push(p),
                        _ => {}
                    }
                }
            } else {
                // Path doesn't exist - try as glob pattern anyway
                let pattern_str = path.to_string_lossy();
                let mut found = false;
                for entry in
                    glob::glob(&pattern_str).map_err(|e| maki_core::MakiError::IoError {
                        path: path.clone(),
                        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
                    })?
                {
                    match entry {
                        Ok(p) if p.extension().is_some_and(|ext| ext == "fsh") => {
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

    // Set the canonical manager session if available
    if let Some(session) = &_session {
        rule_engine.set_session(session.clone());
        info!("Canonical manager session configured for rule engine");
    }

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
    if let Some(linter_config) = &config.linter
        && let Some(rule_dirs) = &linter_config.rule_directories
        && !rule_dirs.is_empty()
    {
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
                        severity: maki_core::Severity::Warning, // Default severity
                        description: format!(
                            "Custom GritQL rule from {}",
                            loaded_rule.source_path().display()
                        ),
                        gritql_pattern: loaded_rule.pattern().pattern.clone(),
                        autofix: None,
                        metadata: RuleMetadata {
                            id: loaded_rule.id().to_string(),
                            name: loaded_rule.id().to_string(),
                            description: format!("Custom GritQL rule: {}", loaded_rule.id()),
                            severity: maki_core::Severity::Warning,
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
                            error!("Failed to compile custom rule {}: {}", loaded_rule.id(), e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to load GritQL rules: {}", e);
            }
        }
    }

    debug!("Total rules loaded: {}", compiled_rules.len());

    let semantic_analyzer = Box::new(DefaultSemanticAnalyzer::new());
    let context = ExecutionContext::new(config.clone(), compiled_rules);
    let executor = DefaultExecutor::new(context, semantic_analyzer, Box::new(rule_engine));

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
                    maki_core::Severity::Error => summary.errors += 1,
                    maki_core::Severity::Warning => summary.warnings += 1,
                    maki_core::Severity::Info => summary.info += 1,
                    maki_core::Severity::Hint => summary.hints += 1,
                }
            }
            all_diagnostics.extend(result.diagnostics);
        }
    }

    // Apply fixes if requested
    let mut fixed_diagnostic_ids: std::collections::HashSet<(std::path::PathBuf, usize, String)> =
        std::collections::HashSet::new();

    use std::io::IsTerminal;
    let use_colors = std::env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal();

    if write || dry_run {
        let autofix_engine = DefaultAutofixEngine::new();

        // Generate fixes from diagnostics
        let fixes = autofix_engine.generate_fixes(&all_diagnostics)?;

        if !fixes.is_empty() {
            // Track which diagnostics will be fixed by finding diagnostics with safe suggestions
            // We can't use fix.location because it's the insertion point, not the diagnostic location
            for diagnostic in &all_diagnostics {
                // Check if this diagnostic has a safe suggestion that would be applied
                let has_safe_fix = diagnostic
                    .suggestions
                    .iter()
                    .any(|s| s.applicability == maki_core::Applicability::Always);
                if has_safe_fix {
                    fixed_diagnostic_ids.insert((
                        diagnostic.location.file.clone(),
                        diagnostic.location.offset,
                        diagnostic.rule_id.clone(),
                    ));
                }
            }

            // Create fix configuration based on CLI flags
            let fix_config = FixConfig {
                apply_unsafe: r#unsafe || interactive, // Interactive mode implies unsafe fixes are available
                dry_run,
                interactive,
                max_fixes_per_file: None,
                validate_syntax: true,
            };

            // Apply fixes
            let fix_results = autofix_engine.apply_fixes(&fixes, &fix_config)?;

            // Update summary with number of fixes applied
            for fix_result in &fix_results {
                summary.fixes_applied += fix_result.applied_count;

                // Show errors from fix application
                if !fix_result.errors.is_empty() {
                    for err in &fix_result.errors {
                        error!("Fix error in {}: {}", fix_result.file.display(), err);
                    }
                }
            }

            // Show applied fixes
            if summary.fixes_applied > 0 {
                if dry_run {
                    println!(
                        "{}Would apply {} fix{}:{}\n",
                        if use_colors { "\x1b[36m" } else { "" },
                        summary.fixes_applied,
                        if summary.fixes_applied == 1 { "" } else { "es" },
                        if use_colors { "\x1b[0m" } else { "" }
                    );
                } else {
                    println!(
                        "{}Applied {} fix{}:{}\n",
                        if use_colors { "\x1b[32m" } else { "" },
                        summary.fixes_applied,
                        if summary.fixes_applied == 1 { "" } else { "es" },
                        if use_colors { "\x1b[0m" } else { "" }
                    );
                }

                // Show details of applied fixes
                for fix in &fixes {
                    println!(
                        "  {} {} in {}",
                        if use_colors {
                            "\x1b[32m✓\x1b[0m"
                        } else {
                            "✓"
                        },
                        fix.description,
                        fix.location.file.display()
                    );
                }
                println!();
            }
        }
    }

    // Filter out diagnostics that were fixed (only when --write was used, not dry-run)
    let remaining_diagnostics: Vec<_> = if write && !dry_run && !fixed_diagnostic_ids.is_empty() {
        all_diagnostics
            .into_iter()
            .filter(|d| {
                !fixed_diagnostic_ids.contains(&(
                    d.location.file.clone(),
                    d.location.offset,
                    d.rule_id.clone(),
                ))
            })
            .collect()
    } else {
        all_diagnostics
    };

    // Recalculate summary based on remaining diagnostics
    if write && !dry_run && !fixed_diagnostic_ids.is_empty() {
        summary.errors = 0;
        summary.warnings = 0;
        summary.info = 0;
        summary.hints = 0;
        for d in &remaining_diagnostics {
            match d.severity {
                maki_core::Severity::Error => summary.errors += 1,
                maki_core::Severity::Warning => summary.warnings += 1,
                maki_core::Severity::Info => summary.info += 1,
                maki_core::Severity::Hint => summary.hints += 1,
            }
        }
    }

    // Format and print remaining results
    let formatter = OutputFormatter::new(format, use_colors);
    formatter.print_results(&remaining_diagnostics, &summary, progress)?;

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
#[allow(clippy::too_many_arguments)]
pub async fn format_command(
    paths: Vec<PathBuf>,
    format: OutputFormat,
    write: bool,
    check: bool,
    diff: bool,
    #[allow(unused_variables)] r#unsafe: bool, // Kept for API consistency, formatter fixes are always safe
    include: Vec<String>,
    exclude: Vec<String>,
    line_width: Option<usize>,
    indent_size: Option<usize>,
    progress: bool,
    quiet: bool,
    show_config: bool,
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
            UnifiedConfig::default()
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

    // If show_config is true, display configuration and exit
    if show_config {
        if !quiet {
            println!("Current formatting configuration:");
            println!(
                "  Enabled:        {}",
                formatter_config.enabled.unwrap_or(true)
            );
            println!(
                "  Indent size:    {}",
                formatter_config.indent_size.unwrap_or(2)
            );
            println!(
                "  Line width:     {}",
                formatter_config.line_width.unwrap_or(100)
            );
            println!(
                "  Align carets:   {}",
                formatter_config.align_carets.unwrap_or(false)
            );
            println!(
                "  Group rules:    {}",
                formatter_config.group_rules.unwrap_or(false)
            );
            println!(
                "  Sort rules:     {}",
                formatter_config.sort_rules.unwrap_or(false)
            );
            println!(
                "  Normalize spacing: {}",
                formatter_config.normalize_spacing.unwrap_or(true)
            );
            println!(
                "  Blank line before rules: {}",
                formatter_config.blank_line_before_rules.unwrap_or(true)
            );
            println!(
                "  Blank lines between groups: {}",
                formatter_config.blank_lines_between_groups.unwrap_or(1)
            );
        }
        return Ok(());
    }

    let start_time = Instant::now();

    // Determine which files to format
    let fsh_files = if paths.is_empty() {
        let file_discovery = DefaultFileDiscovery::new(std::env::current_dir()?);
        file_discovery.discover_files(&config)?
    } else {
        let mut files = Vec::new();
        for path in paths {
            if path.is_file() {
                if path.extension().is_some_and(|ext| ext == "fsh") {
                    files.push(path);
                }
            } else if path.is_dir() {
                let file_discovery = DefaultFileDiscovery::new(&path);
                files.extend(file_discovery.discover_files(&config)?);
            } else if path.to_string_lossy().contains('*') {
                let pattern_str = path.to_string_lossy();
                for entry in
                    glob::glob(&pattern_str).map_err(|e| maki_core::MakiError::IoError {
                        path: path.clone(),
                        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
                    })?
                {
                    match entry {
                        Ok(p) if p.extension().is_some_and(|ext| ext == "fsh") => files.push(p),
                        _ => {}
                    }
                }
            }
        }
        files
    };

    if fsh_files.is_empty() {
        if !quiet {
            println!("No FSH files found in specified paths.");
        }
        return Ok(());
    }

    debug!("Found {} FSH files to format", fsh_files.len());

    if progress && !quiet {
        println!(
            "Formatting {} file{}...",
            fsh_files.len(),
            if fsh_files.len() == 1 { "" } else { "s" }
        );
    }

    // Generate diagnostics for files that need formatting
    // For better performance with large projects, we process files in parallel using rayon
    use rayon::prelude::*;
    use std::sync::{Arc, Mutex};

    // Create a progress bar if progress mode is enabled
    let progress_bar = if progress && !quiet {
        let pb = indicatif::ProgressBar::new(fsh_files.len() as u64);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(Arc::new(pb))
    } else {
        None
    };

    let diagnostics: Vec<_> = fsh_files
        .par_iter()
        .filter_map(|file_path| {
            let result = {
                // Each thread needs its own parser and formatter
                let thread_parser = CachedFshParser::new().ok()?;
                let mut thread_formatter = AstFormatter::new(thread_parser);
                thread_formatter
                    .format_file_with_diagnostic(file_path.as_path(), &formatter_config)
                    .ok()
            };

            // Update progress bar
            if let Some(ref pb) = progress_bar {
                pb.inc(1);
            }

            result
        })
        .flatten()
        .collect();

    // Finish progress bar
    if let Some(pb) = progress_bar {
        pb.finish_with_message("Formatting complete");
    }

    // Build summary
    let mut summary = LintSummary::new();
    summary.files_checked = fsh_files.len();
    summary.info = diagnostics.len(); // Each diagnostic is an info-level "file needs formatting"

    // Handle diff mode separately (doesn't fit into diagnostic model)
    if diff {
        use maki_core::DiffRenderer;
        let diff_renderer = DiffRenderer::new();

        for diagnostic in &diagnostics {
            println!("\n{}", diagnostic.location.file.display());

            if let Some(suggestion) = diagnostic.suggestions.first() {
                // Read original content
                let original = std::fs::read_to_string(&diagnostic.location.file)?;
                println!(
                    "{}",
                    diff_renderer.render_diff(&original, &suggestion.replacement)
                );
            }
        }

        println!(
            "\nCompleted in {}",
            crate::output::utils::format_duration(start_time.elapsed())
        );
        return Ok(());
    }

    // Apply formatting fixes if requested
    if write {
        let fixes_applied = Arc::new(Mutex::new(0usize));

        // Create progress bar for writing if progress mode is enabled
        let write_progress_bar = if progress && !quiet && !diagnostics.is_empty() {
            let pb = indicatif::ProgressBar::new(diagnostics.len() as u64);
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} Writing fixes...")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            Some(Arc::new(pb))
        } else {
            None
        };

        diagnostics.par_iter().for_each(|diagnostic| {
            // Each thread needs its own formatter
            let thread_parser = match CachedFshParser::new() {
                Ok(p) => p,
                Err(_) => return,
            };
            let mut thread_formatter = AstFormatter::new(thread_parser);

            // Get the formatted content for this file
            if let Ok(Some(formatted_content)) =
                thread_formatter.get_formatted_content(&diagnostic.location.file, &formatter_config)
            {
                // Write the formatted content
                if std::fs::write(&diagnostic.location.file, &formatted_content).is_ok() {
                    let mut applied = fixes_applied.lock().unwrap();
                    *applied += 1;
                }
            }

            // Update progress bar
            if let Some(ref pb) = write_progress_bar {
                pb.inc(1);
            }
        });

        // Finish progress bar
        if let Some(pb) = write_progress_bar {
            pb.finish_and_clear();
        }

        summary.fixes_applied = *fixes_applied.lock().unwrap();
    }

    // Format and print results using same system as lint
    use std::io::IsTerminal;
    let use_colors = std::env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal();

    // Only show diagnostics if in check mode or if nothing was written
    if !quiet {
        if check || !write {
            let formatter_output = OutputFormatter::new(format, use_colors);
            formatter_output.print_results(&diagnostics, &summary, false)?;
        } else {
            // In write mode, just show summary
            println!("{} files checked", summary.files_checked);
            if summary.fixes_applied > 0 {
                if use_colors {
                    println!(
                        "\x1b[32m✓\x1b[0m Applied formatting to {} file{}",
                        summary.fixes_applied,
                        if summary.fixes_applied == 1 { "" } else { "s" }
                    );
                } else {
                    println!(
                        "Applied formatting to {} file{}",
                        summary.fixes_applied,
                        if summary.fixes_applied == 1 { "" } else { "s" }
                    );
                }
            } else if use_colors {
                println!("\x1b[32m✓\x1b[0m All files are formatted correctly");
            } else {
                println!("All files are formatted correctly");
            }
        }

        println!(
            "Completed in {}",
            crate::output::utils::format_duration(start_time.elapsed())
        );
    }

    // Exit with error code if check mode and files need formatting
    if check && !diagnostics.is_empty() {
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
        UnifiedConfig::default()
    };

    // Collect all built-in rules
    let all_rules = BuiltinRules::all_rules();

    println!("Available Rules:");
    println!("================");

    let mut count = 0;
    for rule in all_rules {
        let rule_category = match &rule.metadata.category {
            maki_core::RuleCategory::Correctness => "correctness",
            maki_core::RuleCategory::Suspicious => "suspicious",
            maki_core::RuleCategory::Style => "style",
            maki_core::RuleCategory::Complexity => "complexity",
            maki_core::RuleCategory::Documentation => "documentation",
            maki_core::RuleCategory::Performance => "performance",
            maki_core::RuleCategory::Nursery => "nursery",
            maki_core::RuleCategory::Accessibility => "accessibility",
            maki_core::RuleCategory::Security => "security",
            maki_core::RuleCategory::Custom(s) => s.as_str(),
        };

        // Apply category filter
        if let Some(ref filter_cat) = category
            && rule_category != filter_cat.as_str()
        {
            continue;
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
            println!("  Category: {rule_category}");
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
        println!("\nTotal: {count} rules");
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
                maki_core::RuleCategory::Correctness => "Correctness",
                maki_core::RuleCategory::Suspicious => "Suspicious",
                maki_core::RuleCategory::Style => "Style",
                maki_core::RuleCategory::Complexity => "Complexity",
                maki_core::RuleCategory::Documentation => "Documentation",
                maki_core::RuleCategory::Performance => "Performance",
                maki_core::RuleCategory::Nursery => "Nursery",
                maki_core::RuleCategory::Accessibility => "Accessibility",
                maki_core::RuleCategory::Security => "Security",
                maki_core::RuleCategory::Custom(s) => s.as_str(),
            };

            println!("Rule: {}", rule.id);
            println!("{}", "=".repeat(rule.id.len() + 6));
            println!();
            println!("Category: {category}");
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
            println!("Rule '{rule_id}' not found.");
            println!();
            println!("Use 'maki rules' to list all available rules.");
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
        println!("No rules found matching '{query}'");
    } else {
        println!("Rules matching '{query}':");
        println!("{}", "=".repeat(20 + query.len()));
        println!();
        for rule in matches {
            let category = match &rule.metadata.category {
                maki_core::RuleCategory::Correctness => "correctness",
                maki_core::RuleCategory::Suspicious => "suspicious",
                maki_core::RuleCategory::Style => "style",
                maki_core::RuleCategory::Complexity => "complexity",
                maki_core::RuleCategory::Documentation => "documentation",
                maki_core::RuleCategory::Performance => "performance",
                maki_core::RuleCategory::Nursery => "nursery",
                maki_core::RuleCategory::Accessibility => "accessibility",
                maki_core::RuleCategory::Security => "security",
                maki_core::RuleCategory::Custom(s) => s.as_str(),
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
        ConfigFormat::Json => ".makirc.json",
        ConfigFormat::Toml => ".makirc.toml",
    };

    let config_path = PathBuf::from(filename);

    // Check if file already exists
    if config_path.exists() && !force {
        error!(
            "Configuration file '{}' already exists. Use --force to overwrite.",
            filename
        );
        return Err(maki_core::MakiError::ConfigError {
            message: format!("Configuration file '{filename}' already exists"),
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
            maki_core::MakiError::ConfigError {
                message: format!("Failed to serialize JSON: {e}"),
            }
        })?,
        ConfigFormat::Toml => toml::to_string_pretty(&default_config).map_err(|e| {
            maki_core::MakiError::ConfigError {
                message: format!("Failed to serialize TOML: {e}"),
            }
        })?,
    };

    std::fs::write(&config_path, config_content)?;

    println!("✅ Created configuration file: {filename}");
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
        ConfigLoader::load(None, None)
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
            println!("   Linter enabled: {linter_enabled}");
            println!("   Formatter enabled: {formatter_enabled}");
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
        UnifiedConfig::default()
    };

    if resolved {
        println!("Resolved Configuration:");
        println!("======================");
    } else {
        println!("Configuration:");
        println!("==============");
    }

    let config_json =
        serde_json::to_string_pretty(&config).map_err(|e| maki_core::MakiError::ConfigError {
            message: format!("Failed to serialize config: {e}"),
        })?;
    println!("{config_json}");

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
