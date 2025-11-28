use maki_core::cst::parse_fsh;
use std::{env, fs};

fn main() {
    let path = env::args().nth(1).expect("usage: parse_file <file>");
    let content = fs::read_to_string(&path).expect("read file");
    let (_cst, lexer_errors, parse_errors) = parse_fsh(&content);

    if lexer_errors.is_empty() && parse_errors.is_empty() {
        println!("{path}: OK");
    } else {
        println!("{path}:");
        for err in lexer_errors {
            println!("  LEX  at {}..{}: {}", err.span.start, err.span.end, err.message);
        }
        for err in parse_errors {
            println!(
                "  PARSE at line {}, col {}: {}",
                err.line, err.col, err.message
            );
        }
    }
}
