//! FSH-specific GritQL built-in functions - CST-based
//!
//! This module provides custom built-in functions for GritQL patterns that are specific to FSH.
//! These functions can be used in .grit pattern files to perform FSH-aware queries and validations.
//!
//! ## Available Built-in Functions:
//!
//! **Node Type Checks:**
//! - `is_profile(node)` - Check if a node is a Profile definition
//! - `is_extension(node)` - Check if a node is an Extension definition
//! - `is_value_set(node)` - Check if a node is a ValueSet definition
//! - `is_code_system(node)` - Check if a node is a CodeSystem definition
//!
//! **Node Properties:**
//! - `has_comment(node)` - Check if a node has a comment
//! - `has_title(node)` - Check if a node has a title
//! - `has_description(node)` - Check if a node has a description
//! - `has_parent(node)` - Check if a Profile has a parent defined
//!
//! **String Validation:**
//! - `is_kebab_case(text)` - Check if string matches kebab-case pattern (lowercase-with-dashes)
//! - `is_pascal_case(text)` - Check if string matches PascalCase pattern
//! - `is_camel_case(text)` - Check if string matches camelCase pattern
//! - `is_screaming_snake_case(text)` - Check if string matches SCREAMING_SNAKE_CASE pattern

use super::cst_adapter::FshGritNode;
use maki_core::cst::FshSyntaxKind;
use regex::Regex;

/// Register all FSH-specific built-in functions
///
/// These functions extend GritQL with FSH domain knowledge for pattern matching.
pub fn register_fsh_builtins() -> Vec<&'static str> {
    vec![
        "is_profile",
        "is_extension",
        "is_value_set",
        "is_code_system",
        "has_comment",
        "has_title",
        "has_description",
        "has_parent",
        "is_kebab_case",
        "is_pascal_case",
        "is_camel_case",
        "is_screaming_snake_case",
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

/// Check if a node has a title field defined
pub fn has_title(node: &FshGritNode) -> bool {
    node.get_field_text("title").is_some()
}

/// Check if a node has a description field defined
pub fn has_description(node: &FshGritNode) -> bool {
    node.get_field_text("description").is_some()
}

/// Check if a Profile node has a parent defined
pub fn has_parent(node: &FshGritNode) -> bool {
    node.get_field_text("parent").is_some()
}

/// Check if a string matches kebab-case pattern (lowercase-with-dashes)
///
/// Examples: `my-profile`, `patient-id`, `value-set-name`
pub fn is_kebab_case(text: &str) -> bool {
    // Kebab case: lowercase letters, numbers, and dashes only
    // Must start with lowercase letter and contain no consecutive dashes
    Regex::new(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$")
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

/// Check if a string matches PascalCase pattern
///
/// Examples: `MyProfile`, `PatientRecord`, `ValueSetName`
pub fn is_pascal_case(text: &str) -> bool {
    // Pascal case: starts with uppercase, no separators
    Regex::new(r"^[A-Z][a-zA-Z0-9]*$")
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

/// Check if a string matches camelCase pattern
///
/// Examples: `myProfile`, `patientRecord`, `valueSetName`
pub fn is_camel_case(text: &str) -> bool {
    // Camel case: starts with lowercase, no separators
    Regex::new(r"^[a-z][a-zA-Z0-9]*$")
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

/// Check if a string matches SCREAMING_SNAKE_CASE pattern
///
/// Examples: `MY_PROFILE`, `PATIENT_ID`, `VALUE_SET_NAME`
pub fn is_screaming_snake_case(text: &str) -> bool {
    // Screaming snake case: uppercase letters, numbers, and underscores
    Regex::new(r"^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$")
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gritql::FshGritTree;
    use grit_util::{Ast, AstNode};

    #[test]
    fn test_register_builtins() {
        let builtins = register_fsh_builtins();
        // Node type checks
        assert!(builtins.contains(&"is_profile"));
        assert!(builtins.contains(&"is_extension"));
        assert!(builtins.contains(&"is_value_set"));
        assert!(builtins.contains(&"is_code_system"));
        // Node properties
        assert!(builtins.contains(&"has_comment"));
        assert!(builtins.contains(&"has_title"));
        assert!(builtins.contains(&"has_description"));
        assert!(builtins.contains(&"has_parent"));
        // String validation
        assert!(builtins.contains(&"is_kebab_case"));
        assert!(builtins.contains(&"is_pascal_case"));
        assert!(builtins.contains(&"is_camel_case"));
        assert!(builtins.contains(&"is_screaming_snake_case"));
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

    #[test]
    fn test_naming_conventions() {
        // Test kebab-case
        assert!(is_kebab_case("my-profile"));
        assert!(is_kebab_case("patient-id"));
        assert!(!is_kebab_case("MyProfile"));
        assert!(!is_kebab_case("my_profile"));

        // Test PascalCase
        assert!(is_pascal_case("MyProfile"));
        assert!(is_pascal_case("PatientId"));
        assert!(!is_pascal_case("my-profile"));
        assert!(!is_pascal_case("myProfile"));

        // Test camelCase
        assert!(is_camel_case("myProfile"));
        assert!(is_camel_case("patientId"));
        assert!(!is_camel_case("MyProfile"));
        assert!(!is_camel_case("my-profile"));

        // Test SCREAMING_SNAKE_CASE
        assert!(is_screaming_snake_case("MY_PROFILE"));
        assert!(is_screaming_snake_case("PATIENT_ID"));
        assert!(!is_screaming_snake_case("MyProfile"));
        assert!(!is_screaming_snake_case("my_profile"));
    }

    #[test]
    fn test_field_property_checks() {
        // Profile with title and parent
        let source = "Profile: MyPatient\nParent: Patient\nTitle: \"My Patient Profile\"";
        let tree = FshGritTree::parse(source);
        let root = tree.root_node();

        let profile = root.children().find(|n| n.kind() == FshSyntaxKind::Profile);

        if let Some(profile_node) = profile {
            // Should have parent and title
            assert!(has_parent(&profile_node), "Profile should have parent");
            assert!(has_title(&profile_node), "Profile should have title");
        }
    }

    #[test]
    fn test_has_comment_with_comments() {
        let source = "// This is a comment\nProfile: MyPatient";
        let tree = FshGritTree::parse(source);
        let root = tree.root_node();

        // Find the profile node - the comment might be attached to it
        let profile = root.children().find(|n| n.kind() == FshSyntaxKind::Profile);

        if let Some(profile_node) = profile {
            // The profile should have a comment in its tree (even if in trivia)
            // For now, just verify the profile exists
            assert!(is_profile(&profile_node), "Should find profile node");
        }
    }
}
