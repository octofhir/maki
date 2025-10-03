//! Output formatting and reporting
//!
//! This module handles different output formats and provides rich reporting capabilities

use colored::*;
use fsh_lint_core::{Result, diagnostics::Diagnostic};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::OutputFormat;

/// Summary statistics for linting results
#[derive(Debug, Clone)]
pub struct LintSummary {
    pub files_checked: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub hints: usize,
    pub fixes_applied: usize,
}

impl LintSummary {
    pub fn new() -> Self {
        Self {
            files_checked: 0,
            errors: 0,
            warnings: 0,
            info: 0,
            hints: 0,
            fixes_applied: 0,
        }
    }

    pub fn total_issues(&self) -> usize {
        self.errors + self.warnings + self.info + self.hints
    }

    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }

    pub fn has_issues(&self) -> bool {
        self.total_issues() > 0
    }
}

/// Output formatter for different formats
pub struct OutputFormatter {
    format: OutputFormat,
    use_colors: bool,
}

impl OutputFormatter {
    pub fn new(format: OutputFormat, use_colors: bool) -> Self {
        Self { format, use_colors }
    }

    /// Format and print linting results
    pub fn print_results(
        &self,
        diagnostics: &[Diagnostic],
        summary: &LintSummary,
        show_progress: bool,
    ) -> Result<()> {
        match self.format {
            OutputFormat::Human => self.print_human_format(diagnostics, summary, show_progress),
            OutputFormat::Json => self.print_json_format(diagnostics, summary),
            OutputFormat::Sarif => self.print_sarif_format(diagnostics, summary),
            OutputFormat::Compact => self.print_compact_format(summary),
            OutputFormat::Github => self.print_github_format(diagnostics),
        }
    }

    fn print_human_format(
        &self,
        diagnostics: &[Diagnostic],
        summary: &LintSummary,
        show_progress: bool,
    ) -> Result<()> {
        if show_progress {
            println!("{} Linting FSH files...", "🔍".bright_blue());
        }

        if diagnostics.is_empty() {
            println!("{} No issues found", "✅".green());
        } else {
            // Group diagnostics by file
            let mut by_file: HashMap<PathBuf, Vec<&Diagnostic>> = HashMap::new();
            for diagnostic in diagnostics {
                by_file
                    .entry(diagnostic.location.file.clone())
                    .or_default()
                    .push(diagnostic);
            }

            for (file, file_diagnostics) in by_file {
                println!("\n{}", file.display().to_string().bold());

                for diagnostic in file_diagnostics {
                    self.print_diagnostic_human(diagnostic)?;
                }
            }
        }

        // Print summary
        self.print_summary_human(summary)?;
        Ok(())
    }

    fn print_diagnostic_human(&self, diagnostic: &Diagnostic) -> Result<()> {
        let severity_icon = match diagnostic.severity {
            fsh_lint_core::diagnostics::Severity::Error => "❌".red(),
            fsh_lint_core::diagnostics::Severity::Warning => "⚠️".yellow(),
            fsh_lint_core::diagnostics::Severity::Info => "ℹ️".blue(),
            fsh_lint_core::diagnostics::Severity::Hint => "💡".cyan(),
        };

        let location = format!(
            "{}:{}:{}",
            diagnostic.location.line,
            diagnostic.location.column,
            diagnostic
                .location
                .end_column
                .unwrap_or(diagnostic.location.column)
        );

        println!(
            "  {} {} {} {}",
            severity_icon,
            location.dimmed(),
            diagnostic.message,
            format!("({})", diagnostic.rule_id).dimmed()
        );

        // Show code context if available
        if let Some(ref context) = diagnostic.code_snippet {
            println!("    {}", context.dimmed());

            // Show caret indicator
            let spaces = " ".repeat(diagnostic.location.column.saturating_sub(1));
            let carets = "^".repeat(
                diagnostic
                    .location
                    .end_column
                    .unwrap_or(diagnostic.location.column)
                    - diagnostic.location.column
                    + 1,
            );
            println!("    {}{}", spaces, carets.red());
        }

        // Show suggestions if available
        for suggestion in &diagnostic.suggestions {
            println!("    {} {}", "💡".cyan(), suggestion.message.dimmed());
        }

        Ok(())
    }

