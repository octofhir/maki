//! Diagnostic types and utilities for FSH linting
//!
//! Provides diagnostics with:
//! - Precise code positioning with line/column information
//! - Code suggestions with applicability levels (safe vs unsafe)
//! - Contextual advice system
//! - Multiple output formats (human, JSON, SARIF)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Represents a diagnostic message from linting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Unique identifier for the rule that generated this diagnostic
    pub rule_id: String,
    /// Severity level of the diagnostic
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// Location in the source file
    pub location: Location,
    /// Code suggestions for fixing the issue
    pub suggestions: Vec<CodeSuggestion>,
    /// Optional code snippet for context
    pub code_snippet: Option<String>,
    /// Optional error code
    pub code: Option<String>,
    /// Optional source of the diagnostic (e.g., "parser", "rule-engine")
    pub source: Option<String>,
    /// Category of the diagnostic
    pub category: Option<DiagnosticCategory>,
}

/// Severity levels for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Informational messages
    Info,
    /// Hints for improvements
    Hint,
    /// Warnings that should be addressed
    Warning,
    /// Errors that must be fixed
    Error,
}

/// Location information for diagnostics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// File path
    pub file: PathBuf,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Optional end position for ranges
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    /// Byte offset in the file
    pub offset: usize,
    /// Length of the span
    pub length: usize,
    /// Optional span information (start, end)
    pub span: Option<(usize, usize)>,
}

/// Indicates how a tool should manage this suggestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Applicability {
    /// The suggestion is definitely correct and should be applied automatically.
    /// Used for: formatting, whitespace, adding semicolons, obvious typos.
    Always,

    /// The suggestion may be correct but is uncertain and requires review.
    /// Requires --unsafe flag to apply.
    /// Used for: semantic changes, refactoring, removing code, renaming.
    MaybeIncorrect,
}

impl fmt::Display for Applicability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Applicability::Always => write!(f, "safe"),
            Applicability::MaybeIncorrect => write!(f, "unsafe"),
        }
    }
}

/// A code suggestion that can be automatically applied
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeSuggestion {
    /// Description of the suggested fix
    pub message: String,

    /// The replacement text to apply
    pub replacement: String,

    /// Location to apply the replacement
    pub location: Location,

    /// When this suggestion should be applied
    pub applicability: Applicability,

    /// Additional labels/highlights for context
    pub labels: Vec<Label>,
}

/// A label highlighting a specific region of code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// The location to highlight
    pub location: Location,

    /// Optional message for this label
    pub message: Option<String>,
}

/// Trait for types that can provide contextual advice in diagnostics
pub trait Advices {
    /// Record advices into the provided visitor
    fn record(&self, visitor: &mut dyn Visit) -> io::Result<()>;
}

/// The Visit trait collects advices from a diagnostic
pub trait Visit {
    /// Prints a single log entry with the provided category and text
    fn record_log(&mut self, category: LogCategory, text: &dyn fmt::Display) -> io::Result<()>;

    /// Prints an unordered list of items
    fn record_list(&mut self, list: &[&dyn fmt::Display]) -> io::Result<()>;

    /// Prints a code frame outlining the provided source location
    fn record_frame(&mut self, location: &Location) -> io::Result<()>;

    /// Prints a code suggestion with applicability marker
    fn record_suggestion(&mut self, suggestion: &CodeSuggestion) -> io::Result<()>;

    /// Prints a group of advices under a common title
    fn record_group(&mut self, title: &dyn fmt::Display, advice: &dyn Advices) -> io::Result<()>;
}

/// The category for a log advice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogCategory {
    /// No specific category
    None,
    /// Informational message
    Info,
    /// Warning message
    Warn,
    /// Error message
    Error,
}

/// Utility type implementing Advices that emits a single log advice
#[derive(Debug)]
pub struct LogAdvice<T> {
    pub category: LogCategory,
    pub text: T,
}

impl<T: fmt::Display> Advices for LogAdvice<T> {
    fn record(&self, visitor: &mut dyn Visit) -> io::Result<()> {
        visitor.record_log(self.category, &self.text)
    }
}

