use maki_core::cst::ast::{AstNode, Instance};
use maki_core::cst::parse_fsh;
use std::{env, fs};

fn main() {
    let path = env::args().nth(1).expect("usage: list_instances <file>");
    let content = fs::read_to_string(&path).expect("read file");
    let (cst, lexer_errors, parse_errors) = parse_fsh(&content);

    if !lexer_errors.is_empty() || !parse_errors.is_empty() {
        eprintln!("lexer errors: {}", lexer_errors.len());
        eprintln!("parse errors: {}", parse_errors.len());
    }

    for inst in cst.descendants().filter_map(Instance::cast) {
        let name = inst.name().unwrap_or_else(|| "<unknown>".to_string());
        println!("{name}");
    }
}
