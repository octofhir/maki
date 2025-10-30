//! Output formatting and reporting
//!
//! This module handles different output formats and provides rich reporting capabilities

use colored::*;
use maki_core::{
    DiagnosticRenderer, OutputFormat as CoreOutputFormat, Result, diagnostics::Diagnostic,
};

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
            // Use the new DiagnosticRenderer from core
            let renderer = if self.use_colors {
                DiagnosticRenderer::new()
            } else {
                DiagnosticRenderer::no_colors()
            };

            // Render diagnostics with summary (includes unsafe fix note if needed)
            println!("{}", renderer.render_diagnostics_with_summary(diagnostics));
        }

        // Print summary
        self.print_summary_human(summary)?;
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
        // Use DiagnosticRenderer for JSON output of diagnostics
        let renderer = DiagnosticRenderer::with_format(CoreOutputFormat::JsonPretty);
        let diagnostics_json: serde_json::Value =
            serde_json::from_str(&renderer.render_diagnostics(diagnostics)).map_err(|e| {
                maki_core::MakiError::ConfigError {
                    message: format!("Failed to parse diagnostics JSON: {e}"),
                }
            })?;

        let result = serde_json::json!({
            "files_checked": summary.files_checked,
            "issues": diagnostics_json,
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
                maki_core::MakiError::ConfigError {
                    message: format!("Failed to serialize JSON: {e}"),
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
                        "name": "maki",
                        "version": maki_core::VERSION,
                        "informationUri": "https://github.com/octofhir/maki"
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
                maki_core::MakiError::ConfigError {
                    message: format!("Failed to serialize SARIF: {e}"),
                }
            })?
        );

        Ok(())
    }

    fn print_compact_format(&self, summary: &LintSummary) -> Result<()> {
        if summary.has_issues() {
            println!(
                "maki: {} files, {} issues ({} errors, {} warnings)",
                summary.files_checked,
                summary.total_issues(),
                summary.errors,
                summary.warnings
            );
        } else {
            println!("maki: {} files checked, no issues", summary.files_checked);
        }

        if summary.fixes_applied > 0 {
            println!("maki: {} fixes applied", summary.fixes_applied);
        }

        Ok(())
    }

    fn print_github_format(&self, diagnostics: &[Diagnostic]) -> Result<()> {
        for diagnostic in diagnostics {
            let level = match diagnostic.severity {
                maki_core::diagnostics::Severity::Error => "error",
                maki_core::diagnostics::Severity::Warning => "warning",
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

    fn diagnostic_to_sarif(&self, diagnostic: &Diagnostic) -> serde_json::Value {
        let level = match diagnostic.severity {
            maki_core::diagnostics::Severity::Error => "error",
            maki_core::diagnostics::Severity::Warning => "warning",
            maki_core::diagnostics::Severity::Info => "note",
            maki_core::diagnostics::Severity::Hint => "note",
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
            eprintln!("\r{message} Complete!");
        }
    }
}

/// Utility functions for output formatting
pub mod utils {
    /// Format duration in human-readable format
    pub fn format_duration(duration: std::time::Duration) -> String {
        let total_ms = duration.as_millis();

        if total_ms < 1000 {
            format!("{total_ms}ms")
        } else if total_ms < 60_000 {
            format!("{:.1}s", total_ms as f64 / 1000.0)
        } else {
            let minutes = total_ms / 60_000;
            let seconds = (total_ms % 60_000) as f64 / 1000.0;
            format!("{minutes}m {seconds:.1}s")
        }
    }
}
