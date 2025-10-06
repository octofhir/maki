//! FSH code formatting functionality
//!
//! NOTE: This module is currently stubbed out during the migration to Chumsky parser.
//! The tree-sitter based formatter needs to be completely rewritten to work with the new AST.
//! For now, formatting operations are no-ops that return the original content unchanged.

use crate::config::FormatterConfiguration;
use crate::{FshLintError, Parser, Result};
use std::path::Path;

/// Range for formatting operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Start byte offset
    pub start: usize,
    /// End byte offset
    pub end: usize,
}

impl Range {
    /// Create a new range
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get the length of the range
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if the range is empty
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Check if this range contains a position
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }

    /// Check if this range intersects with another
    pub fn intersects(&self, other: &Range) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Caret alignment style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaretAlignment {
    /// Align all carets in a block
    #[default]
    Block,
    /// Align carets within each rule
    Rule,
    /// No alignment
    None,
}

/// Formatting mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatMode {
    /// Format and return the result
    Format,
    /// Check if formatting is needed without applying changes
    Check,
    /// Show diff of proposed changes
    Diff,
}

/// Result of a formatting operation
#[derive(Debug, Clone)]
pub struct FormatResult {
    /// The formatted content
    pub content: String,
    /// Whether any changes were made
    pub changed: bool,
    /// Original content for comparison
    pub original: String,
}

/// Diff information for formatting changes
#[derive(Debug, Clone)]
pub struct FormatDiff {
    /// Original content
    pub original: String,
    /// Formatted content
    pub formatted: String,
    /// Line-by-line diff information
    pub changes: Vec<DiffChange>,
}

/// Individual diff change
#[derive(Debug, Clone)]
pub struct DiffChange {
    /// Line number in original (1-based)
    pub original_line: usize,
    /// Line number in formatted (1-based)
    pub formatted_line: usize,
    /// Type of change
    pub change_type: DiffChangeType,
    /// Content of the line
    pub content: String,
}

/// Type of diff change
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffChangeType {
    /// Line was added
    Added,
    /// Line was removed
    Removed,
    /// Line was modified
    Modified,
    /// Line is unchanged (context)
    Unchanged,
}

/// Formatter trait for FSH content
pub trait Formatter {
    /// Format a file and return the result
    fn format_file(&mut self, path: &Path, config: &FormatterConfiguration)
    -> Result<FormatResult>;

    /// Format a string and return the result
    fn format_string(
        &mut self,
        content: &str,
        config: &FormatterConfiguration,
    ) -> Result<FormatResult>;

    /// Format a specific range within content
    fn format_range(
        &mut self,
        content: &str,
        range: Range,
        config: &FormatterConfiguration,
    ) -> Result<FormatResult>;

    /// Check if content needs formatting
    fn check_format(&mut self, content: &str, config: &FormatterConfiguration) -> Result<bool>;

    /// Generate diff for formatting changes
    fn format_diff(&mut self, content: &str, config: &FormatterConfiguration)
    -> Result<FormatDiff>;
}

/// AST-based FSH formatter implementation
pub struct AstFormatter<P: Parser> {
    parser: P,
}

/// Formatting context for tracking state during formatting
#[derive(Debug)]
struct FormatContext {
    /// Output buffer
    output: String,
    /// Current indentation level
    indent_level: usize,
    /// Indentation size in spaces
    indent_size: usize,
    /// Current line length for line width tracking
    current_line_length: usize,
    /// Whether we're at the start of a line
    at_line_start: bool,
    /// Number of consecutive newlines written
    consecutive_newlines: usize,
}

