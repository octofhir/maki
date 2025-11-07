---
title: CLI Commands
description: FSH Lint command-line interface reference
---

Complete reference for all FSH Lint commands.

## `maki lint`

Lint FSH files and report diagnostics.

```bash
maki lint [OPTIONS] <FILES>...
```

### Options

#### Autofix Options

- `--fix` - Automatically fix safe issues (no semantic changes)
- `--unsafe` - Apply unsafe fixes (semantic changes) in addition to safe fixes
- `--dry-run` - Preview fixes without modifying files
- `--interactive`, `-i` - Prompt for confirmation on each unsafe fix
- `-w, --write` - Write fixes to files (alias for `--fix`)

#### Diagnostic Options

- `--severity <LEVEL>` - Only show diagnostics at or above this level: `error`, `warning`, `info`, `hint`
- `--format <FORMAT>` - Output format: `human`, `json`, `sarif`, `github`
- `--max-diagnostics <N>` - Limit number of diagnostics shown

#### Configuration Options

- `--config <PATH>` - Path to configuration file
- `--no-config` - Don't load configuration files

### Examples

```bash
# Lint all FSH files
maki lint **/*.fsh

# Apply safe fixes only
maki lint --fix input/fsh/*.fsh

# Apply all fixes (safe + unsafe)
maki lint --fix --unsafe input/fsh/*.fsh

# Preview fixes without applying
maki lint --fix --dry-run input/fsh/*.fsh

# Interactive mode - review each unsafe fix
maki lint --fix --unsafe --interactive input/fsh/*.fsh

# Show only errors
maki lint --severity error **/*.fsh

# Output JSON format
maki lint --format json **/*.fsh > diagnostics.json
```

### Fix Safety Levels

**Safe fixes** (applied with `--fix`):
- Add missing metadata (`Id`, `Title`, `Description`)
- Remove unused code (redundant aliases)
- Fix formatting and whitespace
- No semantic changes

**Unsafe fixes** (require `--unsafe` flag):
- Change naming conventions
- Add FHIR constraints
- Modify cardinality
- Semantic changes that should be reviewed

See the [Automatic Fixes guide](/guides/autofix/) for detailed information.

## `maki format`

Format FSH files.

```bash
maki format [OPTIONS] <FILES>...
```

### Options

- `--check` - Check if files are formatted (don't modify)
- `--diff` - Show formatting differences
- `--config <PATH>` - Path to configuration file

### Examples

```bash
# Format all FSH files
maki format **/*.fsh

# Check formatting without modifying
maki format --check **/*.fsh
```

## `maki init`

Initialize configuration file.

```bash
maki init [OPTIONS]
```

### Options

- `--full` - Generate full example configuration
- `--output <PATH>` - Output path (default: `maki.json`)

### Examples

```bash
# Create default config
maki init

# Create full example
maki init --full
```

## `maki rules`

List available rules.

```bash
maki rules [OPTIONS]
```

### Options

- `--detailed` - Show detailed information
- `--category <CATEGORY>` - Filter by category
- `--search <QUERY>` - Search rules

### Examples

```bash
# List all rules
maki rules

# Show detailed info for a category
maki rules --detailed --category style

# Search for specific rules
maki rules --search naming
```

## `maki check`

Check configuration validity.

```bash
maki check [OPTIONS]
```

### Options

- `--config <PATH>` - Path to configuration file

### Examples

```bash
# Check default config
maki check

# Check specific config
maki check --config custom-config.json
```

## Global Options

Available for all commands:

- `-h, --help` - Print help information
- `-V, --version` - Print version information
- `-v, --verbose` - Enable verbose output
- `--color <WHEN>` - Colorize output: `auto`, `always`, `never`

## Exit Codes

See [Exit Codes](/cli/exit-codes/) for details.
