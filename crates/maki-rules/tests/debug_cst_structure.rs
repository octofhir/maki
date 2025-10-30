//! Debug test to understand CST structure for profiles

use grit_util::{Ast, AstNode};
use maki_rules::gritql::{FshGritNode, FshGritTree};

#[test]
fn debug_profile_cst_structure() {
    let source = r#"
Profile: myPatientProfile
Parent: Patient
"#;

    let tree = FshGritTree::parse(source);
    let root = tree.root_node();

    println!("\n=== CST Structure ===");
    print_node(&root, 0);
}

fn print_node(node: &FshGritNode, indent: usize) {
    let indent_str = "  ".repeat(indent);
    let text = node.text().unwrap_or(std::borrow::Cow::Borrowed("<error>"));
    let kind = node.kind();
    let byte_range = node.byte_range();

    println!(
        "{}Kind: {:?}, Range: {:?}, Text: {:?}",
        indent_str,
        kind,
        byte_range,
        text.trim()
    );

    for child in node.children() {
        print_node(&child, indent + 1);
    }
}
