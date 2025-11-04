//! Trivia handling for FSH CST
//!
//! This module provides utilities for preserving and manipulating trivia
//! (whitespace, comments, newlines) in the FSH concrete syntax tree.
//!
//! Trivia preservation is essential for:
//! - Lossless source-to-source transformations
//! - Maintaining code formatting and comments during refactoring
//! - Round-trip parsing validation
//!
//! # Example
//!
//! ```rust,ignore
//! use maki_core::cst::trivia::{TriviaCollector, TriviaPreserver};
//!
//! let source = "Profile: MyPatient // Comment\n  Parent: Patient";
//! let (cst, _, _) = parse_fsh(source);
//! 
//! let collector = TriviaCollector::new();
//! let trivia = collector.collect_trivia(&cst);
//! 
//! // Trivia includes the comment and whitespace
//! assert!(trivia.has_comments());
//! assert!(trivia.has_whitespace());
//! ```

use super::{FshSyntaxNode, FshSyntaxToken, FshSyntaxKind};
use rowan::NodeOrToken;
use std::collections::HashMap;

/// Represents trivia information for a CST node
#[derive(Debug, Clone, PartialEq)]
pub struct TriviaInfo {
    /// Leading trivia (before the node)
    pub leading: Vec<TriviaToken>,
    /// Trailing trivia (after the node, until next line)
    pub trailing: Vec<TriviaToken>,
    /// Internal trivia (within the node)
    pub internal: Vec<TriviaToken>,
}

impl TriviaInfo {
    /// Create empty trivia info
    pub fn empty() -> Self {
        Self {
            leading: Vec::new(),
            trailing: Vec::new(),
            internal: Vec::new(),
        }
    }

    /// Check if this has any trivia
    pub fn is_empty(&self) -> bool {
        self.leading.is_empty() && self.trailing.is_empty() && self.internal.is_empty()
    }

    /// Check if this has comments
    pub fn has_comments(&self) -> bool {
        self.leading.iter().any(|t| t.is_comment())
            || self.trailing.iter().any(|t| t.is_comment())
            || self.internal.iter().any(|t| t.is_comment())
    }

    /// Check if this has whitespace
    pub fn has_whitespace(&self) -> bool {
        self.leading.iter().any(|t| t.is_whitespace())
            || self.trailing.iter().any(|t| t.is_whitespace())
            || self.internal.iter().any(|t| t.is_whitespace())
    }

    /// Get all comments
    pub fn comments(&self) -> Vec<&TriviaToken> {
        self.leading
            .iter()
            .chain(self.trailing.iter())
            .chain(self.internal.iter())
            .filter(|t| t.is_comment())
            .collect()
    }

    /// Get all whitespace tokens
    pub fn whitespace(&self) -> Vec<&TriviaToken> {
        self.leading
            .iter()
            .chain(self.trailing.iter())
            .chain(self.internal.iter())
            .filter(|t| t.is_whitespace())
            .collect()
    }
}

/// Represents a single trivia token
#[derive(Debug, Clone, PartialEq)]
pub struct TriviaToken {
    /// The kind of trivia
    pub kind: FshSyntaxKind,
    /// The text content
    pub text: String,
    /// Position in the original source
    pub range: rowan::TextRange,
}

impl TriviaToken {
    /// Create a new trivia token
    pub fn new(kind: FshSyntaxKind, text: String, range: rowan::TextRange) -> Self {
        Self { kind, text, range }
    }

    /// Create from a syntax token
    pub fn from_token(token: &FshSyntaxToken) -> Self {
        Self {
            kind: token.kind(),
            text: token.text().to_string(),
            range: token.text_range(),
        }
    }

    /// Check if this is a comment
    pub fn is_comment(&self) -> bool {
        matches!(self.kind, FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock)
    }

    /// Check if this is whitespace
    pub fn is_whitespace(&self) -> bool {
        matches!(self.kind, FshSyntaxKind::Whitespace | FshSyntaxKind::Newline)
    }

    /// Check if this is a newline
    pub fn is_newline(&self) -> bool {
        self.kind == FshSyntaxKind::Newline
    }

    /// Get the comment content (without // or /* */)
    pub fn comment_content(&self) -> Option<String> {
        if !self.is_comment() {
            return None;
        }

        match self.kind {
            FshSyntaxKind::CommentLine => {
                // Remove // prefix
                let content = self.text.trim_start_matches("//").trim_start();
                Some(content.to_string())
            }
            FshSyntaxKind::CommentBlock => {
                // Remove /* */ wrapper
                let content = self.text
                    .trim_start_matches("/*")
                    .trim_end_matches("*/")
                    .trim();
                Some(content.to_string())
            }
            _ => None,
        }
    }
}

/// Collects trivia information from CST nodes
pub struct TriviaCollector {
    /// Whether to collect internal trivia (within nodes)
    collect_internal: bool,
}

impl TriviaCollector {
    /// Create a new trivia collector
    pub fn new() -> Self {
        Self {
            collect_internal: true,
        }
    }

    /// Create a trivia collector that only collects leading/trailing trivia
    pub fn external_only() -> Self {
        Self {
            collect_internal: false,
        }
    }

    /// Collect trivia for a specific node
    pub fn collect_node_trivia(&self, node: &FshSyntaxNode) -> TriviaInfo {
        let mut info = TriviaInfo::empty();

        // Collect leading trivia (previous siblings that are trivia)
        let mut current = node.prev_sibling();
        let mut leading_trivia = Vec::new();
        
        while let Some(sibling) = current {
            if self.is_trivia_node(&sibling) {
                // Collect all trivia tokens from this sibling
                for element in sibling.children_with_tokens() {
                    if let Some(token) = element.as_token() {
                        if token.kind().is_trivia() {
                            leading_trivia.push(TriviaToken::from_token(token));
                        }
                    }
                }
                current = sibling.prev_sibling();
            } else {
                break;
            }
        }
        
        // Reverse to get correct order
        leading_trivia.reverse();
        info.leading = leading_trivia;

        // Collect trailing trivia (next siblings that are trivia, until newline)
        current = node.next_sibling();
        while let Some(sibling) = current {
            if self.is_trivia_node(&sibling) {
                for element in sibling.children_with_tokens() {
                    if let Some(token) = element.as_token() {
                        if token.kind().is_trivia() {
                            info.trailing.push(TriviaToken::from_token(token));
                            // Stop at newline for trailing trivia
                            if token.kind() == FshSyntaxKind::Newline {
                                return info;
                            }
                        }
                    }
                }
                current = sibling.next_sibling();
            } else {
                break;
            }
        }

        // Collect internal trivia if enabled
        if self.collect_internal {
            info.internal = self.collect_internal_trivia(node);
        }

        info
    }

    /// Collect trivia for all nodes in a CST
    pub fn collect_trivia(&self, root: &FshSyntaxNode) -> HashMap<rowan::TextRange, TriviaInfo> {
        let mut trivia_map = HashMap::new();

        for node in root.descendants() {
            let trivia = self.collect_node_trivia(&node);
            if !trivia.is_empty() {
                trivia_map.insert(node.text_range(), trivia);
            }
        }

        trivia_map
    }

    /// Check if a node is purely trivia
    fn is_trivia_node(&self, node: &FshSyntaxNode) -> bool {
        // A node is trivia if all its tokens are trivia
        node.children_with_tokens()
            .filter_map(|e| e.into_token())
            .all(|t| t.kind().is_trivia())
    }

    /// Collect trivia tokens within a node
    fn collect_internal_trivia(&self, node: &FshSyntaxNode) -> Vec<TriviaToken> {
        let mut internal_trivia = Vec::new();

        for element in node.children_with_tokens() {
            if let Some(token) = element.as_token() {
                if token.kind().is_trivia() {
                    internal_trivia.push(TriviaToken::from_token(token));
                }
            }
        }

        internal_trivia
    }
}

impl Default for TriviaCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Preserves trivia during CST transformations
pub struct TriviaPreserver {
    /// Original trivia information
    original_trivia: HashMap<rowan::TextRange, TriviaInfo>,
}

impl TriviaPreserver {
    /// Create a new trivia preserver with collected trivia
    pub fn new(trivia: HashMap<rowan::TextRange, TriviaInfo>) -> Self {
        Self {
            original_trivia: trivia,
        }
    }

