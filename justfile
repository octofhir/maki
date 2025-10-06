# FSH Lint Development Commands
#
# Quick reference:
#   just test          - Run all tests
#   just format        - Format code
#   just lint          - Run clippy
#   just check         - Run format + lint + test
#   just ready         - Pre-commit checks
#   just ci            - Full CI checks
#   just demo          - Demo the CLI
#
# See `just --list` for all available commands

_default:
  just --list -u

alias t := test
alias f := format
alias l := lint
alias c := check
alias r := ready

# ============================================================================
# Setup & Installation
# ============================================================================

# Install development tools
install-tools:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest taplo-cli

# ============================================================================
# Testing
# ============================================================================

# Run all tests
test:
	cargo nextest run --no-fail-fast || cargo test --no-fail-fast

# Run tests for a specific crate (e.g., just test-crate fsh-lint-core)
test-crate name:
	cargo nextest run -p {{name}} --no-fail-fast || cargo test -p {{name}} --no-fail-fast

# Test a specific rule by name (converts to snake_case automatically)
test-rule name:
	cargo test -p fsh-lint-rules -- {{snakecase(name)}} --show-output

# Run quick smoke tests
test-quick:
	cargo test --lib --no-fail-fast

# Run doc tests
test-doc:
	cargo test --doc

# Run integration tests
test-integration:
	cargo test --package fsh-lint-integration-tests

# Run all GritQL tests
test-gritql:
	cargo test --package fsh-lint-rules --test gritql_integration_test
	cargo test --package fsh-lint-rules --test gritql_full_integration_test
	cargo test --package fsh-lint-rules --test gritql_real_file_test

