---
title: Quick Start
description: Get started with FSH Lint in 5 minutes
---

Get up and running with FSH Lint in just a few minutes.

## Prerequisites

- Rust 1.80 or later (if building from source)
- OR use pre-built binaries

## Step 1: Install FSH Lint

```bash
cargo install maki
```

Verify the installation:

```bash
maki --version
```

## Step 2: Initialize Configuration

Create a default configuration file in your FSH project:

```bash
cd your-fsh-project
maki init
```

This creates `maki.json` with recommended settings:

```json
{
  "$schema": "https://octofhir.github.io/maki/schema/v1.json",
  "root": true,
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true
    }
  }
}
```

## Step 3: Lint Your Files

Lint all FSH files in the current directory:

```bash
maki lint **/*.fsh
```

Or lint specific files:

```bash
maki lint input/fsh/profiles.fsh
```

## Step 4: Review Diagnostics

FSH Lint will show you any issues found:

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
maki format **/*.fsh
```

The formatter will:
- Normalize spacing around `:` and `=`
- Align rules for better readability
- Preserve all comments and blank lines
- Maintain consistent indentation

You can also check formatting without modifying files:

```bash
maki format --check **/*.fsh
```

## Step 6: Auto-fix Issues

Many issues can be fixed automatically:

```bash
maki lint --fix **/*.fsh
```

FSH Lint will apply safe fixes and report what was changed.

## Step 7: Customize Rules

Edit `maki.json` to customize rule behavior:

```jsonc
{
  "linter": {
    "rules": {
      "recommended": true,
      "style": {
        "naming-convention": "error"  // Upgrade to error
      },
      "documentation": {
        "title-required": "off"  // Disable this rule
      }
    }
  }
}
```

## Next Steps

- [Learn about the formatter](/maki/guides/formatter/) for automatic code formatting
- [Configure rules](/maki/configuration/rules/) to match your project's needs
- [Learn about built-in rules](/maki/rules/) to understand what FSH Lint checks
- [Write custom rules](/maki/guides/custom-rules/) for project-specific validation
- [Integrate with CI/CD](/maki/guides/ci-cd/) to automate linting and formatting

## Common Commands

```bash
# Lint with automatic fixes
maki lint --fix **/*.fsh

# Lint and show only errors
maki lint --severity error **/*.fsh

# List all available rules
maki rules

# Get detailed information about a specific rule
maki rules --detailed style/naming-convention

# Format FSH files
maki format **/*.fsh

# Check formatting (useful for CI)
maki format --check **/*.fsh
```

## Help and Support

- Run `maki --help` for command-line help
- Run `maki <command> --help` for command-specific help
- Check the [Troubleshooting Guide](/guides/troubleshooting/) for common issues
