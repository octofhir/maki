//! Diagnostic renderer with rich terminal output

use super::{Applicability, CodeSuggestion, Diagnostic, Location, Severity};
use crate::console::{Color, Console};
use serde_json;
use std::fs;

/// Output format for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text with colors and formatting
    Text,
    /// JSON format for programmatic consumption
    Json,
    /// JSON with pretty-printing
    JsonPretty,
}

/// Diagnostic renderer with rich formatting
pub struct DiagnosticRenderer {
    console: Console,
    output_format: OutputFormat,
}

struct SuggestionRender {
    diff: Option<String>,
    notes: Vec<String>,
}

impl SuggestionRender {
    fn diff(diff: String) -> Self {
        Self {
            diff: Some(diff),
            notes: Vec::new(),
        }
    }

    fn notes<T: Into<String>>(notes: impl IntoIterator<Item = T>) -> Self {
        Self {
            diff: None,
            notes: notes.into_iter().map(Into::into).collect(),
        }
    }
}

impl DiagnosticRenderer {
    /// Create a new diagnostic renderer with automatic terminal detection (text output)
    pub fn new() -> Self {
        Self {
            console: Console::new(),
            output_format: OutputFormat::Text,
        }
    }

    /// Create a renderer with colors disabled
    pub fn no_colors() -> Self {
        Self {
            console: Console::no_colors(),
            output_format: OutputFormat::Text,
        }
    }

    /// Create a renderer with specific output format
    pub fn with_format(format: OutputFormat) -> Self {
        let console = match format {
            OutputFormat::Json | OutputFormat::JsonPretty => Console::no_colors(),
            OutputFormat::Text => Console::new(),
        };

        Self {
            console,
            output_format: format,
        }
    }

    /// Set the output format
    pub fn set_format(&mut self, format: OutputFormat) {
        self.output_format = format;
        if matches!(format, OutputFormat::Json | OutputFormat::JsonPretty) {
            self.console = Console::no_colors();
        }
    }

    /// Render a diagnostic with the configured output format
    pub fn render(&self, diagnostic: &Diagnostic) -> String {
        match self.output_format {
            OutputFormat::Text => self.render_text(diagnostic),
            OutputFormat::Json => self.render_json(&[diagnostic.clone()], false),
            OutputFormat::JsonPretty => self.render_json(&[diagnostic.clone()], true),
        }
    }

    /// Render a diagnostic with text formatting
    fn render_text(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();

        // Header: severity[rule_id]: message
        output.push_str(&self.render_header(diagnostic));
        output.push('\n');

        // Code frame with context
        match self.render_code_frame(diagnostic) {
            Some(frame) => {
                output.push_str(&frame);
                output.push('\n');
            }
            None => {
                output.push('\n');
                output.push_str(&self.render_location_line(&diagnostic.location));
                output.push('\n');
            }
        }

        // Suggestions with applicability markers
        if !diagnostic.suggestions.is_empty() {
            output.push_str(&self.render_suggestions(diagnostic));
        }

        output
    }

    /// Render diagnostic(s) as JSON
    fn render_json(&self, diagnostics: &[Diagnostic], pretty: bool) -> String {
        if pretty {
            serde_json::to_string_pretty(diagnostics)
                .unwrap_or_else(|e| format!("{{\"error\": \"Failed to serialize: {e}\"}}"))
        } else {
            serde_json::to_string(diagnostics)
                .unwrap_or_else(|e| format!("{{\"error\": \"Failed to serialize: {e}\"}}"))
        }
    }

    /// Render the diagnostic header
    fn render_header(&self, diagnostic: &Diagnostic) -> String {
        let severity_color = match diagnostic.severity {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
            Severity::Info => Color::Blue,
            Severity::Hint => Color::Dim,
        };

        let severity_text = self.console.colorize(
            &format!("{:?}", diagnostic.severity).to_lowercase(),
            severity_color,
        );

        let rule_id = self
            .console
            .colorize(&format!("[{}]", diagnostic.rule_id), Color::Dim);

        format!(
            "{}{}: {}",
            severity_text,
            rule_id,
            self.console.colorize(&diagnostic.message, Color::Bold)
        )
    }

