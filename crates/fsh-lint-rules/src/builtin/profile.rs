//! Profile-specific validation rules
//!
//! Validates that profiles and extensions have proper assignments and context.

use fsh_lint_core::{Diagnostic, SemanticModel, Severity};
// TODO: Migrate to Chumsky AST
// use tree_sitter::Node;

/// Rule ID for profile assignment validation
pub const PROFILE_ASSIGNMENT_PRESENT: &str = "correctness/profile-assignment-present";

/// Rule ID for extension context validation
pub const EXTENSION_CONTEXT_MISSING: &str = "correctness/extension-context-missing";

/// Check for missing profile assignments (status, abstract)
pub fn check_profile_assignments(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for profile in &model.document.profiles {
        // Check if Parent is specified (required per spec)
        if profile.parent.is_none() {
            let location = model.source_map.span_to_diagnostic_location(
                &profile.span,
                &model.source,
                &model.source_file,
            );

            diagnostics.push(
                Diagnostic::new(
                    PROFILE_ASSIGNMENT_PRESENT,
                    Severity::Warning,
                    &format!(
                        "Profile '{}' does not specify a Parent. Profiles should declare a parent resource type.",
                        profile.name.value
                    ),
                    location.clone(),
                )
                .with_suggestion(fsh_lint_core::Suggestion {
                    message: "Add Parent declaration".to_string(),
                    replacement: "Parent: <resource-type>".to_string(),
                    location,
                    is_safe: false,
                }),
            );
        }

        // Check for recommended caret assignments (^status, ^experimental)
        let has_status = profile.rules.iter().any(|rule| {
            if let fsh_lint_core::ast::SDRule::CaretValue(caret) = rule {
                caret.caret_path.value == "status"
            } else {
                false
            }
        });

        if !has_status {
            let location = model.source_map.span_to_diagnostic_location(
                &profile.span,
                &model.source,
                &model.source_file,
            );

            diagnostics.push(
                Diagnostic::new(
                    PROFILE_ASSIGNMENT_PRESENT,
                    Severity::Info,
                    &format!(
                        "Profile '{}' does not specify ^status. Consider adding ^status = #draft or #active",
                        profile.name.value
                    ),
                    location.clone(),
                )
                .with_suggestion(fsh_lint_core::Suggestion {
                    message: "Add status assignment".to_string(),
                    replacement: "* ^status = #draft".to_string(),
                    location,
                    is_safe: false,
                }),
            );
        }
    }

    diagnostics
}

