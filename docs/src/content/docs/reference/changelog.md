---
title: Changelog
description: Release history and version changes
---

All notable changes to FSH Lint will be documented here.

## [Unreleased]

### Added
- GritQL-based custom rule support
- Comprehensive built-in rule set
- Auto-fix capabilities
- Multiple output formats (JSON, SARIF, GitHub Actions)
- Configuration file support with JSON Schema
- CLI with rich diagnostics
- CI/CD integration examples

## [0.1.0] - 2025-10-05

### Added
- Initial release
- Core linting engine with CST/AST parser
- Built-in rules:
  - Naming conventions
  - Metadata requirements
  - Profile validation
  - Cardinality checking
  - Required fields
  - Duplicate detection
  - Binding validation
- Command-line interface:
  - `lint` command with auto-fix
  - `format` command
  - `init` command
  - `rules` command
  - `check` command
- Configuration system:
  - JSON/JSONC support
  - Config inheritance
  - Rule customization
- Documentation site
- GitHub Actions workflow

### Known Issues
- GritQL integration in progress
- LSP server not yet implemented
- Some edge cases in parser

## Version History

### Versioning Scheme

FSH Lint follows [Semantic Versioning](https://semver.org/):
- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes (backward compatible)

### Release Cadence

- Major releases: As needed for breaking changes
- Minor releases: Monthly for new features
- Patch releases: As needed for bug fixes

## Upgrade Guide

### From 0.x to 0.1.0

Initial release - no migration needed.

## Future Roadmap

See [GitHub Milestones](https://github.com/octofhir/maki/milestones) for planned features.

### Planned for 0.2.0
- LSP server for real-time linting
- Enhanced GritQL integration
- Performance improvements
- Additional built-in rules

### Planned for 0.3.0
- VS Code extension
- Interactive fix suggestions
- Rule documentation generator
- Configuration wizard

## Contributing

See [Contributing Guide](/reference/contributing/) for how to propose changes.