/// Utility advice that prints a list of items
#[derive(Debug)]
pub struct ListAdvice<T> {
    pub items: Vec<T>,
}

impl<T: fmt::Display> Advices for ListAdvice<T> {
    fn record(&self, visitor: &mut dyn Visit) -> io::Result<()> {
        if self.items.is_empty() {
            visitor.record_log(LogCategory::Warn, &"The list is empty.")
        } else {
            let display_items: Vec<_> = self
                .items
                .iter()
                .map(|item| item as &dyn fmt::Display)
                .collect();
            visitor.record_list(&display_items)
        }
    }
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(
        rule_id: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        location: Location,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity,
            message: message.into(),
            location,
            suggestions: Vec::new(),
            code_snippet: None,
            code: None,
            source: None,
            category: None,
        }
    }

    /// Add a code suggestion to this diagnostic
    pub fn with_suggestion(mut self, suggestion: CodeSuggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add a code snippet for context
    pub fn with_code_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.code_snippet = Some(snippet.into());
        self
    }

    /// Set the category for this diagnostic
    pub fn with_category(mut self, category: DiagnosticCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Set the source for this diagnostic
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the error code for this diagnostic
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Check if this diagnostic has any safe fixes
    pub fn has_safe_fixes(&self) -> bool {
        self.suggestions
            .iter()
            .any(|s| s.applicability == Applicability::Always)
    }

    /// Get all safe fixes for this diagnostic
    pub fn safe_fixes(&self) -> Vec<&CodeSuggestion> {
        self.suggestions
            .iter()
            .filter(|s| s.applicability == Applicability::Always)
            .collect()
    }
}

impl Default for Location {
    fn default() -> Self {
        Self {
            file: PathBuf::new(),
            line: 0,
            column: 0,
            end_line: None,
            end_column: None,
            offset: 0,
            length: 0,
            span: None,
        }
    }
}

impl Location {
    /// Create a new location
    pub fn new(file: PathBuf, line: usize, column: usize, offset: usize, length: usize) -> Self {
        Self {
            file,
            line,
            column,
            end_line: None,
            end_column: None,
            offset,
            length,
            span: None,
        }
    }

    /// Create a location with end position
    pub fn with_end(
        file: PathBuf,
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
        offset: usize,
        length: usize,
    ) -> Self {
        Self {
            file,
            line,
            column,
            end_line: Some(end_line),
            end_column: Some(end_column),
            offset,
            length,
            span: None,
        }
    }

    /// Create a location with span information
    pub fn with_span(
        file: PathBuf,
        line: usize,
        column: usize,
        offset: usize,
        length: usize,
        span: (usize, usize),
    ) -> Self {
        Self {
            file,
            line,
            column,
            end_line: None,
            end_column: None,
            offset,
            length,
            span: Some(span),
        }
    }
}

impl CodeSuggestion {
    /// Create a new code suggestion
    pub fn new(
        message: impl Into<String>,
        replacement: impl Into<String>,
        location: Location,
        applicability: Applicability,
    ) -> Self {
        Self {
            message: message.into(),
            replacement: replacement.into(),
            location,
            applicability,
            labels: vec![],
        }
    }

    /// Create a safe (always applicable) suggestion
    pub fn safe(
        message: impl Into<String>,
        replacement: impl Into<String>,
        location: Location,
    ) -> Self {
        Self::new(message, replacement, location, Applicability::Always)
    }

    /// Create an unsafe (maybe incorrect) suggestion
    pub fn unsafe_fix(
        message: impl Into<String>,
        replacement: impl Into<String>,
        location: Location,
    ) -> Self {
        Self::new(
            message,
            replacement,
            location,
            Applicability::MaybeIncorrect,
        )
    }

    /// Add a label to this suggestion
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Hint => write!(f, "hint"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file.display(), self.line, self.column)
    }
}