    /// Render a code frame showing the error in context
    fn render_code_frame(&self, diagnostic: &Diagnostic) -> Option<String> {
        let source = fs::read_to_string(&diagnostic.location.file).ok()?;
        let lines: Vec<&str> = source.lines().collect();
        let total_lines = if lines.is_empty() { 1 } else { lines.len() };
        let last_line_len = lines.last().map(|l| l.len()).unwrap_or(0);

        if self.location_spans_entire_file(
            &diagnostic.location,
            source.len(),
            total_lines,
            last_line_len,
        ) {
            return None;
        }

        let error_line = diagnostic.location.line;
        let error_col = diagnostic.location.column;
        let error_len = diagnostic.location.length;

        // Show context lines (±2)
        let start_line = error_line.saturating_sub(2).max(1);
        let end_line = (error_line + 2).min(lines.len());

        let gutter_width = format!("{}", end_line.max(total_lines)).len();

        let mut frame = String::new();
        frame.push('\n');

        // File location header
        frame.push_str(&format!(
            "  {}─[{}:{}:{}]\n",
            self.console.colorize("┌", Color::Blue),
            diagnostic.location.file.display(),
            error_line,
            error_col
        ));
        frame.push_str(&format!("  {}\n", self.console.colorize("│", Color::Blue)));

        // Determine color based on severity
        let highlight_color = match diagnostic.severity {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
            Severity::Info => Color::Blue,
            Severity::Hint => Color::Dim,
        };

        for line_num in start_line..=end_line {
            let is_error_line = line_num == error_line;
            let line_content = lines.get(line_num - 1)?;

            // Line marker and number
            if is_error_line {
                frame.push_str(&self.console.colorize(">", highlight_color));
                frame.push(' ');
            } else {
                frame.push_str("  ");
            }

            frame.push_str(
                &self
                    .console
                    .colorize(&format!("{line_num:>gutter_width$}"), Color::Dim),
            );

            frame.push_str(&self.console.colorize(" │ ", Color::Dim));

            // Line content
            if is_error_line {
                frame.push_str(&self.highlight_error_in_line(
                    line_content,
                    error_col,
                    error_len,
                    highlight_color,
                ));
            } else {
                frame.push_str(line_content);
            }
            frame.push('\n');

            // Caret markers under error line
            if is_error_line {
                frame.push_str("  ");
                frame.push_str(&" ".repeat(gutter_width));
                frame.push_str(&self.console.colorize(" │ ", Color::Dim));
                frame.push_str(&" ".repeat(error_col.saturating_sub(1)));

                let carets = "^".repeat(error_len.max(1));
                frame.push_str(&self.console.colorize(&carets, highlight_color));

                frame.push('\n');
            }
        }

        Some(frame)
    }

    /// Highlight the error portion within a line
    fn highlight_error_in_line(&self, line: &str, col: usize, len: usize, color: Color) -> String {
        if col == 0 || col > line.len() {
            return line.to_string();
        }

        let col_idx = col.saturating_sub(1);
        let end_idx = (col_idx + len).min(line.len());

        let before = &line[..col_idx];
        let error_part = &line[col_idx..end_idx];
        let after = &line[end_idx..];

        format!(
            "{}{}{}",
            before,
            self.console.colorize(error_part, color),
            after
        )
    }

    /// Render suggestions with applicability markers
    fn render_suggestions(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();

        for suggestion in &diagnostic.suggestions {
            output.push('\n');

            // Format: "i Safe fix: message" or "i Unsafe fix: message"
            let label = match suggestion.applicability {
                Applicability::Always => "Safe fix",
                Applicability::MaybeIncorrect => "Unsafe fix",
            };

            let label_color = match suggestion.applicability {
                Applicability::Always => Color::Green,
                Applicability::MaybeIncorrect => Color::Yellow,
            };

            output.push_str(&format!(
                "  {} {}: {}\n",
                self.console.colorize("i", Color::Blue),
                self.console.colorize(label, label_color),
                suggestion.message
            ));

            // Render diff output when appropriate; otherwise fall back to guidance
            if suggestion.replacement.is_empty() {
                continue;
            }

            let rendered = self.render_suggestion_diff(suggestion);

            if let Some(diff) = rendered.diff {
                output.push_str(&diff);
            }

            for note in rendered.notes {
                let arrow = self.console.colorize("→", Color::Dim);
                let highlighted = self.highlight_cli_flags(&note);
                output.push_str(&format!("      {arrow} {highlighted}\n"));
            }
        }

        output
    }

