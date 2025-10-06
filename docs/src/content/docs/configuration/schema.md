---
title: Configuration Schema Reference
description: Complete JSON schema reference for FSH Lint configuration
---

The FSH Lint configuration file has a JSON schema available for IDE autocomplete and validation.

## Schema URL

```json
{
  "$schema": "https://octofhir.github.io/fsh-lint-rs/schema/v1.json"
}
```

## Configuration Structure

### Root Configuration

```typescript
interface Configuration {
  $schema?: string;
  root?: boolean;
  extends?: string[];
  linter?: LinterConfiguration;
  formatter?: FormatterConfiguration;
  files?: FilesConfiguration;
}
```

### Linter Configuration

```typescript
interface LinterConfiguration {
  enabled: boolean;
  rules: RuleConfiguration;
  ruleDirectories?: string[];
}
```

### Rule Configuration

```typescript
interface RuleConfiguration {
  recommended?: boolean;
  style?: RuleSeverity | CategoryRules;
  documentation?: RuleSeverity | CategoryRules;
  correctness?: RuleSeverity | CategoryRules;
  suspicious?: RuleSeverity | CategoryRules;
}

type RuleSeverity = "off" | "hint" | "info" | "warn" | "error";
```

### Formatter Configuration

```typescript
interface FormatterConfiguration {
  enabled: boolean;
  indentSize?: number;
  lineWidth?: number;
  useTabs?: boolean;
}
```

### Files Configuration

```typescript
interface FilesConfiguration {
  include?: string[];
  exclude?: string[];
  ignoreFiles?: string[];
}
```

## Full Example

```jsonc
{
  "$schema": "https://octofhir.github.io/fsh-lint-rs/schema/v1.json",
  "root": true,
  "extends": ["./base-config.json"],
  
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true,
      "style": {
        "naming-convention": "error"
      },
      "documentation": "warn",
      "correctness": "error"
    },
    "ruleDirectories": ["./custom-rules"]
  },
  
  "formatter": {
    "enabled": true,
    "indentSize": 2,
    "lineWidth": 100,
    "useTabs": false
  },
  
  "files": {
    "include": ["**/*.fsh"],
    "exclude": [
      "**/node_modules/**",
      "**/build/**",
      "**/temp/**"
    ]
  }
}
```

## IDE Support

The schema enables:
- Autocomplete for configuration options
- Validation of configuration syntax
- Inline documentation
- Error detection

Supported in:
- VS Code
- JetBrains IDEs
- Any editor with JSON Schema support

## Downloading the Schema

The schema is automatically served from GitHub Pages. For offline use:

```bash
curl -O https://octofhir.github.io/fsh-lint-rs/schema/v1.json
```

Then reference it locally:

```json
{
  "$schema": "./fsh-lint-schema.json"
}
```