    fn print_summary_human(&self, summary: &LintSummary) -> Result<()> {
        println!("\n{}", "Summary:".bold());
        println!("  Files checked: {}", summary.files_checked);

        if summary.has_issues() {
            println!("  Issues found:");
            if summary.errors > 0 {
                println!("    Errors: {}", summary.errors.to_string().red());
            }
            if summary.warnings > 0 {
                println!("    Warnings: {}", summary.warnings.to_string().yellow());
            }
            if summary.info > 0 {
                println!("    Info: {}", summary.info.to_string().blue());
            }
            if summary.hints > 0 {
                println!("    Hints: {}", summary.hints.to_string().cyan());
            }
        } else {
            println!("  {} No issues found", "✅".green());
        }

        if summary.fixes_applied > 0 {
            println!(
                "  Fixes applied: {}",
                summary.fixes_applied.to_string().green()
            );
        }

        Ok(())
    }

    fn print_json_format(&self, diagnostics: &[Diagnostic], summary: &LintSummary) -> Result<()> {
        let json_diagnostics: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(|d| self.diagnostic_to_json(d))
            .collect();

        let result = serde_json::json!({
            "files_checked": summary.files_checked,
            "issues": json_diagnostics,
            "summary": {
                "errors": summary.errors,
                "warnings": summary.warnings,
                "info": summary.info,
                "hints": summary.hints,
                "total": summary.total_issues(),
                "fixes_applied": summary.fixes_applied
            }
        });

        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| {
                fsh_lint_core::FshLintError::ConfigError {
                    message: format!("Failed to serialize JSON: {}", e),
                }
            })?
        );

        Ok(())
    }

    fn print_sarif_format(&self, diagnostics: &[Diagnostic], summary: &LintSummary) -> Result<()> {
        let sarif_results: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(|d| self.diagnostic_to_sarif(d))
            .collect();

        let sarif = serde_json::json!({
            "version": "2.1.0",
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "fsh-lint",
                        "version": fsh_lint_core::VERSION,
                        "informationUri": "https://github.com/octofhir/fsh-lint-rs"
                    }
                },
                "results": sarif_results,
                "invocations": [{
                    "executionSuccessful": !summary.has_errors(),
                    "toolExecutionNotifications": []
                }]
            }]
        });

        println!(
            "{}",
            serde_json::to_string_pretty(&sarif).map_err(|e| {
                fsh_lint_core::FshLintError::ConfigError {
                    message: format!("Failed to serialize SARIF: {}", e),
                }
            })?
        );

        Ok(())
    }

    fn print_compact_format(&self, summary: &LintSummary) -> Result<()> {
        if summary.has_issues() {
            println!(
                "fsh-lint: {} files, {} issues ({} errors, {} warnings)",
                summary.files_checked,
                summary.total_issues(),
                summary.errors,
                summary.warnings
            );
        } else {
            println!(
                "fsh-lint: {} files checked, no issues",
                summary.files_checked
            );
        }

        if summary.fixes_applied > 0 {
            println!("fsh-lint: {} fixes applied", summary.fixes_applied);
        }

        Ok(())
    }

    fn print_github_format(&self, diagnostics: &[Diagnostic]) -> Result<()> {
        for diagnostic in diagnostics {
            let level = match diagnostic.severity {
                fsh_lint_core::diagnostics::Severity::Error => "error",
                fsh_lint_core::diagnostics::Severity::Warning => "warning",
                _ => "notice",
            };

            println!(
                "::{} file={},line={},col={}::{} ({})",
                level,
                diagnostic.location.file.display(),
                diagnostic.location.line,
                diagnostic.location.column,
                diagnostic.message,
                diagnostic.rule_id
            );
        }

        Ok(())
    }

    fn diagnostic_to_json(&self, diagnostic: &Diagnostic) -> serde_json::Value {
        serde_json::json!({
            "rule_id": diagnostic.rule_id,
            "severity": match diagnostic.severity {
                fsh_lint_core::diagnostics::Severity::Error => "error",
                fsh_lint_core::diagnostics::Severity::Warning => "warning",
                fsh_lint_core::diagnostics::Severity::Info => "info",
                fsh_lint_core::diagnostics::Severity::Hint => "hint",
            },
            "message": diagnostic.message,
            "location": {
                "file": diagnostic.location.file.display().to_string(),
                "line": diagnostic.location.line,
                "column": diagnostic.location.column,
                "end_column": diagnostic.location.end_column,
                "code_snippet": diagnostic.code_snippet
            },
            "suggestions": diagnostic.suggestions.iter().map(|s| serde_json::json!({
                "message": s.message,
                "replacement": s.replacement,
                "location": {
                    "file": s.location.file.display().to_string(),
                    "line": s.location.line,
                    "column": s.location.column,
                    "end_column": s.location.end_column
                },
                "is_safe": s.is_safe
            })).collect::<Vec<_>>()
        })
    }

    fn diagnostic_to_sarif(&self, diagnostic: &Diagnostic) -> serde_json::Value {
        let level = match diagnostic.severity {
            fsh_lint_core::diagnostics::Severity::Error => "error",
            fsh_lint_core::diagnostics::Severity::Warning => "warning",
            fsh_lint_core::diagnostics::Severity::Info => "note",
            fsh_lint_core::diagnostics::Severity::Hint => "note",
        };

        serde_json::json!({
            "ruleId": diagnostic.rule_id,
            "level": level,
            "message": {
                "text": diagnostic.message
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": diagnostic.location.file.display().to_string()
                    },
                    "region": {
                        "startLine": diagnostic.location.line,
                        "startColumn": diagnostic.location.column,
                        "endColumn": diagnostic.location.end_column.unwrap_or(diagnostic.location.column)
                    }
                }
            }],
            "fixes": diagnostic.suggestions.iter().map(|s| serde_json::json!({
                "description": {
                    "text": s.message
                },
                "artifactChanges": [{
                    "artifactLocation": {
                        "uri": s.location.file.display().to_string()
                    },
                    "replacements": [{
                        "deletedRegion": {
                            "startLine": s.location.line,
                            "startColumn": s.location.column,
                            "endColumn": s.location.end_column.unwrap_or(s.location.column)
                        },
                        "insertedContent": {
                            "text": s.replacement
                        }
                    }]
                }]
            })).collect::<Vec<_>>()
        })
    }
}

