---
title: Introduction
description: Learn about FSH Lint and what it can do for you
---

FSH Lint is a fast, powerful linter and formatter for FHIR Shorthand (FSH) files.

## What is FSH?

FHIR Shorthand (FSH) is a domain-specific language created to simplify the creation
of FHIR Implementation Guides (IGs). It allows you to define FHIR resources, profiles,
extensions, and value sets in a concise, human-readable format.

## Why FSH Lint?

Writing FSH by hand can be error-prone and inconsistent. FSH Lint helps you:

- **Catch errors early** - Find issues before SUSHI compilation
- **Maintain consistency** - Automatic code formatting and style enforcement
- **Follow best practices** - Learn FHIR/FSH patterns from built-in rules
- **Save time** - Auto-fix many issues and format code automatically
- **Customize validation** - Write custom rules for your organization

## Key Features

### Built in Rust 🦀

FSH Lint is written in Rust for maximum performance and reliability. It can lint
thousands of FSH files in seconds.

### Automatic Formatting

Lossless code formatter that maintains consistent style while preserving all comments and semantic content. Format on save, in CI, or on demand.

### Comprehensive Rules

- **Correctness**: Catch logical errors and invalid FSH
- **Style**: Maintain consistent formatting and naming
- **Documentation**: Ensure proper descriptions and metadata
- **Best Practices**: Learn recommended FHIR patterns

### Beautiful Error Messages

Get clear, actionable error messages with:
- Exact line and column positions
- Code frames showing context
- Colored diffs for suggested fixes
- Multiple severity levels (error, warning, info, hint)

### Extensible with GritQL

Write custom validation rules using GritQL, a powerful pattern-matching language.

### CI/CD Integration

First-class support for:
- GitHub Actions
- GitLab CI
- Jenkins
- Any CI/CD platform

## Next Steps

Ready to get started?

1. [Install FSH Lint](/getting-started/installation/)
2. [Run your first lint](/getting-started/quick-start/)
3. [Configure rules](/configuration/config-file/)