/// Trait for collecting and managing diagnostics
pub trait DiagnosticCollector {
    /// Collect a diagnostic
    fn collect(&mut self, diagnostic: Diagnostic);

    /// Collect multiple diagnostics
    fn collect_all(&mut self, diagnostics: Vec<Diagnostic>) {
        for diagnostic in diagnostics {
            self.collect(diagnostic);
        }
    }

    /// Get all collected diagnostics
    fn diagnostics(&self) -> &[Diagnostic];

    /// Filter diagnostics by minimum severity level
    fn filter_by_severity(&self, min_severity: Severity) -> Vec<&Diagnostic>;

    /// Group diagnostics by file
    fn group_by_file(&self) -> HashMap<PathBuf, Vec<&Diagnostic>>;

    /// Group diagnostics by rule ID
    fn group_by_rule(&self) -> HashMap<String, Vec<&Diagnostic>>;

    /// Group diagnostics by severity
    fn group_by_severity(&self) -> HashMap<Severity, Vec<&Diagnostic>>;

    /// Get diagnostics for a specific file
    fn diagnostics_for_file(&self, file: &PathBuf) -> Vec<&Diagnostic>;

    /// Get count of diagnostics by severity
    fn count_by_severity(&self) -> HashMap<Severity, usize>;

    /// Check if there are any errors
    fn has_errors(&self) -> bool;

    /// Check if there are any warnings
    fn has_warnings(&self) -> bool;

    /// Get total count of diagnostics
    fn total_count(&self) -> usize;

    /// Clear all collected diagnostics
    fn clear(&mut self);
}

/// Default implementation of DiagnosticCollector
#[derive(Debug, Clone, Default)]
pub struct DefaultDiagnosticCollector {
    diagnostics: Vec<Diagnostic>,
}

impl DefaultDiagnosticCollector {
    /// Create a new diagnostic collector
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Create a collector with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            diagnostics: Vec::with_capacity(capacity),
        }
    }

    /// Sort diagnostics by location (file, then line, then column)
    pub fn sort_by_location(&mut self) {
        self.diagnostics.sort_by(|a, b| {
            a.location
                .file
                .cmp(&b.location.file)
                .then_with(|| a.location.line.cmp(&b.location.line))
                .then_with(|| a.location.column.cmp(&b.location.column))
        });
    }

    /// Sort diagnostics by severity (errors first, then warnings, etc.)
    pub fn sort_by_severity(&mut self) {
        self.diagnostics.sort_by(|a, b| {
            // Reverse order so errors come first
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.location.file.cmp(&b.location.file))
                .then_with(|| a.location.line.cmp(&b.location.line))
                .then_with(|| a.location.column.cmp(&b.location.column))
        });
    }

    /// Remove duplicate diagnostics
    pub fn deduplicate(&mut self) {
        self.diagnostics.sort_by(|a, b| {
            a.rule_id
                .cmp(&b.rule_id)
                .then_with(|| a.location.file.cmp(&b.location.file))
                .then_with(|| a.location.line.cmp(&b.location.line))
                .then_with(|| a.location.column.cmp(&b.location.column))
        });
        self.diagnostics.dedup_by(|a, b| {
            a.rule_id == b.rule_id
                && a.location.file == b.location.file
                && a.location.line == b.location.line
                && a.location.column == b.location.column
        });
    }
}

