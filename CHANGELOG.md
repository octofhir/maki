# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.6] - 2026-11-04

### Changed
- Keep project integrations enabled by default while allowing a minimal `maki-core`.
- Build `maki-formatter` without package downloads, SQLite, Tokio or file watchers.
- Remove unused direct dependencies from core, formatter, LSP and CLI.
- Replace deprecated YAML and terminal-width backends with `yaml_serde` and `terminal_size`.
- Use the maintained `tower-lsp-server` implementation behind the workspace alias.

### Fixed
- Limit automated release version changes to the inherited workspace version.

## [0.0.5] - 2026-08-18


## [0.0.4] - 2025-11-30


## [0.0.3] - 2025-11-07


## [0.0.2] - 2025-10-07


## [0.0.1] - 2025-10-06


## [0.0.4] - 2025-10-06


## [0.0.3] - 2025-10-06


## [0.0.2] - 2025-10-06


## [0.0.1] - 2025-10-06


### Added
- Linter with diagnostics system
- Built-in rules for naming, metadata, profiles, cardinality, and required fields
- GritQL-based rule engine for pattern matching
- CST-based rule engine with lossless parsing
- Autofix capabilities
- Multiple output formats (human, JSON, SARIF, GitHub Actions)
- CLI with lint, rules, and config commands

### Changed
- Stabilized parser implementation
- Improved test coverage

### Fixed
- Tests and linting issues