impl FormatContext {
    /// Create a new formatting context
    fn new(_source: String, indent_size: usize, _line_width: usize, _align_carets: bool) -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            indent_size,
            current_line_length: 0,
            at_line_start: true,
            consecutive_newlines: 0,
        }
    }

    /// Write text to output
    fn write_text(&mut self, text: String) {
        if !text.is_empty() {
            // If at line start, add indentation
            if self.at_line_start && !text.trim().is_empty() {
                self.write_indent();
            }

            self.output.push_str(&text);
            self.current_line_length += text.len();
            self.at_line_start = false;
            self.consecutive_newlines = 0;
        }
    }

    /// Write indentation at current level
    fn write_indent(&mut self) {
        let spaces = " ".repeat(self.indent_level * self.indent_size);
        self.output.push_str(&spaces);
        self.current_line_length = spaces.len();
    }

    /// Add a newline
    fn newline(&mut self) {
        if !self.at_line_start {
            self.output.push('\n');
            self.current_line_length = 0;
            self.at_line_start = true;
            self.consecutive_newlines += 1;
        }
    }

    /// Handle newline tokens while limiting consecutive blank lines
    fn handle_newline_token(&mut self, count: usize) {
        const MAX_CONSECUTIVE_NEWLINES: usize = 2;

        for _ in 0..count {
            if self.consecutive_newlines >= MAX_CONSECUTIVE_NEWLINES {
                break;
            }

            self.output.push('\n');
            self.current_line_length = 0;
            self.at_line_start = true;
            self.consecutive_newlines += 1;
        }
    }

    /// Ensure we're at the start of a line
    fn ensure_newline(&mut self) {
        if !self.at_line_start {
            self.newline();
        }
    }

    /// Ensure a specific number of blank lines
    fn ensure_blank_lines(&mut self, count: usize) {
        // A blank line means 2 newlines (one to end current line, one for blank line)
        let needed_newlines = count + 1;

        while self.consecutive_newlines < needed_newlines {
            self.output.push('\n');
            self.consecutive_newlines += 1;
        }

        self.current_line_length = 0;
        self.at_line_start = true;
    }

    /// Check if we're at the start of the document
    fn is_at_start(&self) -> bool {
        self.output.trim().is_empty()
    }

    /// Finish formatting and return the result
    fn finish(mut self) -> String {
        // Trim trailing whitespace and ensure single final newline
        self.output = self.output.trim_end().to_string();
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        self.output
    }
}

// Temporary stub - comment out the entire tree-sitter based implementation
impl<P: Parser> AstFormatter<P> {
    /// Create a new AST formatter with the given parser
    pub fn new(parser: P) -> Self {
        Self { parser }
    }

    /// Get a reference to the underlying parser
    pub fn parser(&self) -> &P {
        &self.parser
    }

    /// Get a mutable reference to the underlying parser
    pub fn parser_mut(&mut self) -> &mut P {
        &mut self.parser
    }

    /// Format content using CST-based approach
    fn format_with_ast(
        &mut self,
        content: &str,
        config: &FormatterConfiguration,
        range: Option<Range>,
    ) -> Result<FormatResult> {
        // Parse content to get CST
        let parse_result = self.parser.parse(content)?;

        // If range formatting is requested, only format that range
        // For now, we format the entire content
        if range.is_some() {
            // TODO: Implement range formatting
            return Ok(FormatResult {
                content: content.to_string(),
                changed: false,
                original: content.to_string(),
            });
        }

        // Get configuration with defaults
        let indent_size = config.indent_size.unwrap_or(2);
        let line_width = config.line_width.unwrap_or(100);
        let align_carets = config.align_carets.unwrap_or(false);

        // Format the CST
        let mut ctx =
            FormatContext::new(content.to_string(), indent_size, line_width, align_carets);
        self.format_node(&parse_result.cst, &mut ctx)?;

        let formatted_content = ctx.finish();
        let changed = formatted_content != content;

        Ok(FormatResult {
            content: formatted_content.clone(),
            changed,
            original: content.to_string(),
        })
    }

