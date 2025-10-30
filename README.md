# MAKI - FSH Toolchain

**M**odern **A**nalysis and **K**it for **I**mplementation Guides

[![CI](https://github.com/octofhir/maki-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/octofhir/maki-rs/actions/workflows/ci.yml)
[![Security Audit](https://github.com/octofhir/maki-rs/actions/workflows/security-audit.yml/badge.svg)](https://github.com/octofhir/maki-rs/actions/workflows/security-audit.yml)
[![codecov](https://codecov.io/gh/octofhir/maki-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/octofhir/maki-rs)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A high-performance toolchain for FHIR Shorthand (FSH), written in Rust.

Part of the [OctoFHIR](https://github.com/octofhir) ecosystem.

## Features

### Current Features
- **Fast Linting**: Built in Rust for maximum performance
- **Comprehensive Validation**: Built-in rules for FSH syntax and semantics
- **Smart Diagnostics**: Rich error messages with code frames
- **Auto-fix Capabilities**: Automatic fixes for many issues
- **Custom Rules**: Extend with GritQL-based pattern matching
- **Flexible Configuration**: JSON/TOML configuration files
- **Multiple Output Formats**: Human-readable, JSON, SARIF, GitHub Actions
- **Formatter**: Consistent code formatting for FSH files

### Future Features (Planned)
- **SUSHI-Compatible Build**: Compile FSH to FHIR resources
- **LSP Server**: IDE support for FSH development
- **Test Framework**: Testing capabilities for FSH resources
- **Project Scaffolding**: Initialize new FSH projects

## Quick Start

### Download Pre-built Binary

Download the latest binary for your platform from [GitHub Releases](https://github.com/octofhir/maki-rs/releases/latest):

**Linux:**
```bash
# x86_64 (ARM64 not currently available)
wget https://github.com/octofhir/maki-rs/releases/latest/download/maki-linux-x64
chmod +x maki-linux-x64
sudo mv maki-linux-x64 /usr/local/bin/maki
```

**macOS:**
```bash
# Intel
curl -L https://github.com/octofhir/maki-rs/releases/latest/download/maki-macos-x64 -o maki
chmod +x maki
sudo mv maki /usr/local/bin/

# Apple Silicon
curl -L https://github.com/octofhir/maki-rs/releases/latest/download/maki-macos-arm64 -o maki
chmod +x maki
sudo mv maki /usr/local/bin/
```

**Windows:**

Download `maki-windows-x64.exe` or `maki-windows-arm64.exe` from the releases page and add it to your PATH.

### Build from Source

```bash
git clone https://github.com/octofhir/maki-rs.git
cd maki-rs
cargo build --release --bin maki

# Binary will be at: target/release/maki
```

## Usage

### Initialize Configuration

```bash
# Create a .makirc.json config file in the current directory
maki config init

# Initialize with JSONC format (supports comments)
maki config init --format jsonc
```

### Lint Files

```bash
# Lint specific files
maki lint file1.fsh file2.fsh

# Lint all FSH files in a directory
maki lint input/

# Lint with glob patterns
maki lint "**/*.fsh"

# Auto-fix issues
maki lint --fix input/

# Output as JSON
maki lint --format json input/

# Output as SARIF (for CI integration)
maki lint --format sarif input/
```

### List Available Rules

```bash
# List all rules
maki rules

# List with detailed information
maki rules --detailed

# Filter by category
maki rules --category documentation
maki rules --category correctness
```

### Validate Configuration

```bash
# Validate your .makirc.json
maki config validate
```

## Configuration

FSH Lint supports configuration files in JSON or JSONC format. Place a `.makirc.json` file in your project root:

```jsonc
{
  // Enable/disable specific rules
  "rules": {
    "documentation/require-description": "error",
    "naming/profile-pascal-case": "warn",
    "correctness/valid-cardinality": "error"
  },

  // File patterns to include/exclude
  "include": ["input/**/*.fsh"],
  "exclude": ["**/node_modules/**", "**/temp/**"],

  // Custom rule directories
  "customRules": ["./custom-rules"],

  // Formatter options
  "formatter": {
    "indentWidth": 2,
    "lineWidth": 100
  }
}
```

### Rule Severity Levels

- `"error"` - Fail the linting process
- `"warn"` - Show warning but don't fail
- `"off"` - Disable the rule

### Extending Configurations

You can extend base configurations:

```jsonc
{
  "extends": ["./base-config.json"],
  "rules": {
    // Override specific rules
    "naming/profile-pascal-case": "off"
  }
}
```

## Built-in Rules

FSH Lint includes comprehensive built-in rules organized by category:

### Documentation Rules
- `documentation/require-description` - Require Description for all resources
- `documentation/require-title` - Require Title for ValueSets and CodeSystems

### Naming Rules
- `naming/profile-pascal-case` - Profile names must use PascalCase
- `naming/valueset-pascal-case` - ValueSet names must use PascalCase
- `naming/extension-kebab-case` - Extension names should use kebab-case

### Correctness Rules
- `correctness/valid-cardinality` - Cardinality must be valid (e.g., 1..1, 0..*, 1..*)
- `correctness/no-duplicate-rules` - No duplicate constraint rules
- `correctness/valid-fhir-path` - FHIRPath expressions must be valid

### Suspicious Rules
- `suspicious/unused-ruleset` - Detect unused RuleSets
- `suspicious/unreferenced-profile` - Detect unreferenced profiles

For a complete list, run `maki rules --detailed`.

## Custom Rules with GritQL

You can write custom rules using [GritQL](https://docs.grit.io/language) pattern matching:

1. Create a custom rules directory:
   ```bash
   mkdir custom-rules
   ```

2. Create a `.grit` file with your rule:
   ```grit
   language fsh

   pattern no_test_profiles() {
     `Profile: $name` where {
       $name <: r"^Test"
     }
   }
   ```

3. Configure FSH Lint to use your custom rules:
   ```jsonc
   {
     "customRules": ["./custom-rules"]
   }
   ```

## Integration with CI/CD

### GitHub Actions

```yaml
name: FSH Lint

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download FSH Lint
        run: |
          curl -L https://github.com/octofhir/maki-rs/releases/latest/download/maki-linux-x64 -o maki
          chmod +x maki

      - name: Lint FSH files
        run: ./maki lint --format sarif input/ > results.sarif

      - name: Upload SARIF results
        uses: github/codeql-action/upload-sarif@v3
        if: always()
        with:
          sarif_file: results.sarif
```

### GitLab CI

```yaml
maki:
  image: ubuntu:latest
  script:
    - curl -L https://github.com/octofhir/maki-rs/releases/latest/download/maki-linux-x64 -o maki
    - chmod +x maki
    - ./maki lint input/
```

## Project Structure

This is a Rust workspace with the following crates:

### Core Crates

- **`maki-core`** - Core library containing:
  - CST/AST parser (Rowan-based lossless syntax tree)
  - Semantic analyzer
  - Diagnostic system
  - Autofix engine
  - Formatter
  - FHIR exporters (stub for future implementation)
  - Canonical package management

- **`maki-rules`** - Rule engine and built-in rules:
  - GritQL-based pattern matching
  - AST-based rule engine
  - Built-in rule implementations (naming, metadata, cardinality, etc.)
  - Rule registry and management

- **`maki-cli`** - Command-line interface (binary: `maki`):
  - Lint command
  - Format command
  - Rules management
  - Configuration management
  - Stubs for future commands (build, init, test, lsp)

### Future Crates (Stubs)

- **`maki-lsp`** - Language Server Protocol implementation (future)
- **`maki-formatter`** - Formatter API wrapper (wraps maki-core formatter)
- **`maki-test`** - Testing framework for FSH resources (future)

### Development Crates

- **`maki-devtools`** - Developer tools for schema generation and docs
- **`maki-bench`** - Performance benchmarks
- **`maki-integration-tests`** - Integration test suite

## Documentation

For comprehensive documentation, guides, and API references, visit:

**https://octofhir.github.io/maki-rs/**

Topics covered:
- Getting Started Guide
- Writing Custom Rules
- Configuration Reference
- Rule Development Guide
- Architecture Overview
- Contributing Guide

## Performance

- Parses and lints 1000+ FSH files in under 5 seconds
- Uses less than 100MB memory for typical projects
- Parallel file processing with Rayon
- Efficient CST with incremental reparsing (future)
- Caching to avoid redundant work

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Development setup
- Code style guidelines
- Testing requirements
- Pull request process
- Adding new rules

## Benchmarking

Run performance benchmarks:

```bash
cargo bench
```

Results are saved to `target/criterion/` with detailed HTML reports.

## Testing

```bash
# Run all tests
cargo test --workspace

# Run integration tests
cargo test --test integration_test

# Run specific crate tests
cargo test --package maki-core
cargo test --package maki-rules

# Run with verbose output
cargo test -- --nocapture
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- Built with [Rowan](https://github.com/rust-analyzer/rowan) for lossless syntax trees
- Uses [GritQL](https://docs.grit.io/) for pattern matching
- Part of the [OctoFHIR](https://github.com/octofhir) ecosystem

## Related Projects

- [SUSHI](https://github.com/FHIR/sushi) - Official FSH compiler
- [GoFSH](https://github.com/FHIR/GoFSH) - Converts FHIR to FSH
- [FSH Online](https://fshschool.org/) - Try FSH in your browser

---

Made with ❤️ by the OctoFHIR Team
