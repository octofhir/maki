---
title: Rule Configuration
description: Configure FSH Lint rules for your project
---

Learn how to configure FSH Lint rules to match your project's requirements.

## Configuration File

Rules are configured in `maki.json` (or `.jsonc`) in the `linter.rules` section:

```jsonc
{
  "linter": {
    "enabled": true,
    "rules": {
      // Enable all recommended rules
      "recommended": true,

      // Configure specific categories
      "style": {
        "naming-convention": "error"
      },
      "documentation": {
        "title-required": "warn",
        "description-required": "warn"
      }
    }
  }
}
```

## Rule Severity Levels

Each rule can be set to one of these severity levels:

- **`"off"`** - Disable the rule completely
- **`"hint"`** - Show as a hint (lowest severity)
- **`"info"`** - Informational message
- **`"warn"`** - Warning (doesn't fail builds)
- **`"error"`** - Error (fails builds)

Example:

```jsonc
{
  "linter": {
    "rules": {
      "style/naming-convention": "error",
      "documentation/title-required": "warn",
      "suspicious/unused-alias": "info",
      "style/prefer-short-syntax": "hint",
      "style/trailing-whitespace": "off"
    }
  }
}
```

## Rule Categories

### Recommended Rules

Enable all recommended rules with:

```json
{
  "linter": {
    "rules": {
      "recommended": true
    }
  }
}
```

This enables a curated set of rules considered best practices.

### Style Rules

Control code style and formatting:

```jsonc
{
  "linter": {
    "rules": {
      "style": {
        "naming-convention": "error",
        "prefer-short-syntax": "warn",
        "trailing-whitespace": "error"
      }
    }
  }
}
```

### Documentation Rules

Ensure proper documentation:

```jsonc
{
  "linter": {
    "rules": {
      "documentation": {
        "title-required": "warn",
        "description-required": "warn",
        "metadata-completeness": "info"
      }
    }
  }
}
```

### Correctness Rules

Catch errors and invalid FSH:

```jsonc
{
  "linter": {
    "rules": {
      "correctness": {
        "duplicate-definition": "error",
        "invalid-cardinality": "error",
        "profile-parent-required": "error"
      }
    }
  }
}
```

### Suspicious Rules

Detect suspicious patterns:

```jsonc
{
  "linter": {
    "rules": {
      "suspicious": {
        "unused-alias": "warn",
        "redundant-cardinality": "info",
        "weak-binding": "info"
      }
    }
  }
}
```

## Rule-Specific Options

Some rules accept additional configuration:

```jsonc
{
  "linter": {
    "rules": {
      "style/naming-convention": {
        "severity": "error",
        "options": {
          "profileSuffix": "Profile",
          "valueSetSuffix": "VS",
          "codeSystemSuffix": "CS"
        }
      }
    }
  }
}
```

## Overriding Inherited Rules

Rules from extended configurations can be overridden:

```jsonc
{
  "extends": ["./base-config.json"],
  "linter": {
    "rules": {
      // Override rule from base config
      "style/naming-convention": "warn"  // Was "error" in base
    }
  }
}
```

## Disabling Rules Inline

Disable rules for specific code sections using comments:

```fsh
// maki-disable-next-line style/naming-convention
Profile: patient_profile
Parent: Patient

// maki-disable style/naming-convention
Profile: observation_profile
Profile: condition_profile
// maki-enable style/naming-convention
```

## Per-File Configuration

Use `.makirc.json` in subdirectories for file-specific rules:

```
project/
├── maki.json          # Root config
├── profiles/
│   ├── .makirc.json    # Profile-specific rules
│   └── *.fsh
└── valuesets/
    ├── .makirc.json    # ValueSet-specific rules
    └── *.fsh
```

## Example Configurations

### Strict Configuration

```jsonc
{
  "linter": {
    "rules": {
      "recommended": true,
      "style": "error",          // All style rules as errors
      "documentation": "error",   // All documentation rules as errors
      "correctness": "error"
    }
  }
}
```

### Lenient Configuration

```jsonc
{
  "linter": {
    "rules": {
      "correctness": "error",     // Only correctness rules as errors
      "style": "warn",
      "documentation": "info",
      "suspicious": "info"
    }
  }
}
```

### Migration Configuration

```jsonc
{
  "linter": {
    "rules": {
      "recommended": true,
      // Temporarily disable during migration
      "style/naming-convention": "off",
      "documentation/title-required": "warn"  // Downgrade from error
    }
  }
}
```

## See Also

- [Built-in Rules](/rules/) - Complete list of rules
- [GritQL Rules](/configuration/gritql/) - Write custom rules
- [Schema Reference](/configuration/schema/) - Full configuration schema