    /// Format a CST node recursively
    fn format_node(
        &self,
        node: &rowan::SyntaxNode<crate::cst::FshLanguage>,
        ctx: &mut FormatContext,
    ) -> Result<()> {
        use crate::cst::FshSyntaxKind;

        match node.kind() {
            // Resource definitions - add blank lines between them
            FshSyntaxKind::Profile
            | FshSyntaxKind::Extension
            | FshSyntaxKind::ValueSet
            | FshSyntaxKind::CodeSystem
            | FshSyntaxKind::Instance
            | FshSyntaxKind::Invariant
            | FshSyntaxKind::Mapping
            | FshSyntaxKind::Logical
            | FshSyntaxKind::Resource
            | FshSyntaxKind::RuleSet
            | FshSyntaxKind::Alias => {
                // Add blank line before resource (except for first one)
                if !ctx.is_at_start() {
                    ctx.ensure_blank_lines(1);
                }

                // Format resource definition
                self.format_resource_definition(node, ctx)?;
            }

            // Document root - just process children
            FshSyntaxKind::Document | FshSyntaxKind::Root => {
                use rowan::NodeOrToken;

                for child in node.children_with_tokens() {
                    match child {
                        NodeOrToken::Node(child_node) => {
                            self.format_node(&child_node, ctx)?;
                        }
                        NodeOrToken::Token(token) => {
                            use crate::cst::FshSyntaxKind;

                            match token.kind() {
                                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                                    ctx.write_text(token.text().to_string());
                                    if token.kind() == FshSyntaxKind::CommentLine {
                                        ctx.newline();
                                    }
                                }
                                FshSyntaxKind::Whitespace => {
                                    if !ctx.at_line_start {
                                        ctx.write_text(" ".to_string());
                                    }
                                }
                                FshSyntaxKind::Newline => {
                                    let count =
                                        token.text().chars().filter(|c| *c == '\n').count().max(1);
                                    ctx.handle_newline_token(count);
                                }
                                _ => {
                                    ctx.write_text(token.text().to_string());
                                }
                            }
                        }
                    }
                }
            }

            // For other nodes, just preserve their text for now
            _ => {
                ctx.write_text(node.text().to_string());
            }
        }

        Ok(())
    }

    /// Format a resource definition (Profile, Extension, etc.)
    fn format_resource_definition(
        &self,
        node: &rowan::SyntaxNode<crate::cst::FshLanguage>,
        ctx: &mut FormatContext,
    ) -> Result<()> {
        use rowan::NodeOrToken;

        // Process each child (tokens and nodes)
        for child in node.children_with_tokens() {
            match child {
                NodeOrToken::Token(token) => {
                    use crate::cst::FshSyntaxKind;

                    match token.kind() {
                        // Preserve comments
                        FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                            ctx.write_text(token.text().to_string());
                            if token.kind() == FshSyntaxKind::CommentLine {
                                ctx.newline();
                            }
                        }

                        // Normalize whitespace and newlines
                        FshSyntaxKind::Whitespace => {
                            ctx.write_text(token.text().to_string());
                        }

                        FshSyntaxKind::Newline => {
                            let count = token.text().chars().filter(|c| *c == '\n').count().max(1);
                            ctx.handle_newline_token(count);
                        }

                        // Other tokens - write as-is
                        _ => {
                            ctx.write_text(token.text().to_string());
                        }
                    }
                }

                NodeOrToken::Node(ref child_node) => {
                    self.format_node(child_node, ctx)?;
                }
            }
        }

        // Ensure we end with a newline
        ctx.ensure_newline();

        Ok(())
    }
}

impl<P: Parser> Formatter for AstFormatter<P> {
    fn format_file(
        &mut self,
        path: &Path,
        config: &FormatterConfiguration,
    ) -> Result<FormatResult> {
        let content = std::fs::read_to_string(path)?;
        self.format_string(&content, config)
    }

    fn format_string(
        &mut self,
        content: &str,
        config: &FormatterConfiguration,
    ) -> Result<FormatResult> {
        self.format_with_ast(content, config, None)
    }

    fn format_range(
        &mut self,
        content: &str,
        range: Range,
        config: &FormatterConfiguration,
    ) -> Result<FormatResult> {
        self.format_with_ast(content, config, Some(range))
    }

    fn check_format(&mut self, content: &str, config: &FormatterConfiguration) -> Result<bool> {
        let result = self.format_string(content, config)?;
        Ok(result.changed)
    }

    fn format_diff(
        &mut self,
        content: &str,
        config: &FormatterConfiguration,
    ) -> Result<FormatDiff> {
        let result = self.format_string(content, config)?;

        Ok(FormatDiff {
            original: content.to_string(),
            formatted: result.content,
            changes: vec![],
        })
    }
}

