# Contributing to FSH Lint

Thank you for your interest in contributing to FSH Lint! This document provides guidelines and instructions for contributing.

## Code of Conduct

We are committed to providing a welcoming and inspiring community for all. Please be respectful and constructive in all interactions.

## Development Setup

### Prerequisites

- Rust 1.70 or later (Rust Edition 2024)
- Git
- A code editor (we recommend VS Code with rust-analyzer)

### Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/maki-rs.git
   cd maki-rs
   ```

3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/octofhir/maki-rs.git
   ```

4. Build the project:
   ```bash
   cargo build --workspace
   ```

5. Run the tests to ensure everything works:
   ```bash
   cargo test --workspace
   ```

## Project Structure

This is a Rust workspace with the following crates:

```
maki-rs/
├── crates/
│   ├── maki-core/        # Core linting engine
│   │   ├── src/
│   │   │   ├── cst/           # Concrete Syntax Tree implementation
│   │   │   ├── config/        # Configuration system
│   │   │   ├── diagnostics/   # Diagnostic system
│   │   │   ├── parser.rs      # Main parser
│   │   │   ├── semantic.rs    # Semantic analysis
│   │   │   ├── autofix.rs     # Auto-fix engine
│   │   │   └── formatter.rs   # Code formatter
│   │   └── tests/             # Core tests
│   │
│   ├── maki-rules/        # Rule engine and built-in rules
│   │   ├── src/
│   │   │   ├── builtin/       # Built-in rules
│   │   │   ├── gritql/        # GritQL integration
│   │   │   └── engine.rs      # Rule execution engine
│   │   └── tests/             # Rule tests
│   │
│   ├── maki-cli/          # Command-line interface
│   │   ├── src/
│   │   │   ├── commands.rs    # CLI commands
│   │   │   └── output.rs      # Output formatting
│   │   └── tests/             # CLI tests
│   │
│   └── maki-devtools/     # Developer tools
│       └── src/
│           └── schema.rs      # Schema generation
│
├── tests/                     # Integration tests
├── benches/                   # Performance benchmarks
├── examples/                  # Example FSH files
├── docs/                      # Documentation site (Astro)
└── tasks/                     # Implementation task guides
```

## Development Workflow

### Making Changes

1. Create a new branch for your feature or bug fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes, following the code style guidelines below

3. Add tests for your changes

4. Run the test suite:
   ```bash
   cargo test --workspace
   ```

5. Run the formatter:
   ```bash
   cargo fmt --all
   ```

6. Run Clippy to catch common mistakes:
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```

7. Commit your changes with a descriptive commit message:
   ```bash
   git commit -m "feat: add new rule for validating X"
   ```

### Commit Message Format

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Adding or updating tests
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Maintenance tasks

Examples:
```
feat: add support for extension validation
fix: correct cardinality parsing for ranges
docs: update configuration examples
test: add tests for GritQL integration
refactor: simplify diagnostic rendering
perf: optimize CST traversal
chore: update dependencies
```

## Adding a New Built-in Rule

1. Create a new file in `crates/maki-rules/src/builtin/` (or add to an existing category file)

2. Implement your rule:
   ```rust
   use crate::engine::{Rule, RuleCategory, RuleSeverity, RuleContext};
   use maki_core::diagnostics::Diagnostic;

   pub struct MyNewRule;

   impl Rule for MyNewRule {
       fn id(&self) -> &'static str {
           "category/rule-name"
       }

       fn name(&self) -> &'static str {
           "Descriptive Rule Name"
       }

       fn description(&self) -> &'static str {
           "Detailed description of what this rule checks"
       }

       fn category(&self) -> RuleCategory {
           RuleCategory::Correctness
       }

       fn default_severity(&self) -> RuleSeverity {
           RuleSeverity::Error
       }

       fn execute(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
           // Implement your rule logic here
           vec![]
       }
   }
   ```

3. Register your rule in `crates/maki-rules/src/builtin/mod.rs`:
   ```rust
   pub fn all_builtin_rules() -> Vec<Box<dyn Rule>> {
       vec![
           // ... existing rules
           Box::new(MyNewRule),
       ]
   }
   ```

4. Add tests for your rule in `crates/maki-rules/tests/`

5. Update documentation by running:
   ```bash
   cargo run --bin maki-devtools -- generate-rule-docs
   ```

## Writing Custom GritQL Rules

GritQL rules are pattern-matching rules that can be written without Rust code:

1. Create a `.grit` file in your custom rules directory

2. Define your pattern:
   ```grit
   language fsh

   pattern my_custom_rule() {
     `Profile: $name` where {
       // Your pattern conditions
     }
   }
   ```

3. Test your GritQL pattern in `crates/maki-rules/tests/gritql_integration_test.rs`

See [Writing Custom Rules Guide](https://octofhir.github.io/maki-rs/guides/custom-rules/) for more details.

## Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test --package maki-core
cargo test --package maki-rules
cargo test --package maki-cli

# Run a specific test
cargo test test_name

# Run integration tests
cargo test --test integration_test

# Run tests with output
cargo test -- --nocapture

# Run tests with specific features
cargo test --all-features
```

## Running Benchmarks

Performance benchmarks use Criterion:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench bench_parser