impl DiagnosticCollector for DefaultDiagnosticCollector {
    fn collect(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    fn filter_by_severity(&self, min_severity: Severity) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity >= min_severity)
            .collect()
    }

    fn group_by_file(&self) -> HashMap<PathBuf, Vec<&Diagnostic>> {
        let mut groups = HashMap::new();
        for diagnostic in &self.diagnostics {
            groups
                .entry(diagnostic.location.file.clone())
                .or_insert_with(Vec::new)
                .push(diagnostic);
        }
        groups
    }

    fn group_by_rule(&self) -> HashMap<String, Vec<&Diagnostic>> {
        let mut groups = HashMap::new();
        for diagnostic in &self.diagnostics {
            groups
                .entry(diagnostic.rule_id.clone())
                .or_insert_with(Vec::new)
                .push(diagnostic);
        }
        groups
    }

    fn group_by_severity(&self) -> HashMap<Severity, Vec<&Diagnostic>> {
        let mut groups = HashMap::new();
        for diagnostic in &self.diagnostics {
            groups
                .entry(diagnostic.severity)
                .or_insert_with(Vec::new)
                .push(diagnostic);
        }
        groups
    }

    fn diagnostics_for_file(&self, file: &PathBuf) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| &d.location.file == file)
            .collect()
    }

    fn count_by_severity(&self) -> HashMap<Severity, usize> {
        let mut counts = HashMap::new();
        for diagnostic in &self.diagnostics {
            *counts.entry(diagnostic.severity).or_insert(0) += 1;
        }
        counts
    }

    fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Warning)
    }

    fn total_count(&self) -> usize {
        self.diagnostics.len()
    }

    fn clear(&mut self) {
        self.diagnostics.clear();
    }
}

/// Category for diagnostic classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    /// Correctness issues such as syntax or semantic violations
    Correctness,
    /// Suspicious patterns that may indicate bugs
    Suspicious,
    /// Excessive complexity warnings
    Complexity,
    /// Performance-related issues
    Performance,
    /// Style and formatting issues
    Style,
    /// Experimental or incubating diagnostics
    Nursery,
    /// Accessibility-related concerns
    Accessibility,
    /// Documentation or guidance improvements
    Documentation,
    /// Security-related issues
    Security,
    /// Compatibility issues with tooling or platforms
    Compatibility,
    /// Custom category
    Custom(String),
}

impl DiagnosticCategory {
    /// Convert category to kebab-case slug
    pub fn slug(&self) -> &str {
        match self {
            DiagnosticCategory::Correctness => "correctness",
            DiagnosticCategory::Suspicious => "suspicious",
            DiagnosticCategory::Complexity => "complexity",
            DiagnosticCategory::Performance => "performance",
            DiagnosticCategory::Style => "style",
            DiagnosticCategory::Nursery => "nursery",
            DiagnosticCategory::Accessibility => "accessibility",
            DiagnosticCategory::Documentation => "documentation",
            DiagnosticCategory::Security => "security",
            DiagnosticCategory::Compatibility => "compatibility",
            DiagnosticCategory::Custom(name) => name.as_str(),
        }
    }
}

impl std::fmt::Display for DiagnosticCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.slug())
    }
}

/// Formatter for diagnostic output
pub struct DiagnosticFormatter {
    /// Whether to include colors in output
    pub use_colors: bool,
    /// Whether to include code snippets
    pub include_snippets: bool,
    /// Number of context lines to show around the error
    pub context_lines: usize,
    /// Maximum width for formatting
    pub max_width: usize,
}

impl Default for DiagnosticFormatter {
    fn default() -> Self {
        Self {
            use_colors: true,
            include_snippets: true,
            context_lines: 2,
            max_width: 120,
        }
    }
}

impl DiagnosticFormatter {
    /// Create a new formatter
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a formatter without colors
    pub fn no_colors() -> Self {
        Self {
            use_colors: false,
            ..Self::default()
        }
    }

    /// Create a formatter without code snippets
    pub fn no_snippets() -> Self {
        Self {
            include_snippets: false,
            ..Self::default()
        }
    }

    /// Format a single diagnostic
    pub fn format_diagnostic(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();

        // Format the header with location and severity
        output.push_str(&self.format_header(diagnostic));
        output.push('\n');

        // Add the main message
        output.push_str(&self.format_message(diagnostic));
        output.push('\n');

        // Add code snippet if available and requested
        if self.include_snippets {
            if let Some(snippet) = &diagnostic.code_snippet {
                output.push('\n');
                output.push_str(&self.format_code_snippet(snippet, diagnostic));
                output.push('\n');
            } else if let Ok(snippet) = self.extract_code_snippet(diagnostic) {
                output.push('\n');
                output.push_str(&self.format_code_snippet(&snippet, diagnostic));
                output.push('\n');
            }
        }

        // Add suggestions if available
        if !diagnostic.suggestions.is_empty() {
            output.push('\n');
            output.push_str(&self.format_suggestions(&diagnostic.suggestions));
        }

        output
    }

