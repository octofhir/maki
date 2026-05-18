# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

maki is a high-performance drop-in replacement for SUSHI (https://github.com/FHIR/sushi) written in Rust, part of the OctoFHIR ecosystem. It compiles FHIR Shorthand (FSH) files into FHIR resources with additional features including:

- **FSH to FHIR compilation** - Core functionality that transforms FSH definitions into FHIR JSON resources
- **Linting** - Comprehensive syntax checking, semantic validation, and rule-based analysis
- **Formatting** - Automatic code formatting with configurable style options
- **Autofix** - Automatic correction of common issues and violations
- **Language Server Protocol (LSP)** - Editor integration for real-time diagnostics and formatting (planned)

**Key References:**
- **FHIR Shorthand (FSH) Specification**: <https://hl7.org/fhir/uv/shorthand/>
- **SUSHI (reference implementation)**: <https://github.com/FHIR/sushi>

## Workspace Structure

This is a Rust workspace with multiple crates organized for the full MAKI toolchain:

### Core Crates

- **`maki-core`** - Core library containing:
  - Parser (Rowan-based CST for lossless parsing)
  - CST/AST definitions
  - Semantic analyzer
  - Diagnostic system
  - Autofix engine
  - Formatter
  - Execution engine
  - FHIR exporters (stub for Task 28)
  - Canonical package management

- **`maki-rules`** - Rule engine and built-in rules:
  - GritQL-based pattern matching
  - AST-based rule engine
  - Built-in rule implementations (naming, metadata, cardinality, etc.)
  - Rule registry and management

- **`maki-cli`** - Command-line interface (binary: `maki`):
  - Current commands: lint, format, rules, config
  - Future commands (stubs): build, init, test, lsp

### Future Crates (Stubs for upcoming features)

- **`maki-lsp`** - Language Server Protocol implementation (Task 31)
- **`maki-formatter`** - Formatter API wrapper (wraps maki-core formatter)
- **`maki-test`** - Testing framework for FSH resources (Task 34)

### Development Crates

- **`maki-devtools`** - Developer tools for schema generation and docs
- **`maki-bench`** - Performance benchmarks
- **`maki-integration-tests`** - Integration test suite

## Important

YOU don't create useless summary files and don't spend time for it.

## Common Commands

### Building

```bash
# Build entire workspace
cargo build --workspace

# Build for release
cargo build --workspace --release
```

### Running the CLI

```bash
# Run with cargo
cargo run --bin maki -- --help

# Lint files
cargo run --bin maki -- lint examples/
./target/debug/maki lint examples/

# List available rules
cargo run --bin maki -- rules
./target/debug/maki rules --detailed --category documentation

# Lint with automatic fixes
cargo run --bin maki -- lint --fix examples/
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test --package maki-core
cargo test --package maki-rules

# Run a specific test
cargo test --package maki-rules --test builtin_rules_test

# Run with verbose output
cargo test -- --nocapture
```

### Development Tools

```bash
# Check code without building
cargo check --workspace

# Clean build artifacts
cargo clean

# View dependency tree
cargo tree
```

## Architecture Overview

### Parsing Pipeline

1. **Lexer** (`cst/lexer.rs`) - Tokenizes FSH source with trivia (whitespace, comments)
2. **Parser** (`cst/parser.rs`) - Builds lossless Concrete Syntax Tree (CST) using Rowan library
3. **CST** (`cst/nodes.rs`) - Rowan-based tree with green/red node pattern:
   - **Green Tree**: Immutable, position-independent storage with all trivia preserved
   - **Red Tree**: Dynamic view with parent pointers for efficient traversal
   - **Lossless Property**: `parse(source).text() == source`
4. **Typed AST Layer** (`cst/ast.rs`) - High-level API over CST for semantic analysis
5. **Semantic Analyzer** (`semantic.rs`) - Builds semantic model with:
   - FHIR resource tracking
   - Symbol table for cross-references
   - Type information

### Rule Execution

Rules can be implemented in two ways:

1. **GritQL-based rules** - Pattern matching using GritQL syntax (`gritql/*.rs`)
   - Adapts our CST to GritQL's tree interface via `FshGritTree` and `FshGritNode`
   - Can match on structure AND trivia (comments, whitespace)
   - Supports variable captures and complex predicates
2. **CST-based rules** - Direct CST traversal and analysis (`builtin/*.rs`)
   - Uses Rowan's typed API for efficient tree walking
   - Access to all source information including trivia
   - Can implement precise autofixes that preserve formatting

The rule engine (`engine.rs`) manages rule discovery, compilation, and execution. Rules are categorized as:

- **Blocking rules** - Must pass before other rules run (critical validations)
- **Correctness rules** - Syntax and semantic violations
- **Suspicious rules** - Patterns that often indicate bugs
- **Style rules** - Naming conventions and formatting
- **Documentation rules** - Metadata and guidance requirements

### Diagnostic System

The diagnostic system (`diagnostics.rs`) provides:

- Rich error reporting with source locations
- Multiple output formats (human, JSON, SARIF, GitHub Actions)
- Suggestions and automatic fixes
- Severity levels (Error, Warning, Info, Hint)

### Autofix Engine

The autofix engine (`autofix.rs`) handles:

- Safe vs unsafe fix classification
- Conflict detection and resolution
- Dry-run previews
- Rollback capabilities

## Key Implementation Details

### Built-in Rules Location

Built-in rules are organized in `crates/maki-rules/src/builtin/`:

- `naming.rs` - Naming convention rules
- `metadata.rs` - Metadata requirement rules
- `profile.rs` - Profile-specific rules
- `cardinality.rs` - Cardinality validation rules
- `required_fields.rs` - Required field validation
- `duplicates.rs` - Duplicate detection rules
- `binding.rs` - Value set binding rules

### CST Architecture (`crates/maki-core/src/cst/`)

**Concrete Syntax Tree** using Rowan library:

- **Green Tree** (immutable, shared):
  - Position-independent storage
  - Stores all source text including trivia (whitespace, comments)
  - Deduplicates identical subtrees for memory efficiency
  - Cheap to clone (uses Arc internally)

- **Red Tree** (dynamic view):
  - Created on-demand for traversal
  - Provides parent/child/sibling navigation
  - Efficient ancestor queries via parent pointers

- **Lossless Property**: `parse(source).text() == source`
  - Enables perfect source reconstruction
  - Supports precise autofixes that preserve formatting
  - Better error recovery with full context

### Typed AST Layer

The typed AST layer (`cst/ast.rs`) provides high-level API over CST:

- Type-safe access to FSH constructs (Profile, ValueSet, Extension, etc.)
- Pattern matching over language elements
- Maintains connection to underlying CST for precise locations
- Used by semantic analyzer and rule implementations

### Semantic Model

The semantic model (`semantic.rs`) enriches the CST/AST with:

- FHIR resource metadata
- Symbol table for identifier resolution
- Reference tracking between resources
- Source map for efficient location lookups

### Parser Implementation

The parser uses Rowan for CST construction:

- Lossless representation of all source information
- Excellent error recovery (can continue after errors)
- Trivia preservation for formatting and comments
- Green/red tree pattern for memory efficiency
- Incremental reparsing support (future enhancement)

### Formatter Implementation

The formatter (`cst/formatter.rs`) leverages CST for lossless transformations:

- Preserves all trivia (comments, blank lines)
- Can selectively reformat specific nodes
- Maintains original formatting where not explicitly changed
- Supports various formatting options (indent style, line width, etc.)

**Token Optimization Pattern (Required):**

The formatter implements the Token optimization pattern proven by Ruff and Biome formatters for 2-3% performance improvement:

- `Token` variant for static, ASCII-only keywords and operators (fast path using bulk string operations)
- `Text` variant for dynamic content from source (slow path with Unicode support)
- **Always use `token("Profile")`** for FSH keywords: `Profile`, `ValueSet`, `Extension`, `Parent`, `Id`, etc.
- **Always use `token("*")` `token("..")`** for FSH operators and modifiers
- **Always use `text(&name, position)`** for identifiers and content from CST
- Pattern integrates seamlessly with Rowan CST (proven by Biome using same library)
- Expected: 70-85% of text operations use fast path, achieving 2-5% overall improvement
- See `TOKEN_OPTIMIZATION_ANALYSIS.md` for detailed analysis and `tasks/38_fsh_formatter.md` for implementation guide

### Canonical Package Management

The canonical package management system (`crates/maki-core/src/canonical/mod.rs`) integrates with `octofhir-canonical-manager` for FHIR package resolution:

**Key Features:**

- **SQLite-based storage**: Industry-standard database with WAL mode and B-tree indexes
- **Batch installation**: Installs multiple packages with single index rebuild (eliminates O(n²) problem)
- **Fast resolution**: ~7ms canonical URL lookups using single-query JOINs
- **Automatic package management**: Downloads and caches FHIR packages from registries

**Integration:**

- Uses `ensure_packages()` with batch installation API for better performance
- Tracks installed packages to avoid redundant installations
- Supports package priorities for resolution ordering
- Database location: `~/.maki/index/fhir.db`

**Dependencies:**

- Local path dependency: `../canonical-manager` (development)
- Published as: `octofhir-canonical-manager` on crates.io

Note: The workspace edition is set to "2024" (Rust Edition 2024).

## Testing Strategy

- **Unit tests** - In `src/` files with `#[cfg(test)]`
- **Integration tests** - In `crates/*/tests/` directories
- **Golden files** - Test fixtures in `crates/maki-core/tests/golden_files/`
- **Example files** - Real FSH examples in `examples/`

When adding rules, ensure:

1. Rule definition in `builtin/*.rs`
2. Tests in `crates/maki-rules/tests/`
3. Example FSH files demonstrating the rule

## Configuration

The linter supports configuration files:

- `.makirc.json` - JSON format
- `.makirc.toml` - TOML format

Configuration includes:

- Rule enable/disable
- Severity overrides
- File inclusion/exclusion patterns
- Custom rule directories

## Rust Development Resources

For writing idiomatic Rust code in this project:

- **Rust API Guidelines**: <https://rust-lang.github.io/api-guidelines/>
- **Rust Design Patterns**: <https://rust-unofficial.github.io/patterns/>
- **Effective Rust**: <https://www.lurklurk.org/effective-rust/>
- **Rust Performance Book**: <https://nnethercote.github.io/perf-book/>

When implementing new features, follow Rust idioms:

- Use `Result<T>` and `?` operator for error handling
- Prefer iterators over explicit loops
- Use `Arc<str>` for shared string data (already used in parser)
- Leverage the type system for compile-time guarantees
- Use `#[derive]` macros for common traits when possible
