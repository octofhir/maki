//! FSH Target Language for GritQL - CST-based implementation
//!
//! This module implements the Language trait that tells GritQL how to work
//! with our FSH syntax tree.

use super::cst_adapter::FshGritNode;
use grit_util::{CodeRange, EffectRange, Language};
use maki_core::cst::FshSyntaxKind;
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

/// FSH Target Language implementation for GritQL
#[derive(Clone, Debug)]
pub struct FshTargetLanguage;

impl FshTargetLanguage {
    /// Convert node name string to syntax kind
    pub fn kind_by_name(name: &str) -> Option<FshSyntaxKind> {
        // Normalize to uppercase with underscores
        let normalized = name.to_uppercase().replace('-', "_");

        match normalized.as_str() {
            // Definitions
            "PROFILE" => Some(FshSyntaxKind::Profile),
            "EXTENSION" => Some(FshSyntaxKind::Extension),
            "VALUESET" | "VALUE_SET" => Some(FshSyntaxKind::ValueSet),
            "CODESYSTEM" | "CODE_SYSTEM" => Some(FshSyntaxKind::CodeSystem),
            "INSTANCE" => Some(FshSyntaxKind::Instance),
            "INVARIANT" => Some(FshSyntaxKind::Invariant),
            "MAPPING" => Some(FshSyntaxKind::Mapping),
            "LOGICAL" => Some(FshSyntaxKind::Logical),
            "RESOURCE" => Some(FshSyntaxKind::Resource),
            "ALIAS" => Some(FshSyntaxKind::Alias),
            "RULESET" | "RULE_SET" => Some(FshSyntaxKind::RuleSet),

            // Document/Root
            "DOCUMENT" => Some(FshSyntaxKind::Document),
            "ROOT" => Some(FshSyntaxKind::Root),

            // Clauses
            "PARENT_CLAUSE" => Some(FshSyntaxKind::ParentClause),
            "ID_CLAUSE" => Some(FshSyntaxKind::IdClause),
            "TITLE_CLAUSE" => Some(FshSyntaxKind::TitleClause),
            "DESCRIPTION_CLAUSE" => Some(FshSyntaxKind::DescriptionClause),
            "INSTANCEOF_CLAUSE" => Some(FshSyntaxKind::InstanceofClause),

            // Rules
            "CARD_RULE" => Some(FshSyntaxKind::CardRule),
            "FLAG_RULE" => Some(FshSyntaxKind::FlagRule),
            "VALUESET_RULE" => Some(FshSyntaxKind::ValuesetRule),
            "FIXED_VALUE_RULE" => Some(FshSyntaxKind::FixedValueRule),
            "ONLY_RULE" => Some(FshSyntaxKind::OnlyRule),
            "CONTAINS_RULE" => Some(FshSyntaxKind::ContainsRule),
            "OBEYS_RULE" => Some(FshSyntaxKind::ObeysRule),
            "CARET_VALUE_RULE" | "CARET_RULE" => Some(FshSyntaxKind::CaretValueRule),

            // Literals
            "IDENT" => Some(FshSyntaxKind::Ident),
            "STRING" => Some(FshSyntaxKind::String),
            "INTEGER" => Some(FshSyntaxKind::Integer),
            "DECIMAL" => Some(FshSyntaxKind::Decimal),

            // Comments (trivia)
            "COMMENT_LINE" => Some(FshSyntaxKind::CommentLine),
            "COMMENT_BLOCK" => Some(FshSyntaxKind::CommentBlock),

            _ => None,
        }
    }

