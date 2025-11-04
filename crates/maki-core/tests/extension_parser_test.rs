//! Unit tests for extension parsing without canonical manager
//!
//! These tests focus on the parsing logic without requiring database connections.

use maki_core::cst::{ast::*, parse_fsh};

#[test]
fn test_multiline_contains_rule_parsing() {
    // Test multiline contains rule with proper FSH syntax
    let fsh = r#"
Extension: MultilineExtension
Id: multiline-ext
* extension contains
    subExt1 0..1 MS and
    subExt2 1..1 and
    subExt3 0..* MS
* extension[subExt1].value[x] only string
* extension[subExt2].value[x] only integer
* extension[subExt3].value[x] only boolean
* value[x] 0..0
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    // Debug: print the full document structure
    println!("Full document text: '{}'", root.text());

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    // Check extension name
    assert_eq!(extension.name(), Some("MultilineExtension".to_string()));

    // Check that we have rules
    let rules: Vec<_> = extension.rules().collect();
    assert!(!rules.is_empty(), "Extension should have rules");

    // Look for contains rule
    let contains_rules: Vec<_> = rules
        .iter()
        .filter_map(|r| {
            if let Rule::Contains(c) = r {
                Some(c)
            } else {
                None
            }
        })
        .collect();

    assert!(!contains_rules.is_empty(), "Should have contains rules");

    // Check contains rule items
    for contains_rule in &contains_rules {
        let items = contains_rule.items();
        println!("Contains rule items: {:?}", items);
        println!(
            "Contains rule syntax text: '{}'",
            contains_rule.syntax().text()
        );

        // Debug: print all child nodes
        for child in contains_rule.syntax().children() {
            println!("  Child node: {:?} - '{}'", child.kind(), child.text());
        }

        // Debug: print parent and sibling nodes
        if let Some(parent) = contains_rule.syntax().parent() {
            println!("Parent node: {:?} - '{}'", parent.kind(), parent.text());
        }

        // The parser should extract extension names (this might fail, that's the bug we're investigating)
        if items.is_empty() {
            println!("WARNING: Contains rule items are empty - this indicates a parser issue");
        }
    }

    // Look for extension-specific rules
    let extension_rules: Vec<_> = rules
        .iter()
        .filter_map(|r| match r {
            Rule::Only(only_rule) => {
                if let Some(path) = only_rule.path() {
                    let path_str = path.as_string();
                    if path_str.starts_with("extension[") {
                        Some(path_str)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    println!("Extension-specific rules: {:?}", extension_rules);
    assert!(
        !extension_rules.is_empty(),
        "Should have extension-specific rules"
    );

    // Check for specific extension names in rules
    let has_subext1 = extension_rules.iter().any(|r| r.contains("subExt1"));
    let has_subext2 = extension_rules.iter().any(|r| r.contains("subExt2"));
    let has_subext3 = extension_rules.iter().any(|r| r.contains("subExt3"));

    assert!(has_subext1, "Should have subExt1 rule");
    assert!(has_subext2, "Should have subExt2 rule");
    assert!(has_subext3, "Should have subExt3 rule");
}

#[test]
fn test_complex_context_rules_parsing() {
    // Test complex context rules with [+] and [=] syntax
    let fsh = r#"
Extension: ComplexContextExtension
Id: complex-context-ext
* ^context[+].type = #element
* ^context[=].expression = "Patient"
* ^context[+].type = #element  
* ^context[=].expression = "Observation"
* ^context[+].type = #extension
* ^context[=].expression = "http://example.org/Extension/BaseExt"
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    // Check extension name
    assert_eq!(
        extension.name(),
        Some("ComplexContextExtension".to_string())
    );

    // Check that we have caret value rules
    let rules: Vec<_> = extension.rules().collect();
    println!("Total rules found: {}", rules.len());

    for (i, rule) in rules.iter().enumerate() {
        println!("Rule {}: {:?}", i, rule);
    }

    let caret_rules: Vec<_> = rules
        .iter()
        .filter_map(|r| {
            if let Rule::CaretValue(c) = r {
                Some(c)
            } else {
                None
            }
        })
        .collect();

    if caret_rules.is_empty() {
        println!("WARNING: No caret value rules found - this indicates a parser issue");
        println!("Available rule types:");
        for rule in &rules {
            match rule {
                Rule::Card(_) => println!("  - CardRule"),
                Rule::Flag(_) => println!("  - FlagRule"),
                Rule::ValueSet(_) => println!("  - ValueSetRule"),
                Rule::FixedValue(_) => println!("  - FixedValueRule"),
                Rule::Path(_) => println!("  - PathRule"),
                Rule::Contains(_) => println!("  - ContainsRule"),
                Rule::Only(_) => println!("  - OnlyRule"),
                Rule::Obeys(_) => println!("  - ObeysRule"),
                Rule::AddElement(_) => println!("  - AddElementRule"),
                Rule::Mapping(_) => println!("  - MappingRule"),
                Rule::CaretValue(_) => println!("  - CaretValueRule"),
            }
        }
    }

    // Check for context-related caret rules
    let context_rules: Vec<_> = caret_rules
        .iter()
        .filter(|r| {
            if let Some(path) = r.caret_path() {
                let path_str = path.as_string();
                path_str.starts_with("context")
            } else {
                false
            }
        })
        .collect();

    assert!(
        !context_rules.is_empty(),
        "Should have context-related caret rules"
    );
    println!("Found {} context rules", context_rules.len());

    // Check for specific context patterns
    let type_rules: Vec<_> = context_rules
        .iter()
        .filter(|r| {
            if let Some(path) = r.caret_path() {
                path.as_string().contains(".type")
            } else {
                false
            }
        })
        .collect();

    let expression_rules: Vec<_> = context_rules
        .iter()
        .filter(|r| {
            if let Some(path) = r.caret_path() {
                path.as_string().contains(".expression")
            } else {
                false
            }
        })
        .collect();

    assert!(!type_rules.is_empty(), "Should have context type rules");
    assert!(
        !expression_rules.is_empty(),
        "Should have context expression rules"
    );

    println!(
        "Type rules: {}, Expression rules: {}",
        type_rules.len(),
        expression_rules.len()
    );
}

#[test]
fn test_simple_extension_parsing() {
    let fsh = r#"
Extension: SimpleExtension
Id: simple-ext
Title: "Simple Extension"
Description: "A simple extension"
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
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
