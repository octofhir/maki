//! Unit tests for extension parsing without canonical manager
//!
//! These tests focus on the parsing logic without requiring database connections.

use maki_core::cst::{ast::*, parse_fsh};

#[test]
fn test_simple_extension_parsing() {
    let fsh = r#"
Extension: SimpleExtension
Id: simple-ext
Title: "Simple Extension"
Description: "A simple extension"
* value[x] only string
"#;

    let (root, _lexer_errors, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    // Check basic properties
    assert_eq!(extension.name(), Some("SimpleExtension".to_string()));

    if let Some(id_clause) = extension.id() {
        assert_eq!(id_clause.value(), Some("simple-ext".to_string()));
    }

    if let Some(title_clause) = extension.title() {
        assert_eq!(title_clause.value(), Some("Simple Extension".to_string()));
    }

    if let Some(desc_clause) = extension.description() {
        assert_eq!(desc_clause.value(), Some("A simple extension".to_string()));
    }

    // Check rules
    let rules: Vec<_> = extension.rules().collect();
    assert!(!rules.is_empty(), "Extension should have rules");

    // Look for only rule
    let only_rules: Vec<_> = rules
        .iter()
        .filter_map(|r| if let Rule::Only(o) = r { Some(o) } else { None })
        .collect();

    assert!(!only_rules.is_empty(), "Should have only rules");

    // Check the only rule targets value[x]
    for only_rule in &only_rules {
        if let Some(path) = only_rule.path() {
            let path_str = path.as_string();
            if path_str.contains("value") {
                let types = only_rule.types();
                assert!(!types.is_empty(), "Only rule should have types");
                assert!(
                    types.contains(&"string".to_string()),
                    "Should constrain to string type"
                );
            }
        }
    }
}
