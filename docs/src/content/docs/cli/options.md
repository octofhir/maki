---
title: CLI Options
description: Command-line options reference
---

Complete reference for all command-line options.

## Global Options

### `--help, -h`

Print help information for the command.

```bash
fsh-lint --help
fsh-lint lint --help
```

### `--version, -V`

Print version information.

```bash
fsh-lint --version
# Output: fsh-lint 0.1.0
```

### `--verbose, -v`

Enable verbose logging output.

```bash
fsh-lint --verbose lint **/*.fsh
```

### `--color <WHEN>`

Control color output:
- `auto` - Automatic (default)
- `always` - Always colorize
- `never` - Never colorize

```bash
fsh-lint --color always lint **/*.fsh
fsh-lint --color never lint **/*.fsh > output.txt
```

## Lint Options

### `--fix`

Automatically apply safe fixes.

```bash
fsh-lint lint --fix **/*.fsh
```

### `--severity <LEVEL>`

Filter diagnostics by minimum severity:
- `hint`
- `info`
- `warn`
- `error`

```bash
fsh-lint lint --severity error **/*.fsh
```

### `--format <FORMAT>`

Output format:
- `human` - Human-readable (default)
- `json` - JSON format
- `sarif` - SARIF format
- `github` - GitHub Actions annotations

```bash
fsh-lint lint --format json **/*.fsh
fsh-lint lint --format github **/*.fsh
```

### `--config <PATH>`

Specify configuration file path.

```bash
fsh-lint lint --config custom-config.json **/*.fsh
```

### `--no-config`

Ignore all configuration files.

```bash
fsh-lint lint --no-config **/*.fsh
```

### `--max-diagnostics <N>`

Limit number of diagnostics shown.

```bash
fsh-lint lint --max-diagnostics 50 **/*.fsh
```

### `--rule <RULE>`

Enable only specific rules.

```bash
fsh-lint lint --rule style/naming-convention **/*.fsh
fsh-lint lint --rule correctness/** **/*.fsh
```

### `--ignore-pattern <PATTERN>`

Ignore files matching pattern.

```bash
fsh-lint lint --ignore-pattern "**/*.generated.fsh" **/*.fsh
```

## Format Options

### `--check`

Check formatting without modifying files.

```bash
fsh-lint format --check **/*.fsh
```

### `--diff`

Show formatting differences.

```bash
fsh-lint format --diff **/*.fsh
```

## Init Options

### `--full`

Generate full example configuration.

```bash
fsh-lint init --full
```

### `--output <PATH>`

Specify output path for configuration.

```bash
fsh-lint init --output .fshlintrc.json
```

## Rules Options

### `--detailed`

Show detailed rule information.

```bash
fsh-lint rules --detailed
```

### `--category <CATEGORY>`

Filter rules by category:
- `style`
- `documentation`
- `correctness`
- `suspicious`

```bash
fsh-lint rules --category style
```

### `--search <QUERY>`

Search rules by name or description.

```bash
fsh-lint rules --search naming
```

## Environment Variables

### `FSH_LINT_CONFIG`

Override configuration file path.

```bash
export FSH_LINT_CONFIG=custom-config.json
fsh-lint lint **/*.fsh
```

### `FSH_LINT_NO_COLOR`

Disable color output.

```bash
export FSH_LINT_NO_COLOR=1
fsh-lint lint **/*.fsh
```

### `FSH_LINT_CACHE_DIR`

Set cache directory location.

```bash
export FSH_LINT_CACHE_DIR=.fsh-lint-cache
fsh-lint lint **/*.fsh
```