/// Check for missing extension context
pub fn check_extension_context(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for extension in &model.document.extensions {
        // Check if contexts field is empty
        if extension.contexts.is_empty() {
            let location = model.source_map.span_to_diagnostic_location(
                &extension.span,
                &model.source,
                &model.source_file,
            );

            diagnostics.push(
                Diagnostic::new(
                    EXTENSION_CONTEXT_MISSING,
                    Severity::Error,
                    &format!(
                        "Extension '{}' is missing context specification. Extensions must define where they can be used.",
                        extension.name.value
                    ),
                    location.clone(),
                )
                .with_suggestion(fsh_lint_core::Suggestion {
                    message: "Add context specification".to_string(),
                    replacement: "Context: <element-path>".to_string(),
                    location,
                    is_safe: false,
                }),
            );
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsh_lint_core::ast::{Extension, Profile, SDRule, CaretValueRule, Spanned, FSHDocument, Value, Code};
    use fsh_lint_core::SemanticModel;
    use std::path::PathBuf;

    #[test]
    fn test_extension_with_context() {
        let source = "Extension: MyExtension\nContext: Patient\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let mut doc = FSHDocument::new(0..source.len());
        doc.extensions.push(Extension {
            name: Spanned::new("MyExtension".to_string(), 11..22),
            parent: None,
            id: None,
            title: None,
            description: None,
            contexts: vec![Spanned::new("Patient".to_string(), 23..30)], // Has context
            rules: Vec::new(),
            span: 0..source.len(),
        });

        let model = SemanticModel {
            document: doc,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let diagnostics = check_extension_context(&model);
        assert_eq!(diagnostics.len(), 0, "Extension with context should not produce diagnostic");
    }

    #[test]
    fn test_extension_missing_context() {
        let source = "Extension: NoContextExtension\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let mut doc = FSHDocument::new(0..source.len());
        doc.extensions.push(Extension {
            name: Spanned::new("NoContextExtension".to_string(), 11..29),
            parent: None,
            id: None,
            title: None,
            description: None,
            contexts: Vec::new(), // Missing context
            rules: Vec::new(),
            span: 0..source.len(),
        });

        let model = SemanticModel {
            document: doc,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let diagnostics = check_extension_context(&model);
        assert_eq!(diagnostics.len(), 1, "Extension without context should produce diagnostic");
        assert_eq!(diagnostics[0].rule_id, EXTENSION_CONTEXT_MISSING);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains("NoContextExtension"));
        assert!(diagnostics[0].message.contains("missing context specification"));
    }

    #[test]
    fn test_profile_with_parent_and_status() {
        let source = "Profile: GoodProfile\nParent: Patient\n* ^status = #draft\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let mut doc = FSHDocument::new(0..source.len());
        doc.profiles.push(Profile {
            name: Spanned::new("GoodProfile".to_string(), 9..20),
            parent: Some(Spanned::new("Patient".to_string(), 29..36)),
            id: None,
            title: None,
            description: None,
            rules: vec![SDRule::CaretValue(CaretValueRule {
                path: None,
                caret_path: Spanned::new("status".to_string(), 40..46),
                value: Spanned::new(Value::Code(Code {
                    system: None,
                    code: "draft".to_string(),
                    display: None,
                }),49..55),
                span: 38..55,
            })],
            span: 0..source.len(),
        });

        let model = SemanticModel {
            document: doc,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let diagnostics = check_profile_assignments(&model);
        assert_eq!(diagnostics.len(), 0, "Profile with parent and status should not produce diagnostics");
    }

    #[test]
    fn test_profile_missing_parent() {
        let source = "Profile: NoParentProfile\n* ^status = #draft\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let mut doc = FSHDocument::new(0..source.len());
        doc.profiles.push(Profile {
            name: Spanned::new("NoParentProfile".to_string(), 9..24),
            parent: None, // Missing parent
            id: None,
            title: None,
            description: None,
            rules: vec![SDRule::CaretValue(CaretValueRule {
                path: None,
                caret_path: Spanned::new("status".to_string(), 28..34),
                value: Spanned::new(Value::Code(Code {
                    system: None,
                    code: "draft".to_string(),
                    display: None,
                }),37..43),
                span: 26..43,
            })],
            span: 0..source.len(),
        });

        let model = SemanticModel {
            document: doc,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let diagnostics = check_profile_assignments(&model);
        assert_eq!(diagnostics.len(), 1, "Profile without parent should produce warning");
        assert_eq!(diagnostics[0].rule_id, PROFILE_ASSIGNMENT_PRESENT);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("NoParentProfile"));
        assert!(diagnostics[0].message.contains("does not specify a Parent"));
    }

    #[test]
    fn test_profile_missing_status() {
        let source = "Profile: NoStatusProfile\nParent: Patient\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let mut doc = FSHDocument::new(0..source.len());
        doc.profiles.push(Profile {
            name: Spanned::new("NoStatusProfile".to_string(), 9..24),
            parent: Some(Spanned::new("Patient".to_string(), 33..40)),
            id: None,
            title: None,
            description: None,
            rules: Vec::new(), // Missing status
            span: 0..source.len(),
        });

        let model = SemanticModel {
            document: doc,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let diagnostics = check_profile_assignments(&model);
        assert_eq!(diagnostics.len(), 1, "Profile without status should produce info diagnostic");
        assert_eq!(diagnostics[0].rule_id, PROFILE_ASSIGNMENT_PRESENT);
        assert_eq!(diagnostics[0].severity, Severity::Info);
        assert!(diagnostics[0].message.contains("NoStatusProfile"));
        assert!(diagnostics[0].message.contains("does not specify ^status"));
    }

    #[test]
    fn test_profile_missing_both_parent_and_status() {
        let source = "Profile: IncompleteProfile\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let mut doc = FSHDocument::new(0..source.len());
        doc.profiles.push(Profile {
            name: Spanned::new("IncompleteProfile".to_string(), 9..26),
            parent: None, // Missing parent
            id: None,
            title: None,
            description: None,
            rules: Vec::new(), // Missing status
            span: 0..source.len(),
        });

        let model = SemanticModel {
            document: doc,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let diagnostics = check_profile_assignments(&model);
        assert_eq!(diagnostics.len(), 2, "Profile missing both parent and status should produce 2 diagnostics");

        // Should have one warning (parent) and one info (status)
        let severities: Vec<_> = diagnostics.iter().map(|d| d.severity).collect();
        assert!(severities.contains(&Severity::Warning), "Should have warning for missing parent");
        assert!(severities.contains(&Severity::Info), "Should have info for missing status");
    }

    #[test]
    fn test_multiple_extensions_mixed_contexts() {
        let source = "Extension: Ext1\nContext: Patient\n\nExtension: Ext2\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let mut doc = FSHDocument::new(0..source.len());
        doc.extensions.push(Extension {
            name: Spanned::new("Ext1".to_string(), 11..15),
            parent: None,
            id: None,
            title: None,
            description: None,
            contexts: vec![Spanned::new("Patient".to_string(), 23..30)], // Has context
            rules: Vec::new(),
            span: 0..33,
        });
        doc.extensions.push(Extension {
            name: Spanned::new("Ext2".to_string(), 45..49),
            parent: None,
            id: None,
            title: None,
            description: None,
            contexts: Vec::new(), // Missing context
            rules: Vec::new(),
            span: 35..source.len(),
        });

        let model = SemanticModel {
            document: doc,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let diagnostics = check_extension_context(&model);
        assert_eq!(diagnostics.len(), 1, "Should only flag Ext2 without context");
        assert!(diagnostics[0].message.contains("Ext2"));
    }
}

/*
// TODO: Reimplement using Chumsky AST

pub fn check_profile_assignments(node: Node, source: &str, file_path: &PathBuf) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Walk through profile declarations
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "profile_declaration" {
            if let Some(profile_diags) = check_single_profile(child, source, file_path) {
                diagnostics.extend(profile_diags);
            }
        }

        // Recursively check children
        diagnostics.extend(check_profile_assignments(child, source, file_path));
    }

    diagnostics
}

/*
// TODO: Reimplement using Chumsky AST

pub fn check_extension_context(node: Node, source: &str, file_path: &PathBuf) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Walk through extension declarations
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "extension_declaration" {
            if let Some(ext_diags) = check_single_extension(child, source, file_path) {
                diagnostics.extend(ext_diags);
            }
        }

        // Recursively check children
        diagnostics.extend(check_extension_context(child, source, file_path));
    }

    diagnostics
}

/// Check a single profile for required assignments
fn check_single_profile(node: Node, source: &str, file_path: &PathBuf) -> Option<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    let profile_text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let profile_name = extract_resource_name(node, source);

    // Check for ^status assignment
    if !has_assignment(profile_text, "status") {
        diagnostics.push(create_missing_assignment_diagnostic(
            node,
            file_path,
            &profile_name,
            "status",
            "draft",
            Severity::Warning,
        ));
    }

    // Check for ^abstract assignment
    if !has_assignment(profile_text, "abstract") {
        diagnostics.push(create_missing_assignment_diagnostic(
            node,
            file_path,
            &profile_name,
            "abstract",
            "false",
            Severity::Info,
        ));
    }

    // Check for Parent declaration (important for profiles)
    if !has_parent(profile_text) {
        diagnostics.push(create_missing_parent_diagnostic(
            node,
            file_path,
            &profile_name,
        ));
    }

    if diagnostics.is_empty() {
        None
    } else {
        Some(diagnostics)
    }
}

/// Check a single extension for context
fn check_single_extension(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Option<Vec<Diagnostic>> {
    let extension_text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let extension_name = extract_resource_name(node, source);

    // Check for ^context assignment
    if !has_context(extension_text) {
        let diagnostic = create_missing_context_diagnostic(node, file_path, &extension_name);
        Some(vec![diagnostic])
    } else {
        None
    }
}

/// Extract resource name from declaration
fn extract_resource_name(node: Node, source: &str) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");

    if let Some(colon_pos) = text.find(':') {
        let after_colon = &text[colon_pos + 1..];
        if let Some(first_line) = after_colon.lines().next() {
            return first_line.trim().to_string();
        }
    }

    "Unknown".to_string()
}

/// Check if assignment is present (^field = value)
fn has_assignment(text: &str, field: &str) -> bool {
    let pattern = format!("^{}", field);
    text.contains(&pattern)
}

/// Check if Parent declaration is present
fn has_parent(text: &str) -> bool {
    text.contains("Parent:") || text.contains("Parent ")
}

/// Check if context is present
fn has_context(text: &str) -> bool {
    text.contains("^context")
}

/// Create diagnostic for missing assignment
fn create_missing_assignment_diagnostic(
    node: Node,
    file_path: &PathBuf,
    profile_name: &str,
    field: &str,
    default_value: &str,
    severity: Severity,
) -> Diagnostic {
    let location = create_location(node, file_path);

    let message = format!(
        "Profile '{}' is missing ^{} assignment",
        profile_name, field
    );

    let mut diagnostic = Diagnostic::new(
        PROFILE_ASSIGNMENT_PRESENT,
        severity,
        &message,
        location.clone(),
    );

    let suggestion_text = format!("* ^{} = #{}", field, default_value);

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: format!("Add ^{} assignment with default value", field),
        replacement: format!("\n{}", suggestion_text),
        location: location.clone(),
        is_safe: true, // Adding metadata assignments is safe
    });

    diagnostic
}

/// Create diagnostic for missing Parent
fn create_missing_parent_diagnostic(
    node: Node,
    file_path: &PathBuf,
    profile_name: &str,
) -> Diagnostic {
    let location = create_location(node, file_path);

    let message = format!("Profile '{}' is missing Parent declaration", profile_name);

    let mut diagnostic = Diagnostic::new(
        PROFILE_ASSIGNMENT_PRESENT,
        Severity::Error,
        &message,
        location.clone(),
    );

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: "Add Parent declaration".to_string(),
        replacement: "\nParent: Resource".to_string(),
        location: location.clone(),
        is_safe: false, // Choosing parent is semantic decision
    });

    diagnostic
}

/// Create diagnostic for missing extension context
fn create_missing_context_diagnostic(
    node: Node,
    file_path: &PathBuf,
    extension_name: &str,
) -> Diagnostic {
    let location = create_location(node, file_path);

    let message = format!(
        "Extension '{}' is missing ^context specification",
        extension_name
    );

    let mut diagnostic = Diagnostic::new(
        EXTENSION_CONTEXT_MISSING,
        Severity::Error,
        &message,
        location.clone(),
    );

    let suggestion_text = r#"* ^context[+].type = #element
* ^context[=].expression = "Element""#;

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: "Add context specification".to_string(),
        replacement: format!("\n{}", suggestion_text),
        location: location.clone(),
        is_safe: false, // Context choice is semantic decision
    });

    diagnostic
}

/// Create a Location from a tree-sitter Node
fn create_location(node: Node, file_path: &PathBuf) -> Location {
    Location {
        file: file_path.clone(),
        line: node.start_position().row + 1,
        column: node.start_position().column + 1,
        end_line: Some(node.end_position().row + 1),
        end_column: Some(node.end_position().column + 1),
        offset: node.start_byte(),
        length: node.end_byte() - node.start_byte(),
        span: Some((node.start_byte(), node.end_byte())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_assignment() {
        assert!(has_assignment("^status = #draft", "status"));
        assert!(has_assignment("* ^abstract = false", "abstract"));
        assert!(!has_assignment("^status = #draft", "abstract"));
    }

    #[test]
    fn test_has_parent() {
        assert!(has_parent("Parent: Patient"));
        assert!(has_parent("Profile: MyProfile\nParent: Observation"));
        assert!(!has_parent("Profile: MyProfile\nTitle: Test"));
    }

    #[test]
    fn test_has_context() {
        assert!(has_context("^context[+].type = #element"));
        assert!(has_context("* ^context[0].expression = \"Patient\""));
        assert!(!has_context("^status = #draft"));
    }

    #[test]
    fn test_extract_resource_name() {
        // This would normally work with actual tree-sitter nodes
        // For now we just test the helper functions
        assert!(has_assignment("^status = #draft", "status"));
    }

    #[test]
    fn test_multiple_assignments() {
        let text = r#"
Profile: MyProfile
Parent: Patient
* ^status = #draft
* ^abstract = false
"#;
        assert!(has_assignment(text, "status"));
        assert!(has_assignment(text, "abstract"));
        assert!(has_parent(text));
    }

    #[test]
    fn test_missing_assignments() {
        let text = r#"
Profile: MyProfile
Parent: Patient
"#;
        assert!(!has_assignment(text, "status"));
        assert!(!has_assignment(text, "abstract"));
        assert!(has_parent(text));
    }
}

*/
*/