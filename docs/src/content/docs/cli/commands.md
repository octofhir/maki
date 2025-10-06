---
title: CLI Commands
description: FSH Lint command-line interface reference
---

Complete reference for all FSH Lint commands.

## `fsh-lint lint`

Lint FSH files and report diagnostics.

```bash
fsh-lint lint [OPTIONS] <FILES>...
```

### Options

- `--fix` - Automatically fix issues when possible
- `--severity <LEVEL>` - Only show diagnostics at or above this level
- `--format <FORMAT>` - Output format: `human`, `json`, `sarif`, `github`
- `--config <PATH>` - Path to configuration file
- `--no-config` - Don't load configuration files
- `--max-diagnostics <N>` - Limit number of diagnostics shown

### Examples

```bash
# Lint all FSH files
fsh-lint lint **/*.fsh

# Lint with automatic fixes
fsh-lint lint --fix input/fsh/*.fsh

# Show only errors
fsh-lint lint --severity error **/*.fsh

# Output JSON format
fsh-lint lint --format json **/*.fsh > diagnostics.json
```

## `fsh-lint format`

Format FSH files.

```bash
fsh-lint format [OPTIONS] <FILES>...
```

### Options

- `--check` - Check if files are formatted (don't modify)
- `--diff` - Show formatting differences
- `--config <PATH>` - Path to configuration file

### Examples

```bash
# Format all FSH files
fsh-lint format **/*.fsh

# Check formatting without modifying
fsh-lint format --check **/*.fsh
```

## `fsh-lint init`

Initialize configuration file.

```bash
fsh-lint init [OPTIONS]
```

### Options

- `--full` - Generate full example configuration
- `--output <PATH>` - Output path (default: `fsh-lint.json`)

### Examples

```bash
# Create default config
fsh-lint init

# Create full example
fsh-lint init --full
```

## `fsh-lint rules`

List available rules.

```bash
fsh-lint rules [OPTIONS]
```

### Options

- `--detailed` - Show detailed information
- `--category <CATEGORY>` - Filter by category
- `--search <QUERY>` - Search rules

### Examples

```bash
# List all rules
fsh-lint rules

# Show detailed info for a category
fsh-lint rules --detailed --category style

# Search for specific rules
fsh-lint rules --search naming
```

## `fsh-lint check`

Check configuration validity.

```bash
fsh-lint check [OPTIONS]
```

### Options

- `--config <PATH>` - Path to configuration file

### Examples

```bash
# Check default config
fsh-lint check

# Check specific config
fsh-lint check --config custom-config.json
```

## Global Options

Available for all commands:

- `-h, --help` - Print help information
- `-V, --version` - Print version information
- `-v, --verbose` - Enable verbose output
- `--color <WHEN>` - Colorize output: `auto`, `always`, `never`

## Exit Codes

See [Exit Codes](/cli/exit-codes/) for details.
