---
title: FSH Formatter
description: Automatic code formatting for FHIR Shorthand files
---

The FSH Formatter automatically formats your FHIR Shorthand files to maintain consistent code style across your project. It leverages a lossless Concrete Syntax Tree (CST) to ensure perfect preservation of comments, blank lines, and all semantic content.

## Quick Start

Format your FSH files with a single command:

```bash
# Format all FSH files in a directory
maki format input/fsh/*.fsh

# Check if files are formatted without modifying them
maki format --check input/fsh/*.fsh

# Show formatting differences
maki format --diff input/fsh/*.fsh
```

## Key Features

### Lossless Formatting

The formatter uses Rowan-based CST (Concrete Syntax Tree) to ensure:

- **All comments preserved** - Line comments, block comments, and documentation
- **Blank lines maintained** - Intentional spacing between definitions
- **Perfect reconstruction** - `parse(format(source)) == parse(source)`
- **No semantic changes** - Only formatting is modified

### Configurable Style

Control formatting behavior with configuration options:

- Indent style (spaces or tabs)
- Line width for wrapping
- Rule alignment
- Spacing normalization
- Blank line handling

### High Performance

Built in Rust for speed:

- Formats files in <50ms each
- Parallel processing for multiple files
- Token optimization for 2-5% performance boost
- Efficient memory usage

## Formatting Options

### Indent Style

Control how code is indented:

```toml
[format]
indent_style = "spaces"  # Options: "spaces", "tabs"
indent_size = 2          # Number of spaces (when using spaces)
```

**Example:**

```fsh
// Before (mixed indentation)
Profile: MyProfile
Parent: Patient
* name 1..1
    * given 1..1
  * family 1..1

// After (consistent 2-space indentation)
Profile: MyProfile
Parent: Patient
* name 1..1
  * given 1..1
  * family 1..1
```

### Line Width

Set maximum line width before wrapping:

```toml
[format]
line_width = 120
```

### Rule Alignment

Align rules for better readability:

```toml
[format]
align_rules = true
```

**Example:**

```fsh
// Before (no alignment)
Profile: MyProfile
Parent: Patient
* name 1..1 MS
* birthDate 1..1 MS
* gender 1..1 MS

// After (aligned)
Profile: MyProfile
Parent: Patient
* name      1..1 MS
* birthDate 1..1 MS
* gender    1..1 MS
```

### Spacing Normalization

Normalize spacing around operators:

```toml
[format]
normalize_spacing = true
```

**Example:**

```fsh
// Before (inconsistent spacing)
Profile:MyProfile
Parent:Patient
Id:  my-profile
Title:"My Profile"

// After (normalized)
Profile: MyProfile
Parent: Patient
Id: my-profile
Title: "My Profile"
```

### Blank Line Control

Control blank lines between sections:

```toml
[format]
preserve_blank_lines = true
max_blank_lines = 2
blank_lines_between_groups = 1
```

## Configuration File

Add formatting options to your `maki.json` or `maki.toml`:

```json
{
  "format": {
    "indent_style": "spaces",
    "indent_size": 2,
    "line_width": 120,
    "align_rules": true,
    "group_rules": false,
    "sort_rules": false,
    "normalize_spacing": true,
    "preserve_blank_lines": true,
    "max_blank_lines": 2,
    "blank_lines_between_groups": 1
  }
}
```

Or in TOML:

```toml
[format]
indent_style = "spaces"
indent_size = 2
line_width = 120
align_rules = true
group_rules = false
sort_rules = false
normalize_spacing = true
preserve_blank_lines = true
max_blank_lines = 2
blank_lines_between_groups = 1
```

## Default Settings

The formatter uses sensible defaults if no configuration is provided:

| Option | Default | Description |
|--------|---------|-------------|
| `indent_style` | `"spaces"` | Use spaces for indentation |
| `indent_size` | `2` | 2 spaces per indent level |
| `line_width` | `120` | Maximum line width |
| `align_rules` | `true` | Align rule elements |
| `group_rules` | `false` | Don't group rules by type |
| `sort_rules` | `false` | Don't sort rules |
| `normalize_spacing` | `true` | Normalize spacing around `:` and `=` |
| `preserve_blank_lines` | `true` | Keep intentional blank lines |
| `max_blank_lines` | `2` | Maximum consecutive blank lines |
| `blank_lines_between_groups` | `1` | Blank lines between rule groups |

## Special Cases

### Multiline Strings

The formatter preserves multiline string content exactly as written:

```fsh
Profile: MyProfile
Parent: Patient
Description: """
This is a multi-line
description that will be
preserved exactly as-is.
"""
```

### Comments

All comment styles are preserved:

```fsh
// Line comment before profile
Profile: MyProfile
Parent: Patient

// Comment before rule
* name 1..1 MS  // Inline comment

/*
 * Block comment
 * spanning multiple lines
 */
* gender 1..1 MS
```

### Mapping Multi-line Comments