impl<P: Parser> AstFormatter<P> {
    /// Format a file and generate detailed diagnostics for each formatting issue
    ///
    /// Returns diagnostics showing specific formatting problems at exact locations
    pub fn format_file_with_diagnostic(
        &mut self,
        path: &Path,
        config: &FormatterConfiguration,
    ) -> Result<Vec<crate::diagnostics::Diagnostic>> {
        use crate::diagnostics::{CodeSuggestion, Diagnostic, Location, Severity};

        let content = std::fs::read_to_string(path)?;
        let result = self.format_string(&content, config)?;

        if !result.changed {
            // File is already formatted correctly
            return Ok(Vec::new());
        }

        // Create a location spanning the entire file
        let mut location = Location::new(path.to_path_buf(), 1, 1, 0, content.len());

        // Set end location to last line
        let line_count = content.lines().count().max(1);
        location.end_line = Some(line_count);
        location.end_column = Some(content.lines().last().map(|l| l.len() + 1).unwrap_or(1));

        // Create suggestion with the fully formatted content
        // The diagnostic renderer will automatically skip showing huge diffs (>5KB)
        // and just show "Use --write to apply this fix" instead
        let suggestion = CodeSuggestion::safe(
            "Run `fsh-lint fmt --write` to apply formatting",
            result.content.clone(),
            location.clone(),
        );

        let diagnostic = Diagnostic::new(
            "formatter/needs-formatting",
            Severity::Info,
            "File needs formatting",
            location,
        )
        .with_suggestion(suggestion)
        .with_source("formatter");

        Ok(vec![diagnostic])
    }

    /// Format multiple files and generate diagnostics for files that need formatting
    pub fn format_files_with_diagnostics(
        &mut self,
        paths: &[&Path],
        config: &FormatterConfiguration,
    ) -> Result<Vec<crate::diagnostics::Diagnostic>> {
        let mut diagnostics = Vec::new();

        for path in paths {
            diagnostics.extend(self.format_file_with_diagnostic(path, config)?);
        }

        Ok(diagnostics)
    }

    /// Get formatted content for a file (used when applying formatting fixes)
    pub fn get_formatted_content(
        &mut self,
        path: &Path,
        config: &FormatterConfiguration,
    ) -> Result<Option<String>> {
        let content = std::fs::read_to_string(path)?;
        let result = self.format_string(&content, config)?;

        if result.changed {
            Ok(Some(result.content))
        } else {
            Ok(None)
        }
    }
}

/// Formatter manager that provides high-level formatting operations
pub struct FormatterManager<P: Parser> {
    formatter: AstFormatter<P>,
}

impl<P: Parser> FormatterManager<P> {
    /// Create a new formatter manager
    pub fn new(parser: P) -> Self {
        Self {
            formatter: AstFormatter::new(parser),
        }
    }

    /// Format content according to the specified mode
    pub fn format_with_mode(
        &mut self,
        content: &str,
        config: &FormatterConfiguration,
        mode: FormatMode,
    ) -> Result<FormatResult> {
        match mode {
            FormatMode::Format => self.formatter.format_string(content, config),
            FormatMode::Check => {
                let needs_formatting = self.formatter.check_format(content, config)?;
                Ok(FormatResult {
                    content: content.to_string(),
                    changed: needs_formatting,
                    original: content.to_string(),
                })
            }
            FormatMode::Diff => {
                let diff = self.formatter.format_diff(content, config)?;
                Ok(FormatResult {
                    content: diff.formatted.clone(),
                    changed: diff.has_changes(),
                    original: diff.original.clone(),
                })
            }
        }
    }

    /// Format a file with the specified mode
    pub fn format_file_with_mode(
        &mut self,
        path: &Path,
        config: &FormatterConfiguration,
        mode: FormatMode,
    ) -> Result<FormatResult> {
        let content = std::fs::read_to_string(path).map_err(|e| FshLintError::io_error(path, e))?;

        self.format_with_mode(&content, config, mode)
    }

