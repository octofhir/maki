//! Diagnostic types and utilities for FSH linting

use serde::{Deserialize, Serialize};
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
    /// Optional suggestions for fixing the issue
    pub suggestions: Vec<Suggestion>,
    /// Optional code snippet for context
    pub code_snippet: Option<String>,
}

/// Severity levels for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
}

/// Suggestion for fixing a diagnostic
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
    /// Description of the suggested fix
    pub message: String,
    /// The replacement text
    pub replacement: String,
    /// Location to apply the replacement
    pub location: Location,
    /// Whether this fix is safe to apply automatically
    pub is_safe: bool,
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
        }
    }

    /// Add a suggestion to this diagnostic
    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add a code snippet for context
    pub fn with_code_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.code_snippet = Some(snippet.into());
        self
    }

    /// Check if this diagnostic has any safe fixes
    pub fn has_safe_fixes(&self) -> bool {
        self.suggestions.iter().any(|s| s.is_safe)
    }

    /// Get all safe fixes for this diagnostic
    pub fn safe_fixes(&self) -> Vec<&Suggestion> {
        self.suggestions.iter().filter(|s| s.is_safe).collect()
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
        }
    }
}

impl Suggestion {
    /// Create a new suggestion
    pub fn new(
        message: impl Into<String>,
        replacement: impl Into<String>,
        location: Location,
        is_safe: bool,
    ) -> Self {
        Self {
            message: message.into(),
            replacement: replacement.into(),
            location,
            is_safe,
        }
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