    /// Create from a CST node
    pub fn from_cst(root: &FshSyntaxNode) -> Self {
        let collector = TriviaCollector::new();
        let trivia = collector.collect_trivia(root);
        Self::new(trivia)
    }

    /// Get trivia for a specific range
    pub fn get_trivia(&self, range: &rowan::TextRange) -> Option<&TriviaInfo> {
        self.original_trivia.get(range)
    }

    /// Apply preserved trivia to a new CST
    pub fn apply_trivia(&self, new_root: &FshSyntaxNode) -> String {
        let mut result = String::new();
        self.apply_trivia_recursive(new_root, &mut result);
        result
    }

    /// Recursively apply trivia to nodes
    fn apply_trivia_recursive(&self, node: &FshSyntaxNode, result: &mut String) {
        // Look for preserved trivia for this node
        if let Some(trivia) = self.get_trivia(&node.text_range()) {
            // Add leading trivia
            for token in &trivia.leading {
                result.push_str(&token.text);
            }
        }

        // Process the node content
        for element in node.children_with_tokens() {
            match element {
                NodeOrToken::Node(child_node) => {
                    self.apply_trivia_recursive(&child_node.into(), result);
                }
                NodeOrToken::Token(token) => {
                    if !token.kind().is_trivia() {
                        result.push_str(token.text());
                    }
                }
            }
        }

        // Add trailing trivia
        if let Some(trivia) = self.get_trivia(&node.text_range()) {
            for token in &trivia.trailing {
                result.push_str(&token.text);
            }
        }
    }

    /// Check if trivia is preserved for a range
    pub fn has_trivia(&self, range: &rowan::TextRange) -> bool {
        self.original_trivia.contains_key(range)
    }

    /// Get all preserved ranges
    pub fn preserved_ranges(&self) -> impl Iterator<Item = &rowan::TextRange> {
        self.original_trivia.keys()
    }
}

/// Utilities for working with trivia in formatting
pub struct TriviaFormatter {
    /// Whether to preserve comments
    preserve_comments: bool,
    /// Whether to normalize whitespace
    normalize_whitespace: bool,
    /// Indentation string
    indent: String,
}

impl TriviaFormatter {
    /// Create a new trivia formatter
    pub fn new() -> Self {
        Self {
            preserve_comments: true,
            normalize_whitespace: true,
            indent: "  ".to_string(),
        }
    }

    /// Set comment preservation
    pub fn preserve_comments(mut self, preserve: bool) -> Self {
        self.preserve_comments = preserve;
        self
    }

    /// Set whitespace normalization
    pub fn normalize_whitespace(mut self, normalize: bool) -> Self {
        self.normalize_whitespace = normalize;
        self
    }

    /// Set indentation string
    pub fn with_indent(mut self, indent: String) -> Self {
        self.indent = indent;
        self
    }

    /// Format trivia for output
    pub fn format_trivia(&self, trivia: &TriviaInfo, indent_level: usize) -> String {
        let mut result = String::new();

        // Format leading trivia
        for token in &trivia.leading {
            result.push_str(&self.format_trivia_token(token, indent_level));
        }

        // Format trailing trivia
        for token in &trivia.trailing {
            result.push_str(&self.format_trivia_token(token, indent_level));
        }

        result
    }

    /// Format a single trivia token
    fn format_trivia_token(&self, token: &TriviaToken, indent_level: usize) -> String {
        match token.kind {
            FshSyntaxKind::CommentLine if self.preserve_comments => {
                format!("{}{}\n", self.indent.repeat(indent_level), token.text)
            }
            FshSyntaxKind::CommentBlock if self.preserve_comments => {
                format!("{}{}", self.indent.repeat(indent_level), token.text)
            }
            FshSyntaxKind::Whitespace if self.normalize_whitespace => {
                // Normalize to single space or indentation
                if token.text.contains('\n') {
                    format!("\n{}", self.indent.repeat(indent_level))
                } else {
                    " ".to_string()
                }
            }
            FshSyntaxKind::Newline => "\n".to_string(),
            _ => {
                if self.preserve_comments || !token.is_comment() {
                    token.text.clone()
                } else {
                    String::new()
                }
            }
        }
    }
}

