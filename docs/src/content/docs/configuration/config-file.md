---
title: Configuration File
description: Learn how to configure FSH Lint
---

FSH Lint uses a `maki.json` or `maki.jsonc` configuration file.

## Creating a Config File

Run the init command to create a default configuration:

```bash
maki init
```

This creates `maki.json` in your current directory with recommended defaults.

## Config File Format

### Basic Configuration

```jsonc
{
  "$schema": "https://octofhir.github.io/maki-rs/schema/v1.json",
  "root": true,
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true
    }
  }
}
```

### Full Configuration

```jsonc
{
  "$schema": "https://octofhir.github.io/maki-rs/schema/v1.json",
  "root": true,

  // Extend from base config
  "extends": ["./configs/base.json"],

  "linter": {
    "enabled": true,
    "rules": {
      // Enable all recommended rules
      "recommended": true,

      // Configure specific rules
      "correctness": {
        "duplicate-definition": "error",
        "required-fields": "warn"
      },
      "style": {
        "naming-convention": "warn"
      }
    },

    // Load custom GritQL rules
    "ruleDirectories": ["./custom-rules"]
  },

  "formatter": {
    "enabled": true,
    "indentSize": 2,
    "lineWidth": 100
  },

  "files": {
    "include": ["**/*.fsh"],
    "exclude": ["**/node_modules/**", "**/temp/**"]
  }
}
```

## Configuration Options

### Schema Reference

The `$schema` field enables IDE autocomplete and validation:

```jsonc
{
  "$schema": "https://octofhir.github.io/maki-rs/schema/v1.json"
}
```

### Root Flag

Set `root: true` to stop upward config file search:

```jsonc
{
  "root": true
}
```

### Extends

Inherit from other config files:

```jsonc
{
  "extends": ["./base.json", "@my-org/fsh-config"]
}
```

## Auto-Discovery

FSH Lint automatically searches for config files by:

1. Starting from current directory
2. Looking for `maki.jsonc` or `maki.json`
3. Moving up to parent directories
4. Stopping at `root: true` or filesystem root

## JSONC Support

Use comments and trailing commas in `.jsonc` files:

```jsonc
{
  // This is a comment
  "linter": {
    "enabled": true,  // Trailing comma OK
  }
}
```

## See Also

- [Rule Configuration](/configuration/rules/)
- [GritQL Rules](/configuration/gritql/)
- [Schema Reference](/configuration/schema/)