    /// Format multiple diagnostics
    pub fn format_diagnostics(&self, diagnostics: &[Diagnostic]) -> String {
        let mut output = String::new();

        for (i, diagnostic) in diagnostics.iter().enumerate() {
            if i > 0 {
                output.push('\n');
                output.push_str(&"-".repeat(self.max_width.min(80)));
                output.push('\n');
                output.push('\n');
            }
            output.push_str(&self.format_diagnostic(diagnostic));
        }

        output
    }

    /// Format diagnostics grouped by file
    pub fn format_by_file(&self, diagnostics: &HashMap<PathBuf, Vec<&Diagnostic>>) -> String {
        let mut output = String::new();
        let mut files: Vec<_> = diagnostics.keys().collect();
        files.sort();

        for (file_idx, file) in files.iter().enumerate() {
            if file_idx > 0 {
                output.push('\n');
                output.push_str(&"=".repeat(self.max_width.min(80)));
                output.push('\n');
                output.push('\n');
            }

            // File header
            output.push_str(&self.colorize(&format!("File: {}", file.display()), Color::Bold));
            output.push('\n');
            output.push('\n');

            let file_diagnostics = &diagnostics[*file];
            for (diag_idx, diagnostic) in file_diagnostics.iter().enumerate() {
                if diag_idx > 0 {
                    output.push('\n');
                }
                output.push_str(&self.format_diagnostic(diagnostic));
            }
        }

        output
    }

    /// Format a summary of diagnostics
    pub fn format_summary(&self, collector: &dyn DiagnosticCollector) -> String {
        let counts = collector.count_by_severity();
        let total = collector.total_count();

        if total == 0 {
            return self.colorize("No issues found", Color::Green);
        }

        let mut parts = Vec::new();

        if let Some(&error_count) = counts.get(&Severity::Error) {
            if error_count > 0 {
                parts.push(self.colorize(
                    &format!(
                        "{} error{}",
                        error_count,
                        if error_count == 1 { "" } else { "s" }
                    ),
                    Color::Red,
                ));
            }
        }

        if let Some(&warning_count) = counts.get(&Severity::Warning) {
            if warning_count > 0 {
                parts.push(self.colorize(
                    &format!(
                        "{} warning{}",
                        warning_count,
                        if warning_count == 1 { "" } else { "s" }
                    ),
                    Color::Yellow,
                ));
            }
        }

        if let Some(&info_count) = counts.get(&Severity::Info) {
            if info_count > 0 {
                parts.push(self.colorize(&format!("{info_count} info"), Color::Blue));
            }
        }

        if let Some(&hint_count) = counts.get(&Severity::Hint) {
            if hint_count > 0 {
                parts.push(self.colorize(
                    &format!(
                        "{} hint{}",
                        hint_count,
                        if hint_count == 1 { "" } else { "s" }
                    ),
                    Color::Cyan,
                ));
            }
        }

        format!(
            "Found {} ({})",
            self.colorize(
                &format!("{} issue{}", total, if total == 1 { "" } else { "s" }),
                Color::Bold
            ),
            parts.join(", ")
        )
    }

    fn format_header(&self, diagnostic: &Diagnostic) -> String {
        let severity_color = match diagnostic.severity {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
            Severity::Info => Color::Blue,
            Severity::Hint => Color::Cyan,
        };

        let severity_str = self.colorize(&diagnostic.severity.to_string(), severity_color);
        let location_str = self.colorize(&diagnostic.location.to_string(), Color::Bold);

        format!("{severity_str}: {location_str}")
    }

