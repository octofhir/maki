---
title: CLI Options
description: Command-line options reference
---

Complete reference for all command-line options.

## Global Options

### `--help, -h`

Print help information for the command.

```bash
maki --help
maki lint --help
```

### `--version, -V`

Print version information.

```bash
maki --version
# Output: maki 0.1.0
```

### `--verbose, -v`

Enable verbose logging output.

```bash
maki --verbose lint **/*.fsh
```

### `--color <WHEN>`

Control color output:
- `auto` - Automatic (default)
- `always` - Always colorize
- `never` - Never colorize

```bash
maki --color always lint **/*.fsh
maki --color never lint **/*.fsh > output.txt
```

## Lint Options

### `--fix`

Automatically apply safe fixes.

```bash
maki lint --fix **/*.fsh
```

### `--severity <LEVEL>`

Filter diagnostics by minimum severity:
- `hint`
- `info`
- `warn`
- `error`

```bash
maki lint --severity error **/*.fsh
```

### `--format <FORMAT>`

Output format:
- `human` - Human-readable (default)
- `json` - JSON format
- `sarif` - SARIF format
- `github` - GitHub Actions annotations

```bash
maki lint --format json **/*.fsh
maki lint --format github **/*.fsh
```

### `--config <PATH>`

Specify configuration file path.

```bash
maki lint --config custom-config.json **/*.fsh
```

### `--no-config`

Ignore all configuration files.

```bash
maki lint --no-config **/*.fsh
```

### `--max-diagnostics <N>`

Limit number of diagnostics shown.

```bash
maki lint --max-diagnostics 50 **/*.fsh
```

### `--rule <RULE>`

Enable only specific rules.

```bash
maki lint --rule style/naming-convention **/*.fsh
maki lint --rule correctness/** **/*.fsh
```

### `--ignore-pattern <PATTERN>`

Ignore files matching pattern.

```bash
maki lint --ignore-pattern "**/*.generated.fsh" **/*.fsh
```

## Format Options

### `--check`

Check formatting without modifying files.

```bash
maki format --check **/*.fsh
```

### `--diff`

Show formatting differences.

```bash
maki format --diff **/*.fsh
```

## Init Options

### `--full`

Generate full example configuration.

```bash
maki init --full
```

### `--output <PATH>`

Specify output path for configuration.

```bash
maki init --output .makirc.json
```

## Rules Options

### `--detailed`

Show detailed rule information.

```bash
maki rules --detailed
```

### `--category <CATEGORY>`

Filter rules by category:
- `style`
- `documentation`
- `correctness`
- `suspicious`

```bash
maki rules --category style
```

### `--search <QUERY>`

Search rules by name or description.

```bash
maki rules --search naming
```

## Environment Variables

### `FSH_LINT_CONFIG`

Override configuration file path.

```bash
export FSH_LINT_CONFIG=custom-config.json
maki lint **/*.fsh
```

### `FSH_LINT_NO_COLOR`

Disable color output.

```bash
export FSH_LINT_NO_COLOR=1
maki lint **/*.fsh
```

### `FSH_LINT_CACHE_DIR`

Set cache directory location.

```bash
export FSH_LINT_CACHE_DIR=.maki-cache
maki lint **/*.fsh
```