    /// Render a diff for a suggestion
    fn render_suggestion_diff(&self, suggestion: &CodeSuggestion) -> SuggestionRender {
        const MAX_DIFF_SIZE: usize = 5000; // Avoid overwhelming output with very large diffs

        let source = match fs::read_to_string(&suggestion.location.file) {
            Ok(s) => s,
            Err(_) => {
                return SuggestionRender::notes([
                    "Unable to display diff (failed to read file)",
                    "Use --write to apply this fix",
                ]);
            }
        };

        let lines: Vec<&str> = source.lines().collect();
        let total_lines = if lines.is_empty() { 1 } else { lines.len() };
        let last_line_len = lines.last().map(|l| l.len()).unwrap_or(0);

        if self.location_spans_entire_file(
            &suggestion.location,
            source.len(),
            total_lines,
            last_line_len,
        ) {
            return SuggestionRender::notes([
                "Formatting diff suppressed for large change; run `maki fmt --diff` to review",
                "Use --write to apply this fix",
            ]);
        }

        if suggestion.replacement.len() >= MAX_DIFF_SIZE {
            return SuggestionRender::notes([
                "Diff too large to display; run `maki fmt --diff` for details",
                "Use --write to apply this fix",
            ]);
        }

        let line_num = suggestion.location.line;

        if line_num == 0 || line_num > lines.len() {
            return SuggestionRender::notes([
                "Unable to display diff for this change",
                "Use --write to apply this fix",
            ]);
        }

        let original_line = lines[line_num - 1];
        let mut output = String::new();

        output.push('\n');

        // Calculate the portion of the line being replaced
        let start_col = suggestion.location.column.saturating_sub(1); // 0-indexed
        let end_col = suggestion
            .location
            .end_column
            .unwrap_or(original_line.len())
            .saturating_sub(1);

        // Build the modified text by replacing the specified range
        let prefix = &original_line[..start_col.min(original_line.len())];
        let modified_text = if start_col < original_line.len() && end_col <= original_line.len() {
            let suffix = &original_line[end_col..];
            format!("{}{}{}", prefix, suggestion.replacement, suffix)
        } else {
            // If location is invalid, just append
            format!("{}{}", prefix, suggestion.replacement)
        };

        // Check if the replacement contains newlines (multi-line insertion)
        if suggestion.replacement.contains('\n') {
            // Multi-line diff rendering
            // Show deletion (original line)
            output.push_str(&format!(
                "    {} │ ",
                self.console.colorize(&format!("{line_num:>4}"), Color::Dim)
            ));
            output.push_str(&self.console.colorize("- ", Color::Red));
            output.push_str(&self.console.colorize(original_line, Color::Red));
            output.push('\n');

            // Show each inserted line with its own line number
            let new_lines: Vec<&str> = modified_text.lines().collect();
            for (i, new_line) in new_lines.iter().enumerate() {
                output.push_str(&format!(
                    "    {} │ ",
                    self.console
                        .colorize(&format!("{:>4}", line_num + i), Color::Dim)
                ));
                output.push_str(&self.console.colorize("+ ", Color::Green));
                output.push_str(&self.console.colorize(new_line, Color::Green));
                output.push('\n');
            }
        } else {
            // Single-line replacement
            // Show deletion (original line)
            output.push_str(&format!(
                "    {} │ ",
                self.console.colorize(&format!("{line_num:>4}"), Color::Dim)
            ));
            output.push_str(&self.console.colorize("- ", Color::Red));
            output.push_str(&self.console.colorize(original_line, Color::Red));
            output.push('\n');

            // Show addition (modified line)
            output.push_str(&format!(
                "    {} │ ",
                self.console.colorize(&format!("{line_num:>4}"), Color::Dim)
            ));
            output.push_str(&self.console.colorize("+ ", Color::Green));
            output.push_str(&self.console.colorize(&modified_text, Color::Green));
            output.push('\n');
        }

        SuggestionRender::diff(output)
    }

    fn highlight_cli_flags(&self, text: &str) -> String {
        let mut result = text.to_string();

        for flag in ["--write", "--diff"] {
            if result.contains(flag) {
                let colored = self.console.colorize(flag, Color::Bold);
                result = result.replace(flag, &colored);
            }
        }

        result
    }

    fn render_location_line(&self, location: &Location) -> String {
        format!(
            "  {} {}",
            self.console.colorize("→", Color::Blue),
            self.console.colorize(&format!("{location}"), Color::Dim)
        )
    }

    fn location_spans_entire_file(
        &self,
        location: &Location,
        source_len: usize,
        total_lines: usize,
        last_line_len: usize,
    ) -> bool {
        if source_len == 0 {
            return location.line <= 1
                && location.column <= 1
                && location.offset == 0
                && location.length == 0;
        }

        let covers_start = location.line <= 1 && location.column <= 1 && location.offset == 0;

        let covers_end = match (location.end_line, location.end_column) {
            (Some(end_line), Some(end_col)) => {
                end_line >= total_lines && end_col >= last_line_len.saturating_add(1)
            }
            _ => location.length >= source_len,
        };

        covers_start && covers_end && location.length >= source_len.saturating_sub(1)
    }

