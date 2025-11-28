use maki_core::cst::ast::{AstNode, Profile};
use maki_core::cst::parse_fsh;
use std::{env, fs};

fn main() {
    let path = env::args().nth(1).expect("usage: print_parents <file>");
    let content = fs::read_to_string(&path).expect("read file");
    let (cst, _lexer_errors, _parse_errors) = parse_fsh(&content);

    for profile in cst.children().filter_map(Profile::cast) {
        let name = profile.name().unwrap_or_else(|| "<unknown>".to_string());
        let parent = profile.parent().and_then(|p| p.value()).unwrap_or_default();
        println!("{name}: {parent}");
    }
}