/// Progress reporter for long-running operations
pub struct ProgressReporter {
    enabled: bool,
    total: usize,
    current: usize,
}

impl ProgressReporter {
    pub fn new(enabled: bool, total: usize) -> Self {
        Self {
            enabled,
            total,
            current: 0,
        }
    }

    pub fn update(&mut self, current: usize, message: &str) {
        if !self.enabled {
            return;
        }

        self.current = current;
        let percentage = if self.total > 0 {
            (current * 100) / self.total
        } else {
            0
        };

        eprint!("\r{} [{}/{}] {}%", message, current, self.total, percentage);

        if current >= self.total {
            eprintln!(); // New line when complete
        }
    }

    pub fn finish(&self, message: &str) {
        if self.enabled {
            eprintln!("\r{} Complete!", message);
        }
    }
}

/// Utility functions for output formatting
pub mod utils {
    use super::*;

    /// Format file size in human-readable format
    pub fn format_file_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Format duration in human-readable format
    pub fn format_duration(duration: std::time::Duration) -> String {
        let total_ms = duration.as_millis();

        if total_ms < 1000 {
            format!("{}ms", total_ms)
        } else if total_ms < 60_000 {
            format!("{:.1}s", total_ms as f64 / 1000.0)
        } else {
            let minutes = total_ms / 60_000;
            let seconds = (total_ms % 60_000) as f64 / 1000.0;
            format!("{}m {:.1}s", minutes, seconds)
        }
    }

    /// Truncate text to fit within specified width
    pub fn truncate_text(text: &str, max_width: usize) -> String {
        if text.len() <= max_width {
            text.to_string()
        } else if max_width <= 3 {
            "...".to_string()
        } else {
            format!("{}...", &text[..max_width - 3])
        }
    }
}
