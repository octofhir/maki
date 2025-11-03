//! Debug tests for parser issues

use maki_core::cst::{ast::*, parse_fsh, FshSyntaxKind};

#[test]
fn debug_contains_rule_parsing() {
    let fsh = r#"
Extension: TestExt
* extension contains subExt1 0..1 MS and subExt2 1..1
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    println!("=== FULL DOCUMENT ===");
    println!("Text: '{}'", root.text());
    
    // Walk the syntax tree and print all nodes
    fn walk_tree(node: &maki_core::cst::FshSyntaxNode, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{}Node: {:?} - '{}'", indent, node.kind(), node.text());
        
        for child in node.children() {
            walk_tree(&child, depth + 1);
        }
    }
    
    println!("\n=== SYNTAX TREE ===");
    walk_tree(&root, 0);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    println!("\n=== EXTENSION RULES ===");
    let rules: Vec<_> = extension.rules().collect();
    for (i, rule) in rules.iter().enumerate() {
        println!("Rule {}: {:?}", i, rule);
        match rule {
            Rule::Contains(contains_rule) => {
                println!("  Contains rule text: '{}'", contains_rule.syntax().text());
                println!("  Contains rule items: {:?}", contains_rule.items());
            }
            _ => {}
        }
    }
}

#[test]
fn debug_multiline_contains_rule_parsing() {
    let fsh = r#"
Extension: TestExt
* extension contains
    subExt1 0..1 MS and
    subExt2 1..1
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    println!("=== MULTILINE FULL DOCUMENT ===");
    println!("Text: '{}'", root.text());
    
    // Walk the syntax tree and print all nodes
    fn walk_tree(node: &maki_core::cst::FshSyntaxNode, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{}Node: {:?} - '{}'", indent, node.kind(), node.text());
        
        for child in node.children() {
            walk_tree(&child, depth + 1);
        }
    }
    
    println!("\n=== MULTILINE SYNTAX TREE ===");
    walk_tree(&root, 0);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    println!("\n=== MULTILINE EXTENSION RULES ===");
    let rules: Vec<_> = extension.rules().collect();
    for (i, rule) in rules.iter().enumerate() {
        println!("Rule {}: {:?}", i, rule);
        match rule {
            Rule::Contains(contains_rule) => {
                println!("  Contains rule text: '{}'", contains_rule.syntax().text());
                println!("  Contains rule items: {:?}", contains_rule.items());
            }
            _ => {}
        }
    }
}