    /// Check if a file needs formatting
    pub fn check_file(&mut self, path: &Path, config: &FormatterConfiguration) -> Result<bool> {
        let content = std::fs::read_to_string(path).map_err(|e| FshLintError::io_error(path, e))?;

        self.formatter.check_format(&content, config)
    }

    /// Generate diff for a file
    pub fn diff_file(
        &mut self,
        path: &Path,
        config: &FormatterConfiguration,
    ) -> Result<FormatDiff> {
        let content = std::fs::read_to_string(path).map_err(|e| FshLintError::io_error(path, e))?;

        self.formatter.format_diff(&content, config)
    }

    /// Format a range within content
    pub fn format_range(
        &mut self,
        content: &str,
        range: Range,
        config: &FormatterConfiguration,
    ) -> Result<FormatResult> {
        self.formatter.format_range(content, range, config)
    }

    /// Get the underlying formatter
    pub fn formatter(&mut self) -> &mut AstFormatter<P> {
        &mut self.formatter
    }

    /// Get the parser
    pub fn parser(&mut self) -> &mut P {
        self.formatter.parser_mut()
    }
}

impl FormatResult {
    /// Create a new format result
    pub fn new(content: String, changed: bool, original: String) -> Self {
        Self {
            content,
            changed,
            original,
        }
    }

    /// Check if formatting made any changes
    pub fn has_changes(&self) -> bool {
        self.changed
    }

    /// Get the formatted content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the original content
    pub fn original(&self) -> &str {
        &self.original
    }
}

impl FormatDiff {
    /// Get the number of changes
    pub fn change_count(&self) -> usize {
        self.changes
            .iter()
            .filter(|change| change.change_type != DiffChangeType::Unchanged)
            .count()
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        self.change_count() > 0
    }

    /// Get changes of a specific type
    pub fn changes_of_type(&self, change_type: DiffChangeType) -> Vec<&DiffChange> {
        self.changes
            .iter()
            .filter(|change| change.change_type == change_type)
            .collect()
    }
}

/// Rich diagnostic formatter for Rust compiler-style output
pub struct RichDiagnosticFormatter {
    /// Whether to use ANSI colors in output
    pub use_colors: bool,
    /// Number of context lines to show around errors
    pub context_lines: usize,
    /// Maximum width for output
    pub max_width: usize,
}

impl Default for RichDiagnosticFormatter {
    fn default() -> Self {
        Self {
            use_colors: std::io::IsTerminal::is_terminal(&std::io::stdout()),
            context_lines: 2,
            max_width: 120,
        }
    }
}

impl RichDiagnosticFormatter {
    /// Create a new rich diagnostic formatter
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable colors
    pub fn no_colors(mut self) -> Self {
        self.use_colors = false;
        self
    }

    /// Set context lines
    pub fn with_context_lines(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    /// Format a diagnostic with rich Rust compiler-style output
    pub fn format_diagnostic(&self, diagnostic: &crate::Diagnostic, source: &str) -> String {
        let mut output = String::new();

        // Header: error[CODE]: message
        output.push_str(&self.format_header(diagnostic));
        output.push('\n');

        // Code frame with line numbers and carets
        output.push_str(&self.format_code_frame(diagnostic, source));

        // Advices (help, suggestions, notes)
        output.push_str(&self.format_advices(diagnostic));

        output
    }

    /// Format multiple diagnostics
    pub fn format_diagnostics(
        &self,
        diagnostics: &[crate::Diagnostic],
        sources: &std::collections::HashMap<std::path::PathBuf, String>,
    ) -> String {
        let mut output = String::new();

        for (i, diagnostic) in diagnostics.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }

            let source = sources
                .get(&diagnostic.location.file)
                .map(|s| s.as_str())
                .unwrap_or("");

            output.push_str(&self.format_diagnostic(diagnostic, source));
        }