    /// Convert syntax kind to string name
    pub fn name_for_kind(kind: FshSyntaxKind) -> &'static str {
        match kind {
            // Definitions
            FshSyntaxKind::Profile => "Profile",
            FshSyntaxKind::Extension => "Extension",
            FshSyntaxKind::ValueSet => "ValueSet",
            FshSyntaxKind::CodeSystem => "CodeSystem",
            FshSyntaxKind::Instance => "Instance",
            FshSyntaxKind::Invariant => "Invariant",
            FshSyntaxKind::Mapping => "Mapping",
            FshSyntaxKind::Logical => "Logical",
            FshSyntaxKind::Resource => "Resource",
            FshSyntaxKind::Alias => "Alias",
            FshSyntaxKind::RuleSet => "RuleSet",

            // Document
            FshSyntaxKind::Document => "Document",
            FshSyntaxKind::Root => "Root",

            // Clauses
            FshSyntaxKind::ParentClause => "ParentClause",
            FshSyntaxKind::IdClause => "IdClause",
            FshSyntaxKind::TitleClause => "TitleClause",
            FshSyntaxKind::DescriptionClause => "DescriptionClause",
            FshSyntaxKind::InstanceofClause => "InstanceOfClause",

            // Rules
            FshSyntaxKind::CardRule => "CardRule",
            FshSyntaxKind::FlagRule => "FlagRule",
            FshSyntaxKind::ValuesetRule => "ValueSetRule",
            FshSyntaxKind::FixedValueRule => "FixedValueRule",
            FshSyntaxKind::OnlyRule => "OnlyRule",
            FshSyntaxKind::ContainsRule => "ContainsRule",
            FshSyntaxKind::ObeysRule => "ObeysRule",
            FshSyntaxKind::CaretValueRule => "CaretValueRule",

            // Literals
            FshSyntaxKind::Ident => "Ident",
            FshSyntaxKind::String => "String",
            FshSyntaxKind::Integer => "Integer",
            FshSyntaxKind::Decimal => "Decimal",

            // Trivia
            FshSyntaxKind::CommentLine => "CommentLine",
            FshSyntaxKind::CommentBlock => "CommentBlock",
            FshSyntaxKind::Whitespace => "Whitespace",

            _ => "Unknown",
        }
    }

    /// Check if a syntax kind is a comment
    pub fn is_comment_kind(kind: FshSyntaxKind) -> bool {
        matches!(
            kind,
            FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock
        )
    }

    /// Check if a syntax kind is trivia (whitespace/comments)
    pub fn is_trivia_kind(kind: FshSyntaxKind) -> bool {
        kind.is_trivia()
    }
}

// Metavariable regex patterns for GritQL
// GritQL uses $variable for captures
static METAVARIABLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\$[a-zA-Z_][a-zA-Z0-9_]*$").unwrap());

static METAVARIABLE_BRACKET_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\$\{[a-zA-Z_][a-zA-Z0-9_]*\}$").unwrap());

impl Language for FshTargetLanguage {
    type Node<'a> = FshGritNode;

