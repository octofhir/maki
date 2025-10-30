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

- `--fix` - Automatically fix issues when possible
- `--severity <LEVEL>` - Only show diagnostics at or above this level
- `--format <FORMAT>` - Output format: `human`, `json`, `sarif`, `github`
- `--config <PATH>` - Path to configuration file
- `--no-config` - Don't load configuration files
- `--max-diagnostics <N>` - Limit number of diagnostics shown

### Examples

```bash
# Lint all FSH files
maki lint **/*.fsh

# Lint with automatic fixes
maki lint --fix input/fsh/*.fsh

# Show only errors
maki lint --severity error **/*.fsh

# Output JSON format
maki lint --format json **/*.fsh > diagnostics.json
```

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