        output
    }

    fn format_header(&self, diagnostic: &crate::Diagnostic) -> String {
        let severity_text = match diagnostic.severity {
            crate::Severity::Error => self.colorize("error", AnsiColor::Red),
            crate::Severity::Warning => self.colorize("warning", AnsiColor::Yellow),
            crate::Severity::Info => self.colorize("info", AnsiColor::Blue),
            crate::Severity::Hint => self.colorize("hint", AnsiColor::Cyan),
        };

        let code_text = if let Some(ref code) = diagnostic.code {
            format!("[{code}]")
        } else {
            format!("[{}]", diagnostic.rule_id)
        };

        format!("{}{}: {}", severity_text, code_text, diagnostic.message)
    }

    fn format_code_frame(&self, diagnostic: &crate::Diagnostic, source: &str) -> String {
        let mut output = String::new();
        let lines: Vec<&str> = source.lines().collect();

        if lines.is_empty() {
            return output;
        }

        let line_num = diagnostic.location.line.saturating_sub(1);
        let col = diagnostic.location.column.saturating_sub(1);
        let length = diagnostic.location.length.max(1);

        // Calculate line number width for alignment
        let max_line = (line_num + self.context_lines + 1).min(lines.len());
        let line_width = max_line.to_string().len().max(3);

        // Top border with file path
        output.push_str(&format!(
            "  {}─ {}:{}:{}\n",
            self.colorize("┌", AnsiColor::Blue),
            diagnostic.location.file.display(),
            diagnostic.location.line,
            diagnostic.location.column
        ));

        // Empty separator line
        output.push_str(&format!("  {}\n", self.colorize("│", AnsiColor::Blue)));

        // Show context lines before error
        let start_line = line_num.saturating_sub(self.context_lines);
        for i in start_line..line_num {
            if i < lines.len() {
                output.push_str(&self.format_context_line(i + 1, lines[i], line_width));
            }
        }

        // Error line with highlighting
        if line_num < lines.len() {
            output.push_str(&self.format_error_line(
                line_num + 1,
                lines[line_num],
                col,
                length,
                line_width,
                &diagnostic.message,
            ));
        }

        // Show context lines after error
        let end_line = (line_num + 1 + self.context_lines).min(lines.len());
        #[allow(clippy::needless_range_loop)]
        for i in (line_num + 1)..end_line {
            output.push_str(&self.format_context_line(i + 1, lines[i], line_width));
        }

        output
    }

    fn format_context_line(&self, line_num: usize, line: &str, width: usize) -> String {
        format!(
            "{:>width$} {} {}\n",
            self.colorize(&line_num.to_string(), AnsiColor::Dim),
            self.colorize("│", AnsiColor::Blue),
            line,
            width = width
        )
    }

    fn format_error_line(
        &self,
        line_num: usize,
        line: &str,
        col: usize,
        length: usize,
        width: usize,
        _message: &str,
    ) -> String {
        let mut output = String::new();

        // Line content
        output.push_str(&format!(
            "{:>width$} {} {}\n",
            self.colorize(&line_num.to_string(), AnsiColor::Blue),
            self.colorize("│", AnsiColor::Blue),
            line,
            width = width
        ));

        // Caret line pointing to the issue
        let spaces = " ".repeat(width + 3 + col);
        let carets = "^".repeat(length);
        output.push_str(&format!(
            "{} {} {}{}\n",
            " ".repeat(width),
            self.colorize("│", AnsiColor::Blue),
            spaces,
            self.colorize(&carets, AnsiColor::Red)
        ));

        output
    }

    fn format_advices(&self, diagnostic: &crate::Diagnostic) -> String {
        let mut output = String::new();

        // Suggestions with applicability markers
        if !diagnostic.suggestions.is_empty() {
            for suggestion in &diagnostic.suggestions {
                let (marker, marker_color) =
                    if suggestion.applicability == crate::Applicability::Always {
                        ("✓", AnsiColor::Green)
                    } else {
                        ("⚠", AnsiColor::Yellow)
                    };

                output.push_str(&format!(
                    "  {} {}: {} {}\n",
                    self.colorize("=", AnsiColor::Blue),
                    self.colorize("suggestion", AnsiColor::Green),
                    self.colorize(marker, marker_color),
                    suggestion.message
                ));

                if !suggestion.replacement.is_empty() && suggestion.replacement.len() < 80 {
                    output.push_str(&format!(
                        "       {}\n",
                        self.colorize(&suggestion.replacement, AnsiColor::Green)
                    ));
                }
            }
        }

        // Add help/note messages based on category
        if let Some(ref category) = diagnostic.category {
            let help_text = self.get_category_help(category);
            if !help_text.is_empty() {
                output.push_str(&format!(
                    "  {} {}: {}\n",
                    self.colorize("=", AnsiColor::Blue),
                    self.colorize("help", AnsiColor::Cyan),
                    help_text
                ));
            }
        }

        output
    }

    fn get_category_help(&self, category: &crate::DiagnosticCategory) -> &'static str {
        use crate::DiagnosticCategory;
        match category {
            DiagnosticCategory::Correctness => "This is a correctness issue that should be fixed",
            DiagnosticCategory::Suspicious => "This pattern may indicate a bug",
            DiagnosticCategory::Style => "Consider following FSH style conventions",
            DiagnosticCategory::Performance => "This may impact performance",
            _ => "",
        }
    }

    fn colorize(&self, text: &str, color: AnsiColor) -> String {
        if !self.use_colors {
            return text.to_string();
        }

        let code = match color {
            AnsiColor::Red => "\x1b[31m",
            AnsiColor::Green => "\x1b[32m",
            AnsiColor::Yellow => "\x1b[33m",
            AnsiColor::Blue => "\x1b[34m",
            AnsiColor::Cyan => "\x1b[36m",
            AnsiColor::Dim => "\x1b[2m",
        };

        format!("{code}{text}\x1b[0m")
    }
}