    /// Render multiple diagnostics
    pub fn render_diagnostics(&self, diagnostics: &[Diagnostic]) -> String {
        match self.output_format {
            OutputFormat::Text => self.render_diagnostics_text(diagnostics),
            OutputFormat::Json => self.render_json(diagnostics, false),
            OutputFormat::JsonPretty => self.render_json(diagnostics, true),
        }
    }

    /// Render multiple diagnostics as text
    fn render_diagnostics_text(&self, diagnostics: &[Diagnostic]) -> String {
        let mut output = String::new();

        for (i, diagnostic) in diagnostics.iter().enumerate() {
            if i > 0 {
                output.push('\n');
                output.push_str(
                    &self
                        .console
                        .colorize(&"─".repeat(self.console.max_width().min(80)), Color::Dim),
                );
                output.push('\n');
                output.push('\n');
            }
            output.push_str(&self.render_text(diagnostic));
        }

        output
    }

    /// Render diagnostics with summary (including unsafe fix note if needed)
    pub fn render_diagnostics_with_summary(&self, diagnostics: &[Diagnostic]) -> String {
        match self.output_format {
            OutputFormat::Text => {
                let mut output = self.render_diagnostics(diagnostics);

                // Check if there are any unsafe fixes
                let has_unsafe_fixes = diagnostics.iter().any(|d| {
                    d.suggestions
                        .iter()
                        .any(|s| s.applicability == Applicability::MaybeIncorrect)
                });

                if has_unsafe_fixes {
                    output.push('\n');
                    output.push('\n');
                    output.push_str(&self.render_unsafe_fix_note());
                }

                output
            }
            OutputFormat::Json | OutputFormat::JsonPretty => {
                // For JSON, just output the diagnostics
                self.render_diagnostics(diagnostics)
            }
        }
    }

    /// Render the note about using --unsafe flag
    fn render_unsafe_fix_note(&self) -> String {
        let mut output = String::new();

        output.push_str(&self.console.colorize("ℹ ", Color::Blue));
        output.push_str("Some fixes are unsafe and require the ");
        output.push_str(&self.console.colorize("--unsafe", Color::Bold));
        output.push_str(" flag to apply.\n");

        output.push_str(&self.console.colorize("  ", Color::Blue));
        output.push_str("Use ");
        output.push_str(
            &self
                .console
                .colorize("maki lint --write --unsafe", Color::Bold),
        );
        output.push_str(" to apply unsafe fixes.");

        output
    }
}