# View HTML reports
open target/criterion/report/index.html
```

## Code Style

### Rust Code Style

- Use `rustfmt` for formatting (run `cargo fmt --all`)
- Follow Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- Use meaningful variable and function names
- Add doc comments for public APIs
- Keep functions focused and small
- Prefer iterators over explicit loops
- Use `Result<T>` and `?` operator for error handling

### Documentation Comments

Use doc comments for public items:

```rust
/// Parses FSH source code into a CST.
///
/// # Arguments
///
/// * `source` - The FSH source code to parse
///
/// # Returns
///
/// Returns a `ParseResult` containing the CST or parse errors
///
/// # Examples
///
/// ```
/// let parser = Parser::new();
/// let result = parser.parse("Profile: MyProfile\nParent: Patient");
/// ```
pub fn parse(&self, source: &str) -> ParseResult {
    // ...
}
```

## Testing Guidelines

### Unit Tests

Place unit tests in the same file as the code being tested:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        assert_eq!(my_function(input), expected);
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory for testing cross-crate functionality.

### Snapshot Testing

Use `insta` for snapshot testing:

```rust
use insta::assert_snapshot;

#[test]
fn test_diagnostic_output() {
    let output = format_diagnostic(&diagnostic);
    assert_snapshot!(output);
}
```

Update snapshots with:
```bash
cargo insta test
cargo insta review
```

## Continuous Integration (CI)

All pull requests must pass automated CI checks before merging. Our CI pipeline includes:

### Required Checks

1. **Format Check**: Code must be formatted with `cargo fmt`
   ```bash
   cargo fmt --all -- --check
   ```

2. **Clippy Lints**: Code must pass Clippy with no warnings
   ```bash
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```

3. **Tests**: All tests must pass on Linux, macOS, and Windows
   ```bash
   cargo test --workspace
   ```

4. **Build**: Project must compile successfully
   ```bash
   cargo build --workspace --release
   ```

5. **Security Audit**: Dependencies must pass security audit
   - Runs automatically on dependency changes
   - Uses `cargo-audit` to check for known vulnerabilities

### Optional Checks

- **Nightly Rust**: Tests run on nightly but failures don't block merging
- **Benchmarks**: Performance benchmarks run but don't block merging
- **Coverage**: Code coverage is tracked but doesn't block merging (target: ≥80%)

### Running CI Checks Locally

Before submitting a PR, run these commands locally to ensure CI will pass:

```bash
# Run all checks at once
cargo fmt --all -- --check && \
cargo clippy --workspace --all-targets --all-features -- -D warnings && \
cargo test --workspace && \
cargo build --workspace --release
```

### CI Workflow Status

You can see the status of CI checks in your pull request:
- ✅ All checks passed - ready to merge
- ❌ Checks failed - review the logs and fix issues
- 🟡 Checks running - wait for completion

## Pull Request Process

1. Ensure all CI checks pass (see above)
2. Update documentation if needed
3. Add an entry to CHANGELOG.md under "Unreleased"
4. Push your changes to your fork
5. Create a pull request from your branch to `main`
6. Fill out the pull request template with:
   - Clear description of changes
   - Link to any related issues
   - Testing performed
   - Screenshots if applicable

### Pull Request Review

- PRs require at least one approval from a maintainer
- All CI checks must pass before merging
- Address review feedback by pushing new commits
- Once approved and CI passes, a maintainer will merge your PR

## Issue Guidelines

### Reporting Bugs

When reporting bugs, please include:

- FSH Lint version (`maki --version`)
- Operating system
- Minimal reproducible example
- Expected vs actual behavior
- Error messages or diagnostic output

### Requesting Features

When requesting features:

- Describe the use case
- Explain why this would be valuable
- Provide examples if possible
- Consider whether it could be implemented as a custom rule

## Documentation

Documentation is built with [Astro](https://astro.build/) and [Starlight](https://starlight.astro.build/).

### Building Docs Locally

```bash
cd docs
npm install
npm run dev
```

Visit http://localhost:4321 to view the docs.

### Updating Docs

- User guides: `docs/src/content/docs/guides/`
- API reference: `docs/src/content/docs/api/`
- Configuration: `docs/src/content/docs/configuration/`

## Getting Help

- **Documentation**: https://octofhir.github.io/maki-rs/
- **GitHub Issues**: https://github.com/octofhir/maki-rs/issues
- **Discussions**: https://github.com/octofhir/maki-rs/discussions

## Useful Resources

### Rust Development
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)

### Parser/Compiler Development
- [Rowan Documentation](https://github.com/rust-analyzer/rowan)
- [Chumsky Parser Combinator](https://github.com/zesterer/chumsky)
- [Crafting Interpreters](https://craftinginterpreters.com/)

### FHIR Shorthand
- [FSH Specification](https://hl7.org/fhir/uv/shorthand/)
- [SUSHI Compiler](https://github.com/FHIR/sushi)
- [FSH School](https://fshschool.org/)

## License

By contributing to FSH Lint, you agree that your contributions will be licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE))
- MIT License ([LICENSE-MIT](LICENSE))

at the option of the user.

---

Thank you for contributing to FSH Lint! 🎉