    fn format_message(&self, diagnostic: &Diagnostic) -> String {
        let mut message = format!("  {}", diagnostic.message);

        if let Some(ref code) = diagnostic.code {
            message.push_str(&format!(" [{}]", self.colorize(code, Color::Dim)));
        }

        if let Some(ref category) = diagnostic.category {
            message.push_str(&format!(
                " ({})",
                self.colorize(&category.to_string(), Color::Dim)
            ));
        }

        message
    }

    fn format_code_snippet(&self, snippet: &str, diagnostic: &Diagnostic) -> String {
        let lines: Vec<&str> = snippet.lines().collect();
        let mut output = String::new();

        let line_num_width = (diagnostic.location.line + lines.len())
            .to_string()
            .len()
            .max(3);

        for (i, line) in lines.iter().enumerate() {
            let line_num = diagnostic.location.line + i;
            let is_error_line = i == 0; // Assume first line is the error line

            let line_num_str = format!("{line_num:line_num_width$}");
            let line_num_colored = if is_error_line {
                self.colorize(&line_num_str, Color::Red)
            } else {
                self.colorize(&line_num_str, Color::Dim)
            };

            output.push_str(&format!("  {line_num_colored} | {line}\n"));

            // Add error indicator for the error line
            if is_error_line && diagnostic.location.length > 0 {
                let spaces =
                    " ".repeat(line_num_width + 3 + diagnostic.location.column.saturating_sub(1));
                let carets = "^".repeat(diagnostic.location.length.min(line.len()));
                output.push_str(&format!(
                    "  {}{}\n",
                    spaces,
                    self.colorize(&carets, Color::Red)
                ));
            }
        }

        output
    }

    fn format_suggestions(&self, suggestions: &[CodeSuggestion]) -> String {
        let mut output = String::new();

        output.push_str(&self.colorize("Suggestions:", Color::Bold));
        output.push('\n');

        for (i, suggestion) in suggestions.iter().enumerate() {
            let prefix = if suggestion.applicability == Applicability::Always {
                self.colorize("  ✓", Color::Green)
            } else {
                self.colorize("  ⚠", Color::Yellow)
            };

            output.push_str(&format!("{} {}\n", prefix, suggestion.message));

            if !suggestion.replacement.is_empty() {
                let replacement_preview = if suggestion.replacement.len() > 50 {
                    format!("{}...", &suggestion.replacement[..47])
                } else {
                    suggestion.replacement.clone()
                };
                output.push_str(&format!(
                    "    Replace with: {}\n",
                    self.colorize(&replacement_preview, Color::Green)
                ));
            }

            if i < suggestions.len() - 1 {
                output.push('\n');
            }
        }

        output
    }

    fn extract_code_snippet(&self, diagnostic: &Diagnostic) -> Result<String, std::io::Error> {
        let content = fs::read_to_string(&diagnostic.location.file)?;
        let lines: Vec<&str> = content.lines().collect();

        let start_line = diagnostic
            .location
            .line
            .saturating_sub(self.context_lines + 1);
        let end_line = (diagnostic.location.line + self.context_lines).min(lines.len());

        let snippet_lines = &lines[start_line..end_line];
        Ok(snippet_lines.join("\n"))
    }

    fn colorize(&self, text: &str, color: Color) -> String {
        if !self.use_colors {
            return text.to_string();
        }

        let color_code = match color {
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Cyan => "\x1b[36m",
            Color::Bold => "\x1b[1m",
            Color::Dim => "\x1b[2m",
        };

        format!("{color_code}{text}\x1b[0m")
    }
}

#[derive(Debug, Clone, Copy)]
enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
    Bold,
    Dim,
}

/// Trait for formatting diagnostics in different output formats
pub trait DiagnosticOutputFormatter {
    /// Format diagnostics as human-readable text
    fn format_human(&self, diagnostics: &[Diagnostic]) -> String;

    /// Format diagnostics as JSON
    fn format_json(&self, diagnostics: &[Diagnostic]) -> Result<String, serde_json::Error>;

