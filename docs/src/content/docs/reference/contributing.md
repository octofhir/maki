---
title: Contributing
description: How to contribute to FSH Lint
---

Thank you for your interest in contributing to FSH Lint!

## Ways to Contribute

- Report bugs
- Suggest features
- Improve documentation
- Write code
- Create custom rules
- Help others in discussions

## Getting Started

### Prerequisites

- Rust 1.80 or later
- Git
- Familiarity with FSH (FHIR Shorthand)

### Development Setup

1. Fork the repository:
```bash
gh repo fork octofhir/maki
```

2. Clone your fork:
```bash
git clone https://github.com/YOUR_USERNAME/maki.git
cd maki
```

3. Build the project:
```bash
cargo build --workspace
```

4. Run tests:
```bash
cargo test --workspace
```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/my-feature
# or
git checkout -b fix/issue-123
```

### 2. Make Changes

Follow the project structure:
- `crates/maki-core` - Core linting engine
- `crates/maki-rules` - Rule engine and built-in rules
- `crates/maki-cli` - Command-line interface
- `crates/maki-devtools` - Developer tools

### 3. Write Tests

Add tests for new functionality:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_feature() {
        // Test implementation
    }
}
```

### 4. Run Tests

```bash
# Run all tests
cargo test --workspace

# Run specific test
cargo test --package maki-core --test my_test

# With output
cargo test -- --nocapture
```

### 5. Format Code

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
```

### 6. Commit Changes

Follow conventional commits:

```bash
git add .
git commit -m "feat: add new rule for profile validation"
git commit -m "fix: correct cardinality checking logic"
git commit -m "docs: update installation guide"
```

Commit types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `test`: Tests
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `chore`: Maintenance

### 7. Push and Create PR

```bash
git push origin feature/my-feature
```

Then create a pull request on GitHub.

## Code Guidelines

### Rust Style

Follow Rust API Guidelines:
- Use descriptive names
- Prefer iterators over loops
- Use `Result` for error handling
- Document public APIs
- Write integration tests

### Documentation

- Add doc comments to public APIs
- Include examples in doc comments
- Update user documentation
- Add changelog entries

### Testing

- Write unit tests for new code
- Add integration tests for features
- Include golden file tests for parser
- Test error cases

## Adding a New Rule

1. Create rule file in `crates/maki-rules/src/builtin/`:

```rust
use maki_core::{LintContext, Diagnostic, Severity};
use crate::Rule;

pub struct MyRule;

impl Rule for MyRule {
    fn name(&self) -> &str {
        "category/my-rule"
    }
    
    fn category(&self) -> RuleCategory {
        RuleCategory::Style
    }
    
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    
    fn lint(&self, context: &mut LintContext) {
        // Implementation
    }
}
```

2. Register in `crates/maki-rules/src/lib.rs`

3. Add tests in `crates/maki-rules/tests/`

4. Add documentation in `docs/`

## Review Process

1. Automated checks run (tests, clippy, fmt)
2. Maintainer reviews code
3. Address feedback
4. Approval and merge

## Community

- GitHub Discussions: Ask questions
- GitHub Issues: Report bugs
- Pull Requests: Contribute code

## License

By contributing, you agree your code will be licensed under MIT or Apache-2.0.

## Questions?

Open a discussion on GitHub or reach out to maintainers.