impl Default for TriviaFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parse_fsh;

    #[test]
    fn test_trivia_token_creation() {
        let token = TriviaToken::new(
            FshSyntaxKind::CommentLine,
            "// This is a comment".to_string(),
            rowan::TextRange::new(0.into(), 20.into()),
        );

        assert!(token.is_comment());
        assert!(!token.is_whitespace());
        assert_eq!(token.comment_content(), Some("This is a comment".to_string()));
    }

    #[test]
    fn test_comment_content_extraction() {
        let line_comment = TriviaToken::new(
            FshSyntaxKind::CommentLine,
            "// Test comment".to_string(),
            rowan::TextRange::new(0.into(), 15.into()),
        );
        assert_eq!(line_comment.comment_content(), Some("Test comment".to_string()));

        let block_comment = TriviaToken::new(
            FshSyntaxKind::CommentBlock,
            "/* Block comment */".to_string(),
            rowan::TextRange::new(0.into(), 19.into()),
        );
        assert_eq!(block_comment.comment_content(), Some("Block comment".to_string()));

        let whitespace = TriviaToken::new(
            FshSyntaxKind::Whitespace,
            "   ".to_string(),
            rowan::TextRange::new(0.into(), 3.into()),
        );
        assert_eq!(whitespace.comment_content(), None);
    }

    #[test]
    fn test_trivia_info_methods() {
        let mut info = TriviaInfo::empty();
        assert!(info.is_empty());
        assert!(!info.has_comments());
        assert!(!info.has_whitespace());

        info.leading.push(TriviaToken::new(
            FshSyntaxKind::CommentLine,
            "// Comment".to_string(),
            rowan::TextRange::new(0.into(), 10.into()),
        ));

        info.trailing.push(TriviaToken::new(
            FshSyntaxKind::Whitespace,
            "  ".to_string(),
            rowan::TextRange::new(10.into(), 12.into()),
        ));

        assert!(!info.is_empty());
        assert!(info.has_comments());
        assert!(info.has_whitespace());
        assert_eq!(info.comments().len(), 1);
        assert_eq!(info.whitespace().len(), 1);
    }

    #[test]
    fn test_trivia_collector() {
        let source = r#"// Header comment
Profile: MyPatient // Inline comment
Parent: Patient"#;

        let (cst, _, _) = parse_fsh(source);
        let collector = TriviaCollector::new();
        let trivia_map = collector.collect_trivia(&cst);

        // Should collect trivia for various nodes
        assert!(!trivia_map.is_empty());
    }

    #[test]
    fn test_trivia_preserver() {
        let source = r#"Profile: MyPatient // Comment
Parent: Patient"#;

        let (cst, _, _) = parse_fsh(source);
        let preserver = TriviaPreserver::from_cst(&cst);

        // Should have preserved some trivia
        assert!(!preserver.preserved_ranges().collect::<Vec<_>>().is_empty());
    }

    #[test]
    fn test_trivia_formatter() {
        let formatter = TriviaFormatter::new()
            .preserve_comments(true)
            .normalize_whitespace(true)
            .with_indent("    ".to_string());

        let mut info = TriviaInfo::empty();
        info.leading.push(TriviaToken::new(
            FshSyntaxKind::CommentLine,
            "// Test comment".to_string(),
            rowan::TextRange::new(0.into(), 15.into()),
        ));

        let formatted = formatter.format_trivia(&info, 1);
        assert!(formatted.contains("// Test comment"));
        assert!(formatted.contains("    ")); // Indentation
    }

    #[test]
    fn test_trivia_formatter_options() {
        let formatter_no_comments = TriviaFormatter::new().preserve_comments(false);
        let formatter_no_normalize = TriviaFormatter::new().normalize_whitespace(false);

        let mut info = TriviaInfo::empty();
        info.leading.push(TriviaToken::new(
            FshSyntaxKind::CommentLine,
            "// Test comment".to_string(),
            rowan::TextRange::new(0.into(), 15.into()),
        ));

        let formatted_no_comments = formatter_no_comments.format_trivia(&info, 0);
        let formatted_no_normalize = formatter_no_normalize.format_trivia(&info, 0);

        // No comments formatter should not include comments
        assert!(!formatted_no_comments.contains("// Test comment"));
        
        // Both should handle the trivia differently
        assert_ne!(formatted_no_comments, formatted_no_normalize);
    }
}