    fn language_name(&self) -> &'static str {
        "fsh"
    }

    fn snippet_context_strings(&self) -> &[(&'static str, &'static str)] {
        // Provide context for parsing FSH fragments in GritQL patterns
        &[
            // Empty context (for complete documents)
            ("", ""),
            // Profile context for rules
            ("Profile: __GritQLTest__\nParent: Patient\n", ""),
            // ValueSet context
            ("ValueSet: __GritQLTest__\n", ""),
            // Extension context
            ("Extension: __GritQLTest__\n", ""),
        ]
    }

    fn is_comment(&self, node: &Self::Node<'_>) -> bool {
        Self::is_comment_kind(node.kind())
    }

    fn is_metavariable(&self, node: &Self::Node<'_>) -> bool {
        use grit_util::AstNode;

        // Check if node text matches metavariable pattern
        if let Ok(text) = node.text() {
            METAVARIABLE_REGEX.is_match(text.as_ref())
                || METAVARIABLE_BRACKET_REGEX.is_match(text.as_ref())
        } else {
            false
        }
    }

    fn align_padding<'a>(
        &self,
        _node: &Self::Node<'a>,
        _range: &CodeRange,
        _skip_ranges: &[CodeRange],
        _new_padding: Option<usize>,
        _offset: usize,
        _substitutions: &mut [(EffectRange, String)],
    ) -> Cow<'a, str> {
        // Padding alignment not yet implemented
        // This is used for maintaining indentation in code rewrites
        Cow::Borrowed("")
    }

    fn pad_snippet<'a>(&self, snippet: &'a str, _padding: &str) -> Cow<'a, str> {
        // For now, just return snippet as-is
        // In the future, this could add proper indentation
        Cow::Borrowed(snippet)
    }

    fn get_skip_padding_ranges(&self, _node: &Self::Node<'_>) -> Vec<CodeRange> {
        // No skip padding ranges for now
        // This would return ranges that shouldn't be affected by padding changes
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_by_name() {
        assert_eq!(
            FshTargetLanguage::kind_by_name("profile"),
            Some(FshSyntaxKind::Profile)
        );
        assert_eq!(
            FshTargetLanguage::kind_by_name("Profile"),
            Some(FshSyntaxKind::Profile)
        );
        assert_eq!(
            FshTargetLanguage::kind_by_name("ValueSet"),
            Some(FshSyntaxKind::ValueSet)
        );
        assert_eq!(
            FshTargetLanguage::kind_by_name("value_set"),
            Some(FshSyntaxKind::ValueSet)
        );
        assert_eq!(FshTargetLanguage::kind_by_name("unknown"), None);
    }

    #[test]
    fn test_name_for_kind() {
        assert_eq!(
            FshTargetLanguage::name_for_kind(FshSyntaxKind::Profile),
            "Profile"
        );
        assert_eq!(
            FshTargetLanguage::name_for_kind(FshSyntaxKind::ValueSet),
            "ValueSet"
        );
        assert_eq!(
            FshTargetLanguage::name_for_kind(FshSyntaxKind::CodeSystem),
            "CodeSystem"
        );
    }

    #[test]
    fn test_is_comment_kind() {
        assert!(FshTargetLanguage::is_comment_kind(
            FshSyntaxKind::CommentLine
        ));
        assert!(FshTargetLanguage::is_comment_kind(
            FshSyntaxKind::CommentBlock
        ));
        assert!(!FshTargetLanguage::is_comment_kind(FshSyntaxKind::Profile));
    }

    #[test]
    fn test_is_trivia_kind() {
        assert!(FshTargetLanguage::is_trivia_kind(FshSyntaxKind::Whitespace));
        assert!(FshTargetLanguage::is_trivia_kind(
            FshSyntaxKind::CommentLine
        ));
        assert!(FshTargetLanguage::is_trivia_kind(FshSyntaxKind::Newline));
        assert!(!FshTargetLanguage::is_trivia_kind(FshSyntaxKind::Profile));
    }

    #[test]
    fn test_metavariable_pattern() {
        assert!(METAVARIABLE_REGEX.is_match("$name"));
        assert!(METAVARIABLE_REGEX.is_match("$my_var"));
        assert!(METAVARIABLE_REGEX.is_match("$myVar123"));
        assert!(!METAVARIABLE_REGEX.is_match("name"));
        assert!(!METAVARIABLE_REGEX.is_match("$123"));
    }

    #[test]
    fn test_bracket_metavariable_pattern() {
        assert!(METAVARIABLE_BRACKET_REGEX.is_match("${name}"));
        assert!(METAVARIABLE_BRACKET_REGEX.is_match("${my_var}"));
        assert!(!METAVARIABLE_BRACKET_REGEX.is_match("$name"));
        assert!(!METAVARIABLE_BRACKET_REGEX.is_match("{name}"));
    }

    #[test]
    fn test_language_name() {
        let lang = FshTargetLanguage;
        assert_eq!(lang.language_name(), "fsh");
    }

    #[test]
    fn test_snippet_contexts() {
        let lang = FshTargetLanguage;
        let contexts = lang.snippet_context_strings();

        assert!(!contexts.is_empty());
        assert_eq!(contexts[0], ("", ""));
    }
}