# Test CLI against example files
test-examples:
	cargo run --bin fsh-lint -- lint examples/*.fsh

# Test with different config files
test-configs:
	@echo "Testing with base config..."
	cargo run --bin fsh-lint -- lint --config examples/configs/base.json examples/
	@echo "\nTesting with full config..."
	cargo run --bin fsh-lint -- lint --config examples/configs/full.jsonc examples/
	@echo "\nTesting with minimal config..."
	cargo run --bin fsh-lint -- lint --config examples/configs/minimal.json examples/

# ============================================================================
# Snapshot Testing (insta)
# ============================================================================

# Update golden file snapshots (for insta tests)
gen-snapshots:
	cargo insta test --review
	cargo insta accept

# Run tests and create new snapshots
insta-test:
	cargo insta test

# Review pending snapshots interactively
insta-review:
	cargo insta review

# Accept all pending snapshots
insta-accept:
	cargo insta accept

# Reject all pending snapshots
insta-reject:
	cargo insta reject

# Update snapshots (test + accept all) - use with caution!
insta-update:
	cargo insta test --accept

# ============================================================================
# Code Quality
# ============================================================================

# Format code (Rust + TOML)
format:
	cargo fmt --all
	-taplo format 2>/dev/null || true

# Lint with clippy
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Run all checks (format, lint, test)
check:
	just format
	just lint
	just test

# Prepare for commit - ensures everything passes
ready:
	@echo "🔨 Running pre-commit checks..."
	@echo ""
	@echo "📝 Formatting..."
	just format
	@echo ""
	@echo "🔍 Linting..."
	just lint
	@echo ""
	@echo "🧪 Testing..."
	just test
	@echo ""
	@echo "📚 Doc tests..."
	just test-doc
	@echo ""
	@echo "✅ All checks passed! Ready to commit."

# Run all CI checks (comprehensive)
ci:
	@echo "🚀 Running CI checks..."
	@echo ""
	@echo "📋 Checking formatting..."
	cargo fmt --all -- --check
	@echo ""
	@echo "🔍 Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings
	@echo ""
	@echo "🏗️  Building all crates..."
	cargo build --workspace --all-features
	@echo ""
	@echo "🧪 Running all tests..."
	cargo test --workspace --all-features
	@echo ""
	@echo "📚 Running doc tests..."
	cargo test --doc
	@echo ""
	@echo "✅ All CI checks passed!"

# ============================================================================
# Building
# ============================================================================

# Build release binary
build-release:
	cargo build --release --bin fsh-lint

# ============================================================================
# Demo & Documentation
# ============================================================================

# Run the CLI on examples with pretty output
demo:
	@echo "=== Demo: Linting examples directory ==="
	cargo run --bin fsh-lint -- lint examples/
	@echo "\n=== Demo: List rules ==="
	cargo run --bin fsh-lint -- rules --detailed
	@echo "\n=== Demo: Format check ==="
	cargo run --bin fsh-lint -- fmt --check examples/

# Generate and view documentation
docs:
	cargo doc --no-deps --open

# Generate documentation without opening
docs-no-open:
	cargo doc --no-deps

# ============================================================================
# Benchmarking
# ============================================================================

# Run benchmarks
bench:
	cargo bench --package fsh-lint-bench

# Run specific benchmark
bench-one name:
	cargo bench --package fsh-lint-bench --bench {{name}}

# ============================================================================
# Dependency Management
# ============================================================================

# Check for outdated dependencies
outdated:
	cargo outdated

# Run security audit
audit:
	cargo audit

# ============================================================================
# Profiling & Debugging
# ============================================================================

# Profile a test run
profile-test:
	cargo test --release -- --nocapture

# Show code coverage
coverage:
	cargo tarpaulin --out Html --output-dir target/coverage

# Run fuzzing (requires cargo-fuzz and nightly toolchain)
fuzz target='parse':
	cargo +nightly fuzz run {{target}}

# ============================================================================
# File Watching
# ============================================================================

# Watch for changes and run tests
watch:
	cargo watch -x test

# Watch for changes and run specific crate tests
watch-crate name:
	cargo watch -x "test -p {{name}}"

# Watch for changes and run checks
watch-check:
	cargo watch -x check

# Watch for changes and run clippy
watch-clippy:
	cargo watch -x "clippy --all-targets"

# ============================================================================
# CLI Command Testing
# ============================================================================

# Validate all example configs
validate-configs:
	@echo "Validating configuration files..."
	cargo run --bin fsh-lint -- config validate examples/configs/base.json
	cargo run --bin fsh-lint -- config validate examples/configs/full.jsonc
	cargo run --bin fsh-lint -- config validate examples/configs/minimal.json

# Lint examples with dry-run (show fixes without applying)
lint-dry-run:
	cargo run --bin fsh-lint -- lint --dry-run examples/

# Apply safe autofixes to examples
lint-write:
	cargo run --bin fsh-lint -- lint --write examples/

# Show rule documentation
show-rule rule:
	cargo run --bin fsh-lint -- rules explain {{rule}}

# List all available rules
list-rules:
	cargo run --bin fsh-lint -- rules

# Search rules by query
search-rules query:
	cargo run --bin fsh-lint -- rules search {{query}}

# Lint examples and output JSON
lint-json:
	cargo run --bin fsh-lint -- lint --format json examples/

# Lint examples and output SARIF
lint-sarif:
	cargo run --bin fsh-lint -- lint --format sarif examples/

# Lint examples and output compact format
lint-compact:
	cargo run --bin fsh-lint -- lint --format compact examples/

# Lint examples in GitHub Actions format
lint-github:
	cargo run --bin fsh-lint -- lint --format github examples/

# Format examples (dry-run by default)
fmt-examples:
	cargo run --bin fsh-lint -- fmt examples/

# Format examples and write changes
fmt-write:
	cargo run --bin fsh-lint -- fmt --write examples/

# Format examples and show diff
fmt-diff:
	cargo run --bin fsh-lint -- fmt --diff examples/

# Check if examples are formatted correctly
fmt-check:
	cargo run --bin fsh-lint -- fmt --check examples/

# Initialize a new config file (JSON)
config-init:
	cargo run --bin fsh-lint -- config init

# Initialize a new config file with examples
config-init-examples:
	cargo run --bin fsh-lint -- config init --with-examples

# Initialize a new config file (TOML)
config-init-toml:
	cargo run --bin fsh-lint -- config init --format toml

# Validate current config file
config-validate:
	cargo run --bin fsh-lint -- config validate

# Show current configuration
config-show:
	cargo run --bin fsh-lint -- config show

# Show resolved configuration
config-show-resolved:
	cargo run --bin fsh-lint -- config show --resolved

# Generate shell completions (bash)
gen-completions shell="bash":
	cargo run --bin fsh-lint -- --generate-completion {{shell}}

# Show version information
version:
	cargo run --bin fsh-lint -- version

# Show detailed version information
version-detailed:
	cargo run --bin fsh-lint -- version --detailed

# ============================================================================
# Real-world Testing Commands
# ============================================================================

# Lint mcode-ig example project
lint-mcode:
	cargo run --bin fsh-lint -- lint examples/mcode-ig/

# Lint mcode-ig with verbose output
lint-mcode-verbose:
	cargo run --bin fsh-lint -- lint -vv examples/mcode-ig/

# Lint mcode-ig and show statistics
lint-mcode-stats:
	cargo run --bin fsh-lint -- lint --progress examples/mcode-ig/

# Format mcode-ig example project (dry-run)
fmt-mcode:
	cargo run --bin fsh-lint -- fmt examples/mcode-ig/

# Test lint on all example directories
test-all-examples:
	@echo "Testing examples/..."
	cargo run --bin fsh-lint -- lint examples/*.fsh
	@echo "\nTesting examples/gritql/..."
	cargo run --bin fsh-lint -- lint examples/gritql/
	@echo "\nTesting examples/mcode-ig/..."
	cargo run --bin fsh-lint -- lint examples/mcode-ig/

# ============================================================================
# Development Utilities
# ============================================================================

# Run cargo check on all crates
check-all:
	cargo check --workspace --all-features --all-targets

# Build all crates in workspace
build-all:
	cargo build --workspace --all-features

# Build and install the CLI locally
install:
	cargo install --path crates/fsh-lint-cli --force

# Uninstall the CLI
uninstall:
	cargo uninstall fsh-lint

# Show workspace dependency tree
tree:
	cargo tree --workspace

# Show workspace dependency tree (duplicates only)
tree-duplicates:
	cargo tree --workspace --duplicates

# Update all dependencies
update:
	cargo update

# Check for security vulnerabilities
security-audit:
	cargo audit

# ============================================================================
# Performance & Profiling
# ============================================================================

# Run cargo bloat to analyze binary size
bloat:
	cargo bloat --release --bin fsh-lint

# Run cargo bloat for specific crate
bloat-crate name:
	cargo bloat --release --package {{name}}

# Profile parsing performance
profile-parse:
	cargo build --release
	@echo "Profiling parser on examples/mcode-ig..."
	hyperfine './target/release/fsh-lint lint examples/mcode-ig/ --format compact'

# Profile formatting performance
profile-fmt:
	cargo build --release
	@echo "Profiling formatter on examples/mcode-ig..."
	hyperfine './target/release/fsh-lint fmt --check examples/mcode-ig/'

# ============================================================================
# Cleanup & Maintenance
# ============================================================================

# Clean build artifacts
clean:
	cargo clean

# Clean all build artifacts and caches
clean-all:
	cargo clean
	rm -rf target/
	rm -rf .cargo-cache/
	find . -name "*.snap.new" -delete

# Remove all test snapshots (use with caution!)
clean-snapshots:
	find . -name "*.snap" -delete
	find . -name "*.snap.new" -delete

# ============================================================================
# Code Generation
# ============================================================================

# Create a new builtin rule stub (CST/AST-based)
new-builtin-rule name category:
	@echo "Creating new builtin rule: {{name}} in category {{category}}"
	@echo "// TODO: Implement rule {{name}}" >> crates/fsh-lint-rules/src/builtin/{{snakecase(category)}}.rs
	just format
