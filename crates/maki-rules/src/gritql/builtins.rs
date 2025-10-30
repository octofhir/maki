//! FSH-specific GritQL built-in functions - CST-based
//!
//! This module provides custom built-in functions for GritQL patterns that are specific to FSH.
//! These functions can be used in .grit pattern files to perform FSH-aware queries and validations.

use super::cst_adapter::FshGritNode;
use maki_core::cst::FshSyntaxKind;

/// Register all FSH-specific built-in functions
///
/// These functions extend GritQL with FSH domain knowledge:
/// - `is_profile(node)` - Check if a node is a Profile definition
/// - `is_extension(node)` - Check if a node is an Extension definition
/// - `is_value_set(node)` - Check if a node is a ValueSet definition
/// - `is_code_system(node)` - Check if a node is a CodeSystem definition
/// - `has_comment(node)` - Check if a node has a comment
pub fn register_fsh_builtins() -> Vec<&'static str> {
    vec![
        "is_profile",
        "is_extension",
        "is_value_set",
        "is_code_system",
        "has_comment",
    ]
}

/// Check if a node is a Profile definition
pub fn is_profile(node: &FshGritNode) -> bool {
    node.kind() == FshSyntaxKind::Profile
}

/// Check if a node is an Extension definition
pub fn is_extension(node: &FshGritNode) -> bool {
    node.kind() == FshSyntaxKind::Extension
}

/// Check if a node is a ValueSet definition
pub fn is_value_set(node: &FshGritNode) -> bool {
    node.kind() == FshSyntaxKind::ValueSet
}

/// Check if a node is a CodeSystem definition
pub fn is_code_system(node: &FshGritNode) -> bool {
    node.kind() == FshSyntaxKind::CodeSystem
}

/// Check if a node or its children contain comments
pub fn has_comment(node: &FshGritNode) -> bool {
    use grit_util::AstNode;

    // Check if current node is a comment
    if matches!(
        node.kind(),
        FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock
    ) {
        return true;
    }

    // Check children for comments
    node.children().any(|child| has_comment(&child))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gritql::FshGritTree;
    use grit_util::{Ast, AstNode};

    #[test]
    fn test_register_builtins() {
        let builtins = register_fsh_builtins();
        assert!(builtins.contains(&"is_profile"));
        assert!(builtins.contains(&"is_extension"));
        assert!(builtins.contains(&"is_value_set"));
        assert!(builtins.contains(&"has_comment"));
    }

    #[test]
    fn test_is_profile() {
        let source = "Profile: MyPatient\nParent: Patient";
        let tree = FshGritTree::parse(source);
        let root = tree.root_node();

        // Find profile node in children
        let profile = root.children().find(|n| n.kind() == FshSyntaxKind::Profile);

        if let Some(profile_node) = profile {
            assert!(is_profile(&profile_node));
        }
    }

    #[test]
    fn test_is_value_set() {
        let source = "ValueSet: MyVS";
        let tree = FshGritTree::parse(source);
        let root = tree.root_node();

        let value_set = root
            .children()
            .find(|n| n.kind() == FshSyntaxKind::ValueSet);

        if let Some(vs_node) = value_set {
            assert!(is_value_set(&vs_node));
            assert!(!is_profile(&vs_node));
        }
    }
}
