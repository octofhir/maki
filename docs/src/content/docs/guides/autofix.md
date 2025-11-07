---
title: Automatic Fixes
description: How to use MAKI's autofix engine to automatically correct FSH issues
---

MAKI includes a powerful autofix engine that can automatically correct many linting violations. This guide explains how autofixes work, their safety classifications, and how to use them effectively.

## Overview

Autofixes allow you to automatically correct common FSH issues without manual intervention. MAKI classifies fixes by safety level and provides multiple modes for applying them.

### Fix Safety Levels

MAKI classifies all fixes into two safety levels:

#### Safe Fixes
**No semantic changes** - These fixes only modify formatting, add required metadata, or remove redundant code without changing the meaning of your FSH.

**Examples:**
- Adding missing `Id`, `Title`, or `Description` fields
- Removing unused aliases
- Fixing whitespace and indentation
- Correcting punctuation

**Applied with:** `--fix` flag (default)

#### Unsafe Fixes
**Semantic changes** - These fixes modify the meaning or behavior of your FSH code and should be reviewed carefully.

**Examples:**
- Converting identifier naming conventions (e.g., `bad_name` → `BadName`)
- Adding FHIR constraints (cardinality, binding strength)
- Adding extension contexts
- Swapping reversed min/max in cardinality

**Applied with:** `--unsafe` flag

:::caution
Unsafe fixes change your code's behavior. Always review them before committing to version control.
:::

## Using Autofixes

### Apply Safe Fixes Only

The safest way to use autofixes is to apply only safe fixes:

```bash
maki lint --fix input/fsh/
```

This applies all **safe fixes** only, leaving unsafe fixes for manual review.

### Apply All Fixes (Including Unsafe)

To apply all fixes including semantic changes:

```bash
maki lint --fix --unsafe input/fsh/
```

:::tip
Combine with `--dry-run` first to preview what will change:
```bash
maki lint --fix --unsafe --dry-run input/fsh/
```
:::

### Interactive Mode

For maximum control, use interactive mode to review each unsafe fix:

```bash
maki lint --fix --interactive input/fsh/
```

Interactive mode will:
1. Automatically apply all **safe** fixes without prompting
2. **Prompt for confirmation** on each unsafe fix
3. Show a detailed preview with context
4. Allow you to accept, skip, or quit

#### Interactive Prompt Example

```
🔧 Unsafe autofix suggested
────────────────────────────────────────────────────────────────────────────
Rule:     style/naming-convention
Location: profiles/Patient.fsh:5:1
Safety:   unsafe (semantic changes, requires review)

Description: Convert Profile name to PascalCase

Changes:
     3 │ ValueSet: MyValueSet
     4 │
-    5 │ Profile: bad_name
+    5 │ Profile: BadName
     6 │ Parent: Patient
     7 │

────────────────────────────────────────────────────────────────────────────
Apply this fix? [y/N/q]
```

**Options:**
- `y` - Apply this fix
- `N` - Skip this fix (default)
- `q` - Quit and cancel all remaining fixes

### Dry-Run Mode

Preview what would change without modifying files:

```bash
# Preview safe fixes
maki lint --fix --dry-run input/fsh/

# Preview all fixes (safe + unsafe)
maki lint --fix --unsafe --dry-run input/fsh/
```

Dry-run mode shows:
- Which files will be modified
- Detailed diff for each fix
- Total number of fixes by safety level

## How Autofixes Work

### Conflict Detection

MAKI automatically detects when multiple fixes would modify overlapping text and resolves conflicts using priority-based selection:

1. **Detects overlapping ranges** - Identifies fixes that would conflict
2. **Prioritizes by safety** - Safe fixes have higher priority than unsafe fixes
3. **Selects best fix** - Chooses the highest-priority fix and skips conflicting ones

### Fix Application Order

Fixes are applied in **reverse offset order** (from end of file to beginning) to preserve text positions as fixes are applied.

### Syntax Validation

After applying fixes, MAKI validates the modified FSH syntax to ensure:
- Balanced brackets and braces
- Valid FSH structure
- No syntax errors introduced

If validation fails, changes are rolled back and an error is reported.

## Available Autofixes

### Metadata Rules

| Rule | Fix | Safety |
|------|-----|--------|
| `required-id` | Add missing `Id` field | Safe |
| `required-title` | Add missing `Title` field | Safe |
| `required-description` | Add missing `Description` field | Safe |
| `extension-context-required` | Add extension `^context` | Unsafe |
| `code-system-content-required` | Add `^content` field | Safe |
| `value-set-compose-required` | Add `^compose` field | Safe |

### Naming Convention Rules

| Rule | Fix | Safety |
|------|-----|--------|
| `profile-naming` | Convert Profile ID to PascalCase | Unsafe |
| `extension-naming` | Convert Extension ID to PascalCase | Unsafe |
| `value-set-naming` | Convert ValueSet ID to kebab-case | Unsafe |
| `code-system-naming` | Convert CodeSystem ID to kebab-case | Unsafe |

### Cardinality Rules

| Rule | Fix | Safety |
|------|-----|--------|
| `cardinality-min-max-swapped` | Swap reversed min/max values | Safe |

### Binding Rules

| Rule | Fix | Safety |
|------|-----|--------|
| `binding-strength-required` | Add missing binding strength | Unsafe |

### Duplicate Detection Rules

