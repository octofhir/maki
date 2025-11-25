---
title: Quick Start
description: Get started with MAKI in 5 minutes
---

Get up and running with MAKI in just a few minutes.

## Prerequisites

- Rust 1.80 or later (if building from source)
- OR use pre-built binaries

## Step 1: Install MAKI

```bash
cargo install maki
```

Verify the installation:

```bash
maki --version
```

## Step 2: Initialize a New Project

Create a new FHIR Implementation Guide project:

```bash
maki init MyIG
cd MyIG
```

Or initialize configuration in an existing project:

```bash
cd your-fsh-project
maki config init
```

## Step 3: Build Your IG

Build FSH files to FHIR resources (SUSHI-compatible):

```bash
maki build --progress
```

This compiles FSH from `input/fsh/` to FHIR JSON in `fsh-generated/`.

## Step 4: Lint Your Files

Check your FSH files for issues:

```bash
maki lint input/fsh/
```

MAKI will show you any issues found:

```
error[correctness/duplicate-definition]: Duplicate profile definition

  > 15 │ Profile: PatientProfile
       │          ^^^^^^^^^^^^^^ Profile 'PatientProfile' is already defined
    16 │ * name 1..1 MS

  i First defined at line 8
```

## Step 5: Format Your Code

Format your FSH files for consistent style:

```bash
maki fmt input/fsh/
```

The formatter will:
- Normalize spacing around `:` and `=`
- Align rules for better readability
- Preserve all comments and blank lines
- Maintain consistent indentation

You can also check formatting without modifying files:

```bash
maki fmt --check input/fsh/
```

## Step 6: Auto-fix Issues

Many issues can be fixed automatically:

```bash
maki lint --write input/fsh/
```

MAKI will apply safe fixes and report what was changed.

## Step 7: Convert FHIR to FSH (GoFSH)

Convert existing FHIR resources back to FSH:

```bash
maki gofsh ./fsh-generated -o ./converted-fsh --progress
```

## Common Workflows

### Build with Quality Checks

Run linter and formatter before building:

```bash
maki build --lint --format --progress
```

### Strict Mode Build

Treat warnings as errors (useful for CI):

```bash
maki build --lint --strict
```

### Clean Rebuild

Clean output and rebuild from scratch:

```bash
maki build --clean --progress
```

## Next Steps

- [CLI Commands Reference](/maki/cli/commands/) for all available commands
- [Learn about the formatter](/maki/guides/formatter/) for automatic code formatting
- [Configure rules](/maki/configuration/rules/) to match your project's needs
- [Learn about built-in rules](/maki/rules/) to understand what MAKI checks
- [Write custom rules](/maki/guides/custom-rules/) for project-specific validation
- [Integrate with CI/CD](/maki/guides/ci-cd/) to automate linting and formatting

## Common Commands

```bash
# Build FSH to FHIR
maki build --progress

# Lint with automatic fixes
maki lint --write input/fsh/

# Format FSH files
maki fmt input/fsh/

# Check formatting (useful for CI)
maki fmt --check input/fsh/

# Convert FHIR to FSH
maki gofsh ./fsh-generated -o ./output

# List all available rules
maki rules

# Get help
maki --help
maki build --help
```

## Help and Support

- Run `maki --help` for command-line help
- Run `maki <command> --help` for command-specific help
- Check the [Troubleshooting Guide](/maki/guides/troubleshooting/) for common issues
