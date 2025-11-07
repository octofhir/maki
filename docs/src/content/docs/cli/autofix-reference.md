---
title: Autofix Quick Reference
description: Quick reference for MAKI autofix commands and options
---

Quick reference card for using MAKI's automatic fixes.

## Common Commands

```bash
# Apply safe fixes only (recommended)
maki lint --fix input/fsh/

# Apply all fixes (safe + unsafe)
maki lint --fix --unsafe input/fsh/

# Preview without modifying files
maki lint --fix --dry-run input/fsh/

# Interactive mode - review each unsafe fix
maki lint --fix --interactive input/fsh/
maki lint --fix -i input/fsh/  # short form

# Apply fixes and write (alias)
maki lint -w input/fsh/
```

## Command Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--fix` | `-w, --write` | Apply safe fixes only |
| `--unsafe` | - | Also apply unsafe fixes |
| `--dry-run` | - | Preview without modifying |
| `--interactive` | `-i` | Prompt for unsafe fixes |

## Safety Levels

### Safe Fixes (`--fix`)

✅ **No semantic changes** - safe to apply automatically

- Add missing `Id`, `Title`, `Description`
- Remove unused aliases
- Fix whitespace and formatting
- Correct syntax errors

### Unsafe Fixes (`--fix --unsafe`)

⚠️ **Semantic changes** - review carefully

- Change naming conventions
- Add FHIR constraints
- Modify cardinality
- Add binding strength
- Add extension contexts

## Interactive Prompt

When using `--interactive`, you'll see:

```
🔧 Unsafe autofix suggested
────────────────────────────────────────────────────────────────────────────
Rule:     style/naming-convention
Location: profiles/Patient.fsh:5:1
Safety:   unsafe (semantic changes, requires review)

Description: Convert Profile name to PascalCase

Changes:
-    5 │ Profile: bad_name
+    5 │ Profile: BadName

────────────────────────────────────────────────────────────────────────────
Apply this fix? [y/N/q]
```

**Options:**
- `y` - Apply this fix
- `N` - Skip (default)
- `q` - Quit

## Workflow Recommendations

### 1. Safe Fixes First

```bash
# Start with safe-only fixes
maki lint --fix input/fsh/
```

### 2. Review Unsafe Interactively

```bash
# Review each unsafe fix
maki lint --fix --unsafe -i input/fsh/
```

### 3. Preview Before Applying

```bash
# Check what would change
maki lint --fix --unsafe --dry-run input/fsh/
```

### 4. With Version Control

```bash
# Commit before autofixing
git add . && git commit -m "Before autofixes"

# Apply fixes
maki lint --fix input/fsh/

# Review and commit
git diff
git add . && git commit -m "Apply safe autofixes"
```

## Configuration

`.makirc.json` or `.makirc.toml`:

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

## Available Autofixes by Category

### Metadata (Safe)
- `required-id` - Add missing `Id`
- `required-title` - Add missing `Title`
- `required-description` - Add missing `Description`
- `code-system-content-required` - Add `^content`
- `value-set-compose-required` - Add `^compose`

### Metadata (Unsafe)
- `extension-context-required` - Add `^context`

### Naming (Unsafe)
- `profile-naming` - Convert to PascalCase
- `extension-naming` - Convert to PascalCase
- `value-set-naming` - Convert to kebab-case
- `code-system-naming` - Convert to kebab-case

### Cardinality (Safe)
- `cardinality-min-max-swapped` - Swap min/max

### Binding (Unsafe)
- `binding-strength-required` - Add binding strength

### Duplicates (Safe)
- `redundant-alias` - Remove unused aliases

## Troubleshooting

**Fixes not applied?**
- Check you're using `--fix` flag
- Unsafe fixes need `--unsafe` flag
- Review rule configuration

**Syntax errors after fix?**
- File a bug report with the FSH file
- Include command and error message

**Too many changes?**
- Use `--max-fixes-per-file` limit
- Apply fixes in multiple passes
- Use `--dry-run` to preview

## See Also

- [Automatic Fixes Guide](/guides/autofix/) - Comprehensive documentation
- [CLI Commands](/cli/commands/) - All command options
- [Configuration](/configuration/config-file/) - Config file format
