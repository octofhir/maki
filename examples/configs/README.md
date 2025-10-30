# Configuration Examples

This directory contains example configuration files for maki-rs.

## Files

### minimal.json
A minimal configuration file with just the essential settings. Good starting point for new projects.

### base.json
A base configuration with common settings. Can be extended by other configs.

### full.jsonc
A comprehensive configuration showing all available options. Uses JSONC format (JSON with comments).

## Usage

### Using a configuration file

Place a `maki.json` or `maki.jsonc` file in your project root:

```bash
# Copy a template
cp examples/configs/minimal.json ./maki.json

# Or create your own
cat > maki.json << 'EOF'
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
EOF
```

### Extending a base configuration

You can extend other configurations using the `extends` field:

```jsonc
{
  "extends": ["./base.json"],
  "linter": {
    "rules": {
      "correctness": {
        "duplicate-definition": "error"
      }
    }
  }
}
```

### Configuration discovery

maki will automatically discover configuration files by searching upward from the current directory:

1. Starts from the current directory
2. Looks for `maki.jsonc` or `maki.json`
3. If not found, moves up to the parent directory
4. Stops when a config with `"root": true` is found or reaches filesystem root

### Schema validation

Modern editors with JSON Schema support will provide:
- Auto-completion
- Validation
- Documentation tooltips

Just include the `$schema` field in your config:

```json
{
  "$schema": "https://octofhir.github.io/maki-rs/schema/v1.json"
}
```

## Rule Severities

Rules can be configured with the following severities:

- `"off"` - Disable the rule
- `"info"` - Informational message (doesn't fail build)
- `"warn"` - Warning (doesn't fail build)
- `"error"` - Error (fails build)

## Rule Categories

Rules are organized into categories:

- **blocking** - Critical requirements that must pass before other rules run
- **correctness** - Errors in FSH logic (semantic errors)
- **suspicious** - Patterns that often indicate bugs
- **style** - Formatting and naming conventions
- **documentation** - Metadata and guidance requirements

## Custom Rules

You can load custom GritQL-based rules from directories:

```jsonc
{
  "linter": {
    "ruleDirectories": [
      "./custom-rules",
      "./node_modules/@my-org/fsh-rules/rules"
    ]
  }
}
```

Place `.grit` files in these directories to define custom linting rules.
