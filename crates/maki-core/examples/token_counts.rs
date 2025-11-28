use maki_core::cst::{FshSyntaxKind, lex_with_trivia};
use std::{env, fs};

fn main() {
    let path = env::args().nth(1).expect("usage: token_counts <file>");
    let content = fs::read_to_string(&path).expect("read file");
    let (tokens, lex_errors) = lex_with_trivia(&content);
    println!("tokens: {}", tokens.len());
    println!("lexer errors: {}", lex_errors.len());

    let mut counts = std::collections::HashMap::new();
    let mut instance_text = 0usize;
    for t in &tokens {
        *counts.entry(t.kind).or_insert(0usize) += 1;
        if t.text == "Instance" {
            instance_text += 1;
        }
    }

    for (kind, count) in counts.iter() {
        if *count > 0 && (*kind == FshSyntaxKind::InstanceKw || *kind == FshSyntaxKind::RulesetKw) {
            println!("{:?}: {}", kind, count);
        }
    }

    if let Some(last) = tokens.last() {
        println!(
            "last token kind: {:?}, span: {:?}, text: {}",
            last.kind, last.span, last.text
        );
    }
    println!("tokens with text 'Instance': {}", instance_text);

    for t in tokens.iter().filter(|t| t.text == "Instance") {
        println!("Instance token at span {:?} kind {:?}", t.span, t.kind);
    }

    println!("Tokens around 10900..11100:");
    for t in tokens
        .iter()
        .filter(|t| t.span.start >= 10900 && t.span.start <= 11100)
    {
        println!("{:?} {:?} {:?}", t.kind, t.span, t.text);
    }
}
