# MAKI Configuration Examples

This directory contains example configuration files for MAKI, showcasing different configuration scenarios.

## Configuration Files

### `full-example.json` / `full-example.yaml`

Comprehensive configuration file demonstrating **all available options** in MAKI. This example includes:

- **JSON Schema reference** - For IDE autocomplete and validation
- **Root marker** - Stops configuration file discovery at this directory
- **Top-level dependencies** - Shared FHIR packages across build and linter
- **Build configuration** - SUSHI-compatible IG metadata
- **Linter configuration** - All rule categories with custom severity levels
- **Formatter configuration** - Code formatting preferences
- **Files configuration** - Include/exclude patterns for FSH files

Use this as a reference when you need to:
- Understand all available configuration options
- Set up complex linting rules
- Configure custom rule directories
- Fine-tune file discovery patterns

## Configuration Sections

### 1. Schema and Root

```json
{
  "$schema": "https://octofhir.github.io/maki/schema/v1.json",
  "root": true
}
```

- `$schema` - Enables IDE autocomplete and validation
- `root` - Stops upward config file search (useful for monorepos)

### 2. Dependencies

```json
{
  "dependencies": {
    "hl7.fhir.us.core": "6.1.0",
    "hl7.terminology.r4": "5.3.0"
  }
}
```

Top-level dependencies are shared between build and linter, avoiding duplication.

### 3. Build Configuration (SUSHI-compatible)

```json
{
  "build": {
    "canonical": "http://example.org/fhir/my-ig",
    "fhirVersion": ["4.0.1"],
    "id": "my.example.ig",
    "name": "MyImplementationGuide",
    "title": "My Example Implementation Guide",
    "version": "1.0.0",
    "status": "draft",
    "publisher": {
      "name": "Example Organization",
      "url": "http://example.org"
    }
  }
}
```

Contains all fields from `sushi-config.yaml`, allowing MAKI to act as a drop-in replacement for SUSHI.

### 4. Linter Configuration

```json
{
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true,
      "blocking": {
        "validate-critical-requirements": "error"
      },
      "correctness": {
        "duplicate-definition": "error",
        "invalid-reference": "error"
      },
      "suspicious": {
        "unused-alias": "warn"
      },
      "style": {
        "naming-convention": "warn"
      },
      "documentation": {
        "require-description": "warn"
      }
    },
    "ruleDirectories": ["custom-rules/"]
  }
}
```

#### Rule Categories

- **blocking** - Critical requirements that must pass before other rules run
- **correctness** - Syntax and semantic errors
- **suspicious** - Patterns that often indicate bugs
- **style** - Naming conventions and formatting
- **documentation** - Metadata requirements

#### Severity Levels

- `off` - Disable the rule
- `info` - Informational message
- `warn` - Warning (doesn't fail build)
- `error` - Error (fails build)

### 5. Formatter Configuration

```json
{
  "formatter": {
    "enabled": true,
    "indentSize": 2,
    "lineWidth": 100,
    "alignCarets": true
  }
}
```

Controls automatic code formatting with options for:
- Indentation (spaces)
- Maximum line width
- Caret alignment for readability

### 6. Files Configuration

```json
{
  "files": {
    "include": [
      "input/fsh/**/*.fsh",
      "fsh/**/*.fsh"
    ],
    "exclude": [
      "**/node_modules/**",
      "**/temp/**",
      "**/*.generated.fsh"
    ],
    "ignoreFiles": [
      ".fshlintignore",
      ".gitignore"
    ]
  }
}
```

Specifies which FSH files to process using glob patterns.

## Generating Configuration Files

You can generate these example files using the `maki-devtools` command:

```bash
# Generate full example in JSON
cargo run --package maki-devtools -- generate-config --full --output maki.json

# Generate full example in YAML
cargo run --package maki-devtools -- generate-config --full --output maki.yaml

# Generate minimal default configuration
cargo run --package maki-devtools -- generate-config --output maki.json
```

## Configuration Formats

MAKI supports multiple configuration formats:

- **JSON** (`.json`) - Standard JSON format
- **YAML** (`.yaml`, `.yml`) - Human-friendly YAML format

All formats are functionally equivalent; choose the one that best fits your workflow.

## Configuration Discovery

MAKI searches for configuration files in this order:

1. `maki.yaml` or `maki.yml`
2. `maki.json`
3. `.makirc.json`
4. `.makirc` (JSON)

The search starts in the current directory and walks up the directory tree until:
- A configuration file is found
- A configuration with `"root": true` is found
- The root of the filesystem is reached

## Additional Resources

- [MAKI Documentation](../../docs)
- [SUSHI Configuration Reference](https://fshschool.org/docs/sushi/configuration/)
- [FHIR IG Specification](http://hl7.org/fhir/R4/implementationguide.html)
