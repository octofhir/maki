---
title: Introduction
description: Learn about MAKI and what it can do for you
---

MAKI is a high-performance FSH toolchain that serves as a drop-in replacement for SUSHI and GoFSH, with additional linting and formatting capabilities.

## What is FSH?

FHIR Shorthand (FSH) is a domain-specific language created to simplify the creation
of FHIR Implementation Guides (IGs). It allows you to define FHIR resources, profiles,
extensions, and value sets in a concise, human-readable format.

## Why MAKI?

MAKI provides a complete toolkit for FSH development:

- **Build FSH to FHIR** - SUSHI-compatible compilation with better performance
- **Convert FHIR to FSH** - GoFSH functionality with smart optimization
- **Catch errors early** - Find issues before compilation
- **Maintain consistency** - Automatic code formatting and style enforcement
- **Follow best practices** - Learn FHIR/FSH patterns from built-in rules
- **Save time** - Auto-fix many issues and format code automatically
- **Customize validation** - Write custom rules for your organization

## Key Features

### SUSHI-Compatible Build

Compile FSH files to FHIR resources with full compatibility:
- Profiles, Extensions, Logical Models, Resources
- ValueSets, CodeSystems
- Instances (Examples, Capabilities, etc.)
- ImplementationGuide generation
- Pre-build linting and formatting options

### GoFSH Converter

Convert existing FHIR resources back to FSH:
- Load JSON/XML FHIR resources
- Smart rule extraction and optimization
- Multiple file organization strategies
- Automatic config file generation

### Built in Rust

MAKI is written in Rust for maximum performance and reliability. It can process
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

1. [Install MAKI](/maki/getting-started/installation/)
2. [Quick Start Guide](/maki/getting-started/quick-start/)
3. [CLI Commands Reference](/maki/cli/commands/)