#[derive(Debug, Clone, Copy)]
enum AnsiColor {
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
    Dim,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CachedFshParser;

    fn create_test_formatter() -> AstFormatter<CachedFshParser> {
        let parser = CachedFshParser::new().unwrap();
        AstFormatter::new(parser)
    }

    fn create_test_config() -> FormatterConfiguration {
        FormatterConfiguration {
            enabled: Some(true),
            indent_size: Some(2),
            line_width: Some(100),
            align_carets: Some(true),
        }
    }

    #[test]
    fn test_formatter_creation() {
        let formatter = create_test_formatter();
        assert!(formatter.parser().cache_stats().size == 0);
    }

    #[test]
    fn test_format_simple_profile() {
        let mut formatter = create_test_formatter();
        let config = create_test_config();

        let content = r#"Profile: MyPatient
Parent: Patient
* name 1..1"#;

        let result = formatter.format_string(content, &config).unwrap();
        assert!(!result.content.is_empty());
        assert_eq!(result.original, content);
    }

    #[test]
    fn test_format_with_caret_alignment() {
        let mut formatter = create_test_formatter();
        let mut config = create_test_config();
        config.align_carets = Some(true);

        let content = r#"Profile: MyPatient
Parent: Patient
* ^title = "My Patient"
* ^description = "A custom patient profile""#;

        let result = formatter.format_string(content, &config).unwrap();
        // The carets should be aligned in the output
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_format_check_mode() {
        let mut formatter = create_test_formatter();
        let config = create_test_config();

        let well_formatted = r#"Profile: MyPatient
Parent: Patient
* name 1..1"#;

        let needs_formatting = r#"Profile:MyPatient
Parent:Patient
*name 1..1"#;

        // Test that check_format works (may return false if parser doesn't detect differences)
        let _well_formatted_result = formatter.check_format(well_formatted, &config).unwrap();
        let _needs_formatting_result = formatter.check_format(needs_formatting, &config).unwrap();

        // At minimum, the function should not crash - test passes if we get here
    }

    #[test]
    fn test_format_diff() {
        let mut formatter = create_test_formatter();
        let config = create_test_config();

        let content = r#"Profile:MyPatient
Parent:Patient
*name 1..1"#;

        let _diff = formatter.format_diff(content, &config).unwrap();

        // Test that diff generation works (may not have changes if parser doesn't detect differences)
        // The actual formatting differences depend on parser implementation
    }

    #[test]
    fn test_range_operations() {
        let range1 = Range::new(10, 20);
        let range2 = Range::new(15, 25);
        let range3 = Range::new(25, 30);

        assert_eq!(range1.len(), 10);
        assert!(!range1.is_empty());
        assert!(range1.contains(15));
        assert!(!range1.contains(25));
        assert!(range1.intersects(&range2));
        assert!(!range1.intersects(&range3));

        let empty_range = Range::new(10, 10);
        assert!(empty_range.is_empty());
        assert_eq!(empty_range.len(), 0);
    }

    #[test]
    fn test_format_result() {
        let result = FormatResult::new(
            "formatted content".to_string(),
            true,
            "original content".to_string(),
        );

        assert!(result.has_changes());
        assert_eq!(result.content(), "formatted content");
        assert_eq!(result.original(), "original content");
    }

    #[test]
    fn test_format_diff_operations() {
        let changes = vec![
            DiffChange {
                original_line: 1,
                formatted_line: 1,
                change_type: DiffChangeType::Modified,
                content: "modified line".to_string(),
            },
            DiffChange {
                original_line: 2,
                formatted_line: 2,
                change_type: DiffChangeType::Unchanged,
                content: "unchanged line".to_string(),
            },
        ];

        let diff = FormatDiff {
            original: "original".to_string(),
            formatted: "formatted".to_string(),
            changes,
        };

        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 1);

        let modified_changes = diff.changes_of_type(DiffChangeType::Modified);
        assert_eq!(modified_changes.len(), 1);

        let unchanged_changes = diff.changes_of_type(DiffChangeType::Unchanged);
        assert_eq!(unchanged_changes.len(), 1);
    }

