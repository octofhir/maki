# FSH Lint Development Commands
# Inspired by Biome's justfile structure

_default:
  just --list -u

alias t := test
alias f := format
alias l := lint
alias c := check
alias r := ready

# Install development tools
install-tools:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest taplo-cli

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

# Test CLI against example files
test-examples:
	cargo run --bin fsh-lint -- examples/*.fsh
	cargo run --bin fsh-lint -- check examples/*.fsh

# Test with different config files
test-configs:
	@echo "Testing with default config..."
	cargo run --bin fsh-lint -- --config examples/configs/default.fshlintrc examples/patient-profile.fsh
	@echo "\nTesting with strict config..."
	cargo run --bin fsh-lint -- --config examples/configs/strict.fshlintrc examples/patient-profile.fsh
	@echo "\nTesting with minimal config..."
	cargo run --bin fsh-lint -- --config examples/configs/minimal.fshlintrc examples/patient-profile.fsh

# Update golden file snapshots (for insta tests)
gen-snapshots:
	cargo insta test --review
	cargo insta accept

# Format code (Rust + TOML)
format:
	cargo fmt --all
	taplo format

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
	@echo "Running pre-commit checks..."
	git diff --exit-code --quiet || (echo "Warning: Uncommitted changes detected" && exit 0)
	just format
	just lint
	just test
	just test-doc
	@echo "\n✅ All checks passed! Ready to commit."

# Build release binary
build-release:
	cargo build --release --bin fsh-lint

# Run the CLI on examples with pretty output
demo:
	@echo "=== Demo: Linting patient profile ==="
	cargo run --bin fsh-lint -- examples/patient-profile.fsh
	@echo "\n=== Demo: Invalid cardinality ==="
	cargo run --bin fsh-lint -- examples/invalid-cardinality.fsh
	@echo "\n=== Demo: Missing metadata ==="
	cargo run --bin fsh-lint -- examples/missing-metadata.fsh

# Run benchmarks
bench:
	cargo bench

# Generate and view documentation
docs:
	cargo doc --no-deps --open

# Clean build artifacts
clean:
	cargo clean

# Check for outdated dependencies
outdated:
	cargo outdated

# Run security audit
audit:
	cargo audit

# Profile a test run
profile-test:
	cargo test --release -- --nocapture

# Watch for changes and run tests
watch:
	cargo watch -x test

# Create a new builtin rule (advanced, AST-based)
new-builtin-rule name category:
	@echo "Creating new builtin rule: {{name}} in category {{category}}"
	@echo "// TODO: Implement rule {{name}}" >> crates/fsh-lint-rules/src/builtin/{{snakecase(category)}}.rs
	just format

# Create a new GritQL rule template (user-extensible)
new-gritql-rule name:
	@echo "Creating new GritQL rule template: {{name}}"
	@echo "---" > crates/fsh-lint-rules/rules/{{snakecase(name)}}.grit
	@echo "rule_id: {{snakecase(name)}}" >> crates/fsh-lint-rules/rules/{{snakecase(name)}}.grit
	@echo "severity: warning" >> crates/fsh-lint-rules/rules/{{snakecase(name)}}.grit
	@echo "pattern: |" >> crates/fsh-lint-rules/rules/{{snakecase(name)}}.grit
	@echo "  // Add your GritQL pattern here" >> crates/fsh-lint-rules/rules/{{snakecase(name)}}.grit

# Run fuzzing (requires cargo-fuzz)
fuzz target='parse':
	cargo +nightly fuzz run {{target}}

# Show code coverage
coverage:
	cargo tarpaulin --out Html --output-dir target/coverage

# Validate all example configs
validate-configs:
	@echo "Validating configuration files..."
	cargo run --bin fsh-lint -- --validate-config examples/configs/default.fshlintrc
	cargo run --bin fsh-lint -- --validate-config examples/configs/strict.fshlintrc
	cargo run --bin fsh-lint -- --validate-config examples/configs/minimal.fshlintrc

# Generate autofix for all examples (dry-run)
autofix-dry-run:
	cargo run --bin fsh-lint -- fix --dry-run examples/*.fsh

# Apply safe autofixes to examples
autofix-safe:
	cargo run --bin fsh-lint -- fix --safe examples/*.fsh

# Show rule documentation
show-rule rule:
	cargo run --bin fsh-lint -- explain {{rule}}

# List all available rules
list-rules:
	cargo run --bin fsh-lint -- rules --list

# Lint examples and output JSON
lint-json:
	cargo run --bin fsh-lint -- --format json examples/*.fsh

# Lint examples and output SARIF
lint-sarif:
	cargo run --bin fsh-lint -- --format sarif examples/*.fsh