    /// Format diagnostics as SARIF (Static Analysis Results Interchange Format)
    fn format_sarif(&self, diagnostics: &[Diagnostic]) -> Result<String, serde_json::Error>;
}

/// Default implementation of DiagnosticOutputFormatter
pub struct DefaultOutputFormatter {
    formatter: DiagnosticFormatter,
}

impl DefaultOutputFormatter {
    pub fn new(formatter: DiagnosticFormatter) -> Self {
        Self { formatter }
    }
}

impl DiagnosticOutputFormatter for DefaultOutputFormatter {
    fn format_human(&self, diagnostics: &[Diagnostic]) -> String {
        self.formatter.format_diagnostics(diagnostics)
    }

    fn format_json(&self, diagnostics: &[Diagnostic]) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(diagnostics)
    }

    fn format_sarif(&self, diagnostics: &[Diagnostic]) -> Result<String, serde_json::Error> {
        // Basic SARIF format structure
        let sarif_report = SarifReport {
            version: "2.1.0".to_string(),
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "fsh-lint".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                },
                results: diagnostics
                    .iter()
                    .map(|d| SarifResult {
                        rule_id: d.rule_id.clone(),
                        level: match d.severity {
                            Severity::Error => "error".to_string(),
                            Severity::Warning => "warning".to_string(),
                            Severity::Info => "note".to_string(),
                            Severity::Hint => "note".to_string(),
                        },
                        message: SarifMessage {
                            text: d.message.clone(),
                        },
                        locations: vec![SarifLocation {
                            physical_location: SarifPhysicalLocation {
                                artifact_location: SarifArtifactLocation {
                                    uri: d.location.file.to_string_lossy().to_string(),
                                },
                                region: SarifRegion {
                                    start_line: d.location.line,
                                    start_column: d.location.column,
                                    end_line: d.location.end_line,
                                    end_column: d.location.end_column,
                                },
                            },
                        }],
                    })
                    .collect(),
            }],
        };

        serde_json::to_string_pretty(&sarif_report)
    }
}

// SARIF format structures
#[derive(Serialize)]
struct SarifReport {
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: usize,
    #[serde(rename = "startColumn")]
    start_column: usize,
    #[serde(rename = "endLine", skip_serializing_if = "Option::is_none")]
    end_line: Option<usize>,
    #[serde(rename = "endColumn", skip_serializing_if = "Option::is_none")]
    end_column: Option<usize>,
}

/// Source map for efficient byte offset to line/column conversion
///
/// This uses a precomputed table of line start offsets for O(log n) lookup
/// performance, which is critical for compiler-grade diagnostic quality.
#[derive(Debug, Clone)]
pub struct SourceMap {
    /// Cumulative byte offsets for each line start (line 0, line 1, ...)
    line_starts: Vec<usize>,
}