impl Default for DiagnosticRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Applicability, CodeSuggestion, Diagnostic, Location, Severity};
    use std::io::Write;

    use tempfile::NamedTempFile;

    fn create_test_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_render_header() {
        let renderer = DiagnosticRenderer::no_colors();
        let diagnostic = Diagnostic {
            rule_id: "test/rule".to_string(),
            severity: Severity::Error,
            message: "Test message".to_string(),
            location: Location::default(),
            suggestions: vec![],
            code_snippet: None,
            code: None,
            source: None,
            category: None,
        };

        let output = renderer.render_header(&diagnostic);
        assert!(output.contains("error"));
        assert!(output.contains("[test/rule]"));
        assert!(output.contains("Test message"));
    }

    #[test]
    fn test_render_code_frame() {
        let file = create_test_file("Line 1\nLine 2 error here\nLine 3\n");
        let renderer = DiagnosticRenderer::no_colors();

        let diagnostic = Diagnostic {
            rule_id: "test/rule".to_string(),
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: Location {
                file: file.path().to_path_buf(),
                line: 2,
                column: 8,
                end_line: Some(2),
                end_column: Some(13),
                offset: 15,
                length: 5,
                span: Some((15, 20)),
            },
            suggestions: vec![],
            code_snippet: None,
            code: None,
            source: None,
            category: None,
        };

        let frame = renderer.render_code_frame(&diagnostic);
        assert!(frame.is_some());

        let frame_text = frame.unwrap();
        assert!(frame_text.contains("Line 1"));
        assert!(frame_text.contains("Line 2 error here"));
        assert!(frame_text.contains("Line 3"));
        assert!(frame_text.contains("^^^^^"));
    }

    #[test]
    fn test_render_suggestions() {
        let renderer = DiagnosticRenderer::no_colors();
        let file = create_test_file("old content");

        let diagnostic = Diagnostic {
            rule_id: "test/rule".to_string(),
            severity: Severity::Warning,
            message: "Test warning".to_string(),
            location: Location {
                file: file.path().to_path_buf(),
                line: 1,
                column: 1,
                end_line: Some(1),
                end_column: Some(11),
                offset: 0,
                length: 11,
                span: Some((0, 11)),
            },
            suggestions: vec![CodeSuggestion {
                message: "Replace with new content".to_string(),
                replacement: "new content".to_string(),
                location: Location {
                    file: file.path().to_path_buf(),
                    line: 1,
                    column: 1,
                    end_line: Some(1),
                    end_column: Some(11),
                    offset: 0,
                    length: 11,
                    span: Some((0, 11)),
                },
                applicability: Applicability::Always,
                labels: vec![],
            }],
            code_snippet: None,
            code: None,
            source: None,
            category: None,
        };

        let output = renderer.render_suggestions(&diagnostic);
        assert!(output.contains("Safe fix"));
        assert!(output.contains("Replace with new content"));
    }

    #[test]
    fn test_render_full_diagnostic() {
        let file = create_test_file("Profile: TestProfile\nTitle: \"Test\"");
        let renderer = DiagnosticRenderer::no_colors();

        let diagnostic = Diagnostic {
            rule_id: "style/naming".to_string(),
            severity: Severity::Warning,
            message: "Profile name should use PascalCase".to_string(),
            location: Location {
                file: file.path().to_path_buf(),
                line: 1,
                column: 10,
                end_line: Some(1),
                end_column: Some(21),
                offset: 9,
                length: 11,
                span: Some((9, 20)),
            },
            suggestions: vec![],
            code_snippet: None,
            code: None,
            source: None,
            category: None,
        };

        let output = renderer.render(&diagnostic);
        assert!(output.contains("warning"));
        assert!(output.contains("[style/naming]"));
        assert!(output.contains("PascalCase"));
        assert!(output.contains("Profile: TestProfile"));
    }

    #[test]
    fn test_json_output() {
        let file = create_test_file("Profile: Test");
        let renderer = DiagnosticRenderer::with_format(OutputFormat::Json);

        let diagnostic = Diagnostic {
            rule_id: "test/rule".to_string(),
            severity: Severity::Error,
            message: "Test message".to_string(),
            location: Location {
                file: file.path().to_path_buf(),
                line: 1,
                column: 1,
                end_line: Some(1),
                end_column: Some(5),
                offset: 0,
                length: 5,
                span: Some((0, 5)),
            },
            suggestions: vec![],
            code_snippet: None,
            code: None,
            source: None,
            category: None,
        };

        let output = renderer.render(&diagnostic);

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.is_array());

        let diag = &parsed[0];
        assert_eq!(diag["rule_id"], "test/rule");
        assert_eq!(diag["severity"], "Error");
        assert_eq!(diag["message"], "Test message");
    }

    #[test]
    fn test_json_pretty_output() {
        let _file = create_test_file("test");
        let renderer = DiagnosticRenderer::with_format(OutputFormat::JsonPretty);

        let diagnostic = Diagnostic {
            rule_id: "test/rule".to_string(),
            severity: Severity::Warning,
            message: "Test".to_string(),
            location: Location::default(),
            suggestions: vec![],
            code_snippet: None,
            code: None,
            source: None,
            category: None,
        };

        let output = renderer.render(&diagnostic);

        // Pretty JSON should have indentation
        assert!(output.contains("  "));
        assert!(serde_json::from_str::<serde_json::Value>(&output).is_ok());
    }

    #[test]
    fn test_multiple_diagnostics_json() {
        let renderer = DiagnosticRenderer::with_format(OutputFormat::Json);

        let diagnostics = vec![
            Diagnostic {
                rule_id: "rule1".to_string(),
                severity: Severity::Error,
                message: "Error 1".to_string(),
                location: Location::default(),
                suggestions: vec![],
                code_snippet: None,
                code: None,
                source: None,
                category: None,
            },
            Diagnostic {
                rule_id: "rule2".to_string(),
                severity: Severity::Warning,
                message: "Warning 1".to_string(),
                location: Location::default(),
                suggestions: vec![],
                code_snippet: None,
                code: None,
                source: None,
                category: None,
            },
        ];

        let output = renderer.render_diagnostics(&diagnostics);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[0]["rule_id"], "rule1");
        assert_eq!(parsed[1]["rule_id"], "rule2");
    }
}