| Rule | Fix | Safety |
|------|-----|--------|
| `redundant-alias` | Remove unused alias definitions | Safe |

## Configuration

Configure autofix behavior in your `.makirc.json` or `.makirc.toml`:

```json
{
  "autofix": {
    "default_safety": "safe",
    "interactive": false,
    "create_backups": true,
    "backup_extension": ".bak",
    "max_fixes_per_file": 100
  }
}
```

### Configuration Options

- **`default_safety`** - Default safety level for `--fix` flag
  - `"safe"` (default) - Only apply safe fixes
  - `"unsafe"` - Apply all fixes

- **`interactive`** - Enable interactive mode by default
  - `false` (default) - Apply fixes automatically
  - `true` - Prompt for confirmation on unsafe fixes

- **`create_backups`** - Create backup files before modifying
  - `true` (default) - Create `.bak` files
  - `false` - Don't create backups

- **`backup_extension`** - File extension for backups
  - `".bak"` (default)

- **`max_fixes_per_file`** - Limit fixes per file
  - `100` (default)
  - Prevents excessive modifications in a single pass

## Best Practices

### 1. Start with Safe Fixes

Always start with safe-only fixes to catch low-hanging fruit:

```bash
maki lint --fix input/fsh/
```

### 2. Review Unsafe Fixes Interactively

For unsafe fixes, use interactive mode to review each change:

```bash
maki lint --fix --unsafe --interactive input/fsh/
```

### 3. Use Dry-Run Before Committing

Preview changes before applying to understand their impact:

```bash
maki lint --fix --unsafe --dry-run input/fsh/
```

### 4. Integrate with Version Control

Combine autofixes with git for safety:

```bash
# Commit your work first
git add .
git commit -m "Before autofixes"

# Apply fixes
maki lint --fix input/fsh/

# Review changes
git diff

# Commit if satisfied, or revert
git add . && git commit -m "Apply safe autofixes"
# or
git restore .
```

### 5. Run in CI/CD

Use autofixes in CI/CD to enforce code quality:

```yaml
# .github/workflows/lint.yml
- name: Check for fixable issues
  run: |
    maki lint --fix --dry-run input/fsh/
    if [ $? -ne 0 ]; then
      echo "Fixable issues found. Run 'maki lint --fix' locally."
      exit 1
    fi
```

## Statistics and Reporting

After applying fixes, MAKI displays comprehensive statistics:

```
═══════════════════════════════════════════════════════════════════════════
📊 Autofix Statistics
═══════════════════════════════════════════════════════════════════════════

📈 Overall:
  Total fixes:         24
  ✅ Applied (safe):    17
  ⚠️  Applied (unsafe):  5
  ❌ Failed:            2
  ⏭️  Skipped:           0
  📁 Files modified:    8

📋 By Rule:
  ✅ required-id
     Applied: 8 | Failed: 0
  ⚠️ naming-convention
     Applied: 5 | Failed: 0
  ✅ redundant-alias
     Applied: 4 | Failed: 0
```

## Troubleshooting

### Fixes Not Being Applied

**Check safety level:** Ensure you're using the correct flag:
- `--fix` applies safe fixes only
- `--fix --unsafe` applies all fixes

**Check configuration:** Verify your config doesn't disable autofixes for specific rules.

### Syntax Errors After Fixing

**File a bug report:** If autofixes introduce syntax errors, this is a bug. Please report it with:
- The original FSH file
- The command you ran
- The error message

### Conflicts Between Fixes

MAKI automatically resolves conflicts, but if fixes are skipped:
- Review the diagnostic output
- Apply fixes in multiple passes if needed
- Consider manual intervention for complex cases

## Advanced Usage

### Batch Processing

Apply fixes to multiple directories:

```bash
for dir in input/fsh/*; do
  maki lint --fix "$dir"
done
```

### Selective Fix Application

Disable specific rules and their fixes:

```json
{
  "rules": {
    "style/naming-convention": "off"
  }
}
```

Then apply only remaining fixes:

```bash
maki lint --fix --unsafe input/fsh/
```

### Custom Fix Scripts

Combine MAKI with custom scripts for complex workflows:

```bash
#!/bin/bash
# apply-fixes.sh

# Apply safe fixes
maki lint --fix input/fsh/

# Apply specific unsafe fixes interactively
maki lint --fix --unsafe --interactive \
  --config unsafe-fixes-only.json \
  input/fsh/

# Run final validation
maki lint input/fsh/
```

## Performance

MAKI's autofix engine is highly optimized:

- **Conflict detection:** O(n²) worst case, typically O(n log n) with smart grouping
- **Fix application:** O(n) per file with reverse-order optimization
- **Typical performance:** <100ms for 50 fixes per file

For large codebases (1000+ files), consider:
- Processing files in parallel using GNU parallel or similar
- Using `--max-fixes-per-file` to limit single-pass changes
- Running multiple targeted passes for different rule categories

## Summary

MAKI's autofix engine provides:

✅ **Safety-first approach** - Safe fixes by default, unsafe with explicit flag
✅ **Interactive review** - Control over semantic changes
✅ **Conflict resolution** - Automatic handling of overlapping fixes
✅ **Dry-run preview** - See changes before applying
✅ **Comprehensive statistics** - Detailed reporting of applied fixes
✅ **High performance** - Fast processing even for large codebases

By following these guidelines, you can safely automate most FSH code corrections while maintaining control over semantic changes.
