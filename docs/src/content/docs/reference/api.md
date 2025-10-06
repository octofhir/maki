---
title: API Documentation
description: Rust API documentation
---

FSH Lint provides a Rust API for programmatic use.

## Crate Structure

```
fsh-lint/
├── fsh-lint-core     # Core linting engine
├── fsh-lint-rules    # Rule engine and built-in rules
└── fsh-lint-cli      # Command-line interface
```

## Using as a Library

Add to `Cargo.toml`:

```toml
[dependencies]
fsh-lint-core = "0.1"
fsh-lint-rules = "0.1"
```

## Basic Usage

```rust
use fsh_lint_core::{LintContext, Parser};
use fsh_lint_rules::RuleRegistry;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse FSH file
    let source = std::fs::read_to_string("profile.fsh")?;
    let ast = Parser::parse(&source)?;
    
    // Create lint context
    let mut context = LintContext::new(&source, &ast);
    
    // Load rules
    let registry = RuleRegistry::default();
    
    // Run linting
    let diagnostics = registry.lint(&mut context)?;
    
    // Print diagnostics
    for diagnostic in diagnostics {
        println!("{}", diagnostic);
    }
    
    Ok(())
}
```

## API Reference

### fsh-lint-core

#### Parser

```rust
pub struct Parser;

impl Parser {
    pub fn parse(source: &str) -> Result<Ast, ParseError>;
    pub fn parse_with_recovery(source: &str) -> (Option<Ast>, Vec<ParseError>);
}
```

#### LintContext

```rust
pub struct LintContext<'a> {
    source: &'a str,
    ast: &'a Ast,
    // ...
}

impl<'a> LintContext<'a> {
    pub fn new(source: &'a str, ast: &'a Ast) -> Self;
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic);
    pub fn diagnostics(&self) -> &[Diagnostic];
}
```

#### Diagnostic

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub location: Location,
    pub suggestions: Vec<Suggestion>,
}

pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}
```

### fsh-lint-rules

#### RuleRegistry

```rust
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
}

impl RuleRegistry {
    pub fn default() -> Self;
    pub fn new() -> Self;
    pub fn add_rule(&mut self, rule: Box<dyn Rule>);
    pub fn lint(&self, context: &mut LintContext) -> Result<Vec<Diagnostic>>;
}
```

#### Rule Trait

```rust
pub trait Rule {
    fn name(&self) -> &str;
    fn category(&self) -> RuleCategory;
    fn severity(&self) -> Severity;
    fn lint(&self, context: &mut LintContext);
}
```

## Custom Rules

Implement the `Rule` trait:

```rust
use fsh_lint_core::{LintContext, Diagnostic, Severity};
use fsh_lint_rules::Rule;

struct MyCustomRule;

impl Rule for MyCustomRule {
    fn name(&self) -> &str {
        "custom/my-rule"
    }
    
    fn category(&self) -> RuleCategory {
        RuleCategory::Style
    }
    
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    
    fn lint(&self, context: &mut LintContext) {
        // Implement linting logic
        for profile in context.ast().profiles() {
            if !profile.name().ends_with("Profile") {
                context.add_diagnostic(Diagnostic {
                    severity: self.severity(),
                    message: format!("Profile '{}' should end with 'Profile'", profile.name()),
                    location: profile.location(),
                    suggestions: vec![],
                });
            }
        }
    }
}
```

## Full API Documentation

Generate complete API docs:

```bash
cargo doc --open --no-deps -p fsh-lint-core -p fsh-lint-rules
```

Online documentation: https://docs.rs/fsh-lint-core