Mappings with multi-line comments are handled correctly (fixes SUSHI issues #1577, #1576):

```fsh
Mapping: PatientMapping
Source: MyProfile
Target: "http://example.org"
* -> "Patient" """
This is a multi-line
mapping comment.
"""
```

## CLI Usage

### Basic Formatting

```bash
# Format a single file
maki format profile.fsh

# Format multiple files
maki format profile.fsh extension.fsh valueset.fsh

# Format all FSH files in directory
maki format input/fsh/*.fsh

# Format recursively
maki format **/*.fsh
```

### Check Mode

Check if files are formatted without modifying them:

```bash
maki format --check input/fsh/*.fsh
```

Exit codes:
- `0` - All files are formatted
- `1` - Some files need formatting
- `2` - Error occurred

### Diff Mode

Show what would change without modifying files:

```bash
maki format --diff input/fsh/*.fsh
```

**Example output:**

```diff
--- input/fsh/profile.fsh
+++ input/fsh/profile.fsh (formatted)
@@ -1,5 +1,5 @@
-Profile:MyProfile
-Parent:Patient
+Profile: MyProfile
+Parent: Patient
 * name 1..1 MS
-* birthDate 1..1 MS
-* gender 1..1 MS
+* birthDate  1..1 MS
+* gender     1..1 MS
```

### Custom Configuration

Use a specific configuration file:

```bash
maki format --config custom-config.json input/fsh/*.fsh
```

## CI/CD Integration

### GitHub Actions

Add formatting checks to your workflow:

```yaml
name: Format Check

on: [push, pull_request]

jobs:
  format:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install maki
        run: cargo install maki

      - name: Check formatting
        run: maki format --check input/fsh/**/*.fsh
```

### Pre-commit Hook

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Format FSH files before commit
maki format input/fsh/**/*.fsh
git add input/fsh/**/*.fsh
```

### Make it executable:

```bash
chmod +x .git/hooks/pre-commit
```

## Editor Integration

### VS Code

Add to your settings:

```json
{
  "[fsh]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "octofhir.maki"
  }
}
```

### Command Palette

1. Open Command Palette (Cmd/Ctrl+Shift+P)
2. Search "Format Document"
3. Select formatter: maki

## Best Practices

### Format Early and Often

Format your code regularly to catch style issues early:

```bash
# Before committing
maki format --check input/fsh/**/*.fsh

# Or auto-format
maki format input/fsh/**/*.fsh
```

### Consistent Team Style

Share your `maki.json` configuration in version control so the entire team uses the same formatting rules.

### Combine with Linting

Use formatting alongside linting for comprehensive code quality:

```bash
# Format first
maki format input/fsh/**/*.fsh

# Then lint
maki lint input/fsh/**/*.fsh
```

### Use in Pre-commit Hooks

Automate formatting with git hooks to ensure all commits are formatted:

```bash
# .git/hooks/pre-commit
#!/bin/bash
maki format --check input/fsh/**/*.fsh
if [ $? -ne 0 ]; then
  echo "Files are not formatted. Run 'maki format input/fsh/**/*.fsh'"
  exit 1
fi
```

## Performance

The formatter is optimized for speed:

- **Single files**: <50ms
- **Large projects**: Parallel processing
- **Token optimization**: 2-5% performance boost
- **Memory efficient**: Streaming processing

### Benchmark Results

Typical formatting performance on real-world projects:

| Project Size | Files | Time | Throughput |
|--------------|-------|------|------------|
| Small | 10 files | ~50ms | 200 files/sec |
| Medium | 100 files | ~300ms | 330 files/sec |
| Large | 1000 files | ~2s | 500 files/sec |

## Troubleshooting

### Formatting Doesn't Match Expected Output

Check your configuration file:

```bash
# Verify config is loaded
maki format --check --verbose input/fsh/profile.fsh
```

### Performance Issues

For large projects, use parallel processing (automatic with multiple files):

```bash
# Formats files in parallel
maki format input/fsh/**/*.fsh
```

### Preserve Specific Formatting

If you need to preserve specific formatting in a section, use comments:

```fsh
// maki-format-off
* name 1..1     MS
* custom   formatting here
// maki-format-on
```

Note: Format control comments are planned for a future release.

## Related Features

- [Automatic Fixes](/maki/guides/autofix/) - Combine formatting with rule fixes
- [CI/CD Integration](/maki/guides/ci-cd/) - Automate formatting checks
- [Editor Integration](/maki/guides/editors/) - Format on save

## Technical Details

### Lossless CST

The formatter uses Rowan-based Concrete Syntax Tree:

- **Green Tree**: Immutable, position-independent storage with all trivia
- **Red Tree**: Dynamic view with parent pointers for efficient traversal
- **Lossless Property**: `parse(format(parse(source))) == parse(source)`

### Token Optimization

Based on optimizations from Ruff and Biome formatters:

- **Token variant**: Static keywords/operators (fast path)
- **Text variant**: Dynamic content from source (slow path)
- **70-85% fast path usage**: High keyword density in FSH
- **2-5% performance improvement**: Proven optimization strategy

### SUSHI Compatibility

The formatter addresses known SUSHI formatting issues:

- **#1569**: Preserves triple-quote endings
- **#1577, #1576**: Handles mapping multi-line delimiters
- **#693**: Accepts missing whitespace in input

## Future Enhancements

Planned improvements for future releases:

1. **Smart line breaking**: Intelligent wrapping of long lines
2. **Custom formatting rules**: User-defined formatting plugins
3. **Format on save**: LSP integration for automatic formatting
4. **Diff-aware formatting**: Only format changed lines
5. **Format control comments**: Selectively disable formatting
6. **Rule grouping**: Group rules by type (metadata, constraints, flags)
7. **Rule sorting**: Alphabetical or custom sorting within groups