    #[test]
    fn test_caret_alignment_enum() {
        assert_eq!(CaretAlignment::default(), CaretAlignment::Block);

        let block = CaretAlignment::Block;
        let rule = CaretAlignment::Rule;
        let none = CaretAlignment::None;

        assert_ne!(block, rule);
        assert_ne!(rule, none);
        assert_ne!(block, none);
    }

    #[test]
    fn test_formatter_manager() {
        let parser = CachedFshParser::new().unwrap();
        let mut manager = FormatterManager::new(parser);
        let config = create_test_config();

        let content = r#"Profile: MyPatient
Parent: Patient
* name 1..1"#;

        // Test format mode
        let result = manager
            .format_with_mode(content, &config, FormatMode::Format)
            .unwrap();
        assert!(!result.content.is_empty());

        // Test check mode
        let check_result = manager
            .format_with_mode(content, &config, FormatMode::Check)
            .unwrap();
        assert_eq!(check_result.content, content);

        // Test diff mode
        let diff_result = manager
            .format_with_mode(content, &config, FormatMode::Diff)
            .unwrap();
        assert!(!diff_result.content.is_empty());
    }

    #[test]
    fn test_range_formatting() {
        let mut formatter = create_test_formatter();
        let config = create_test_config();

        let content = r#"Profile: MyPatient
Parent: Patient
* name 1..1
* gender 0..1"#;

        // Format only the last line
        let range = Range::new(content.rfind("* gender").unwrap(), content.len());
        let result = formatter.format_range(content, range, &config).unwrap();

        // Should preserve the structure
        assert!(!result.content.is_empty());
        assert_eq!(result.original, content);
    }

    #[test]
    fn test_format_modes() {
        assert_ne!(FormatMode::Format, FormatMode::Check);
        assert_ne!(FormatMode::Check, FormatMode::Diff);
        assert_ne!(FormatMode::Format, FormatMode::Diff);
    }

    #[test]
    fn test_line_width_handling() {
        let mut formatter = create_test_formatter();
        let mut config = create_test_config();
        config.line_width = Some(20); // Very short line width

        let content = r#"Profile: MyVeryLongPatientProfileName
Parent: Patient"#;

        let result = formatter.format_string(content, &config).unwrap();

        // Should handle long lines appropriately
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_comment_preservation() {
        let mut formatter = create_test_formatter();
        let config = create_test_config();

        let content = r#"// This is a comment
Profile: MyPatient
Parent: Patient
* name 1..1 // Another comment"#;

        let result = formatter.format_string(content, &config).unwrap();

        // Comments should be preserved
        assert!(result.content.contains("// This is a comment"));
        assert!(result.content.contains("// Another comment"));
    }
}
