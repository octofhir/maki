# MAKI - FSH Toolchain

**M**odern **A**nalysis and **K**it for **I**mplementation Guides

[![CI](https://github.com/octofhir/maki/actions/workflows/ci.yml/badge.svg)](https://github.com/octofhir/maki/actions/workflows/ci.yml)
[![Security Audit](https://github.com/octofhir/maki/actions/workflows/security-audit.yml/badge.svg)](https://github.com/octofhir/maki/actions/workflows/security-audit.yml)
[![codecov](https://codecov.io/gh/octofhir/maki/branch/main/graph/badge.svg)](https://codecov.io/gh/octofhir/maki)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

> **⚠️ ACTIVE DEVELOPMENT**: This project is currently in active development. APIs and features may change. Production use is not yet recommended.

A high-performance toolchain for FHIR Shorthand (FSH), written in Rust.

Part of the [OctoFHIR](https://github.com/octofhir) ecosystem.

## Features

### Current Features
- **SUSHI-Compatible Build**: Compile FSH to FHIR resources (drop-in replacement for SUSHI)
- **GoFSH**: Convert FHIR resources (JSON/XML) back to FSH
- **Fast Linting**: Built in Rust for maximum performance
- **Comprehensive Validation**: Built-in rules for FSH syntax and semantics
- **Smart Diagnostics**: Rich error messages with code frames
- **Auto-fix Capabilities**: Automatic fixes for many issues
- **Custom Rules**: Extend with GritQL-based pattern matching
- **Flexible Configuration**: JSON/TOML configuration files
- **Multiple Output Formats**: Human-readable, JSON, SARIF, GitHub Actions
- **Formatter**: Consistent code formatting for FSH files
- **Project Scaffolding**: Initialize new FSH projects with `maki init`

### Future Features (Planned)
- **LSP Server**: IDE support for FSH development
- **Test Framework**: Testing capabilities for FSH resources

## Quick Start

### Download Pre-built Binary

Download the latest binary for your platform from [GitHub Releases](https://github.com/octofhir/maki/releases/latest):

**Linux:**
```bash
# x86_64 (ARM64 not currently available)
wget https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64
chmod +x maki-linux-x64
sudo mv maki-linux-x64 /usr/local/bin/maki
```

**macOS:**
```bash
# Intel
curl -L https://github.com/octofhir/maki/releases/latest/download/maki-macos-x64 -o maki
chmod +x maki
sudo mv maki /usr/local/bin/

# Apple Silicon
curl -L https://github.com/octofhir/maki/releases/latest/download/maki-macos-arm64 -o maki
chmod +x maki
sudo mv maki /usr/local/bin/
```

**Windows:**

Download `maki-windows-x64.exe` or `maki-windows-arm64.exe` from the releases page and add it to your PATH.

### Build from Source

```bash
git clone https://github.com/octofhir/maki.git
cd maki
cargo build --release --bin maki

# Binary will be at: target/release/maki
```

## Usage

### Initialize Configuration

```bash
# Create a maki.yaml config file in the current directory
maki config init

# Initialize with JSON format
maki config init --format json
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

### Build FSH to FHIR (SUSHI-compatible)

```bash
# Build current directory
maki build

# Build with progress bar
maki build --progress

# Run linter before building
maki build --lint

# Format FSH files before building
maki build --format

# Clean output directory and rebuild
maki build --clean

# Strict mode (treat warnings as errors)
maki build --lint --strict

# Specify output directory
maki build --output ./my-output
```

### Convert FHIR to FSH (GoFSH)

```bash
# Convert FHIR resources in current directory
maki gofsh ./fsh-generated

# Specify output directory
maki gofsh ./fsh-generated -o ./input/fsh

# With FHIR dependencies
maki gofsh ./resources -d hl7.fhir.us.core@5.0.1

# Specify FHIR version
maki gofsh ./resources --fhir-version R5

# With progress reporting
maki gofsh ./resources --progress

# Organization strategies
maki gofsh ./resources --strategy type    # Group by FSH type
maki gofsh ./resources --strategy profile # Group by profile
maki gofsh ./resources --strategy single  # All in one file
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
# Validate your maki.yaml configuration
maki config validate
```

## Configuration

MAKI supports configuration files in YAML or JSON format. Place a `maki.yaml` (or `maki.json`) file in your project root:

```yaml
# maki.yaml
root: true

# FHIR package dependencies
dependencies:
  hl7.fhir.us.core: "6.1.0"
  hl7.terminology.r4: "5.3.0"

# Build configuration (SUSHI-compatible)
build:
  canonical: http://example.org/fhir/my-ig
  fhirVersion: ["4.0.1"]
  id: my.example.ig
  name: MyImplementationGuide
  title: My Example Implementation Guide
  version: "1.0.0"
  status: draft

# Linter configuration
linter:
  enabled: true
  rules:
    recommended: true
    correctness:
      duplicate-definition: error
      invalid-reference: error
    documentation:
      require-description: warn

# Formatter configuration
formatter:
  enabled: true
  indentSize: 2
  lineWidth: 100

# File patterns
files:
  include:
    - "input/fsh/**/*.fsh"
  exclude:
    - "**/node_modules/**"
```

### Configuration Discovery

MAKI searches for configuration files in this order:

1. `maki.yaml` / `maki.yml`
2. `maki.json`
3. `.makirc.json` (legacy)

The search walks up the directory tree until a config with `root: true` is found.

### Rule Severity Levels

- `error` - Fail the linting/build process
- `warn` - Show warning but don't fail
- `info` - Informational message
- `off` - Disable the rule

### Rule Categories

- **blocking** - Critical requirements that must pass first
- **correctness** - Syntax and semantic errors
- **suspicious** - Patterns that often indicate bugs
- **style** - Naming conventions and formatting
- **documentation** - Metadata requirements

See [examples/configs/](examples/configs/) for comprehensive configuration examples.

## Built-in Rules

MAKI includes comprehensive built-in rules organized by category:

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

3. Configure MAKI to use your custom rules in `maki.yaml`:

   ```yaml
   linter:
     ruleDirectories:
       - "./custom-rules"
   ```

## Integration with CI/CD

### GitHub Actions

```yaml
name: MAKI

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download MAKI
        run: |
          curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
          chmod +x maki

      - name: Build FSH to FHIR
        run: ./maki build --lint

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
    - curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
    - chmod +x maki
    - ./maki build --lint
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
  - FHIR exporters (Profiles, Extensions, ValueSets, CodeSystems, Instances)
  - Canonical package management

- **`maki-decompiler`** - FHIR to FSH decompiler (GoFSH functionality):
  - Resource processors (StructureDefinitions, ValueSets, CodeSystems, Instances)
  - FSH rule extraction and optimization
  - Multiple file organization strategies

- **`maki-rules`** - Rule engine and built-in rules:
  - GritQL-based pattern matching
  - AST-based rule engine
  - Built-in rule implementations (naming, metadata, cardinality, etc.)
  - Rule registry and management

- **`maki-cli`** - Command-line interface (binary: `maki`):
  - Build command (SUSHI-compatible FSH to FHIR compilation)
  - GoFSH command (FHIR to FSH conversion)
  - Lint command
  - Format command
  - Init command (project scaffolding)
  - Rules management
  - Configuration management

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

**https://octofhir.github.io/maki/**

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