impl SourceMap {
    /// Create a source map from source text
    ///
    /// Time complexity: O(n) where n is source length
    /// Space complexity: O(lines) - typically much smaller than source
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0]; // Line 1 starts at offset 0

        for (idx, ch) in source.char_indices() {
            if ch == '\n' {
                // Next line starts after this newline
                line_starts.push(idx + 1);
            }
        }

        Self { line_starts }
    }

    /// Convert byte offset to (line, column) position
    ///
    /// Returns 1-based line and column numbers as per LSP convention.
    /// Time complexity: O(log n) using binary search
    ///
    /// # Arguments
    /// * `offset` - Byte offset in source (0-based)
    /// * `source` - Original source text (needed for UTF-8 column calculation)
    ///
    /// # Returns
    /// `(line, column)` tuple, both 1-based
    pub fn offset_to_position(&self, offset: usize, source: &str) -> (usize, usize) {
        // Binary search for the line containing this offset
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,                    // Exact match - offset is at line start
            Err(idx) => idx.saturating_sub(1), // Offset is within line idx-1
        };

        let line = line_idx + 1; // Convert to 1-based

        // Calculate column by counting characters from line start
        let line_start = self.line_starts[line_idx];
        let line_text = &source[line_start..offset.min(source.len())];

        // Count Unicode characters, not bytes, for proper column number
        let column = line_text.chars().count() + 1; // 1-based

        (line, column)
    }

    /// Convert a span (byte range) to full location information
    ///
    /// # Returns
    /// `(start_line, start_col, end_line, end_col)` - all 1-based
    pub fn span_to_location(
        &self,
        span: &std::ops::Range<usize>,
        source: &str,
    ) -> (usize, usize, usize, usize) {
        let (start_line, start_col) = self.offset_to_position(span.start, source);
        let (end_line, end_col) = self.offset_to_position(span.end, source);
        (start_line, start_col, end_line, end_col)
    }

    /// Create a Location struct from a span
    pub fn span_to_diagnostic_location(
        &self,
        span: &std::ops::Range<usize>,
        source: &str,
        file_path: &std::path::PathBuf,
    ) -> Location {
        let (line, column, end_line, end_column) = self.span_to_location(span, source);

        Location {
            file: file_path.clone(),
            line,
            column,
            end_line: Some(end_line),
            end_column: Some(end_column),
            offset: span.start,
            length: span.end - span.start,
            span: Some((span.start, span.end)),
        }
    }

    /// Convert a CST node to a diagnostic location
    pub fn node_to_diagnostic_location(
        &self,
        node: &crate::cst::FshSyntaxNode,
        source: &str,
        file_path: &std::path::PathBuf,
    ) -> Location {
        let range = node.text_range();
        let span = usize::from(range.start())..usize::from(range.end());
        self.span_to_diagnostic_location(&span, source, file_path)
    }

    /// Convert a CST token to a diagnostic location
    pub fn token_to_diagnostic_location(
        &self,
        token: &crate::cst::FshSyntaxToken,
        source: &str,
        file_path: &std::path::PathBuf,
    ) -> Location {
        let range = token.text_range();
        let span = usize::from(range.start())..usize::from(range.end());
        self.span_to_diagnostic_location(&span, source, file_path)
    }
}

#[cfg(test)]
mod source_map_tests {
    use super::*;

    #[test]
    fn test_single_line() {
        let source = "Hello, World!";
        let map = SourceMap::new(source);

        assert_eq!(map.offset_to_position(0, source), (1, 1)); // 'H'
        assert_eq!(map.offset_to_position(7, source), (1, 8)); // 'W'
        assert_eq!(map.offset_to_position(12, source), (1, 13)); // '!'
    }

    #[test]
    fn test_multiple_lines() {
        let source = "Profile: Test\nParent: Patient\nTitle: \"Test Profile\"";
        let map = SourceMap::new(source);

        // Line 1: "Profile: Test\n"
        assert_eq!(map.offset_to_position(0, source), (1, 1)); // 'P' in Profile
        assert_eq!(map.offset_to_position(9, source), (1, 10)); // 'T' in Test

        // Line 2: "Parent: Patient\n" starts at offset 14
        assert_eq!(map.offset_to_position(14, source), (2, 1)); // 'P' in Parent
        assert_eq!(map.offset_to_position(22, source), (2, 9)); // 'P' in Patient

        // Line 3: "Title: \"Test Profile\"" starts at offset 30
        assert_eq!(map.offset_to_position(30, source), (3, 1)); // 'T' in Title
    }

    #[test]
    fn test_unicode() {
        let source = "Profile: 日本語\nTitle: \"Test\"";
        let map = SourceMap::new(source);

        // Unicode characters count as single columns
        assert_eq!(map.offset_to_position(9, source), (1, 10)); // First Japanese char
    }

    #[test]
    fn test_span_to_location() {
        let source = "Profile: Test\nParent: Patient";
        let map = SourceMap::new(source);

        // Span covering "Test" (offset 9-13)
        let (start_line, start_col, end_line, end_col) = map.span_to_location(&(9..13), source);
        assert_eq!(start_line, 1);
        assert_eq!(start_col, 10);
        assert_eq!(end_line, 1);
        assert_eq!(end_col, 14);
    }
}
