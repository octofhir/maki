---
title: Getting Started with GritQL Autofixes
description: A quick guide to creating your first GritQL autofix patterns in MAKI
---

A quick guide to creating your first GritQL autofix patterns in MAKI.

## Prerequisites

- MAKI installed locally
- Basic understanding of FSH syntax
- Familiarity with regex patterns

## 5-Minute Quick Start

### 1. Create Your First Pattern

Create a file `my-rules.grit`:

```gritql
// Fix: lowercase profile names
Profile: $name where {
    $name <: r"^[a-z]"
} => {
    rewrite($name) => capitalize($name)
}
```

### 2. Test It

Create a test file `test.fsh`:

```fsh
Profile: myPatientProfile
Parent: Patient
Title: "Test Profile"
```

### 3. See the Issue

```bash
maki lint test.fsh
```

Output:
```
[W] rule-id: Profile name starts with lowercase (test.fsh:1:1)
```

### 4. Apply the Fix

```bash
maki lint --fix test.fsh
```

Result in `test.fsh`:
```fsh
Profile: MyPatientProfile
Parent: Patient
Title: "Test Profile"
```

**Done!** You've created your first autofix.

## Common Patterns

### Pattern 1: Naming Convention

**Problem**: ID fields not in kebab-case

```gritql
Profile: $p where {
    $p.id <: r"[A-Z_]"
}
=> rewrite($p.id) => to_kebab_case($p.id)
```

**Usage**:
```bash
maki lint --fix examples/
```

### Pattern 2: Add Missing Field

**Problem**: Profile missing Title

```gritql
Profile: $p where {
    not $p.title
}
=> insert_after($p.name) => "\nTitle: \"$p Profile\""
```

### Pattern 3: Standardize Prefix

**Problem**: Names don't follow org standards

```gritql
Profile: $p where {
    not ($p.name <: r"^OrgPrefix")
}
=> rewrite($p.name) => concat("OrgPrefix-", $p.name)
```

### Pattern 4: Fix Multiple Fields

**Problem**: ID needs kebab-case, name needs capitalization

```gritql
Profile: $p where {
    ($p.id <: r"[A-Z_]") and ($p.name <: r"^[a-z]")
}
=> {
    rewrite($p.id) => to_kebab_case($p.id),
    rewrite($p.name) => capitalize($p.name)
}
```

## Step-by-Step: Create a Real Pattern

Let's build a pattern to fix common FSH issues.

### Step 1: Identify the Problem

From your team's FSH code, you notice:
- Profile names start with lowercase
- IDs use underscores instead of hyphens
- Missing Title fields

### Step 2: Create the Pattern File

Create `rules/naming-conventions.grit`:

```gritql
// Rule 1: Capitalize profile names
Profile: $name where {
    $name <: r"^[a-z]"
} => rewrite($name) => capitalize($name)

// Rule 2: Convert ID underscores to hyphens
Profile: $p where {
    $p.id <: r"_"
} => rewrite($p.id) => replace($p.id, "_", "-")

// Rule 3: Add missing Title
Profile: $p where {
    not $p.title
} => insert_after($p.name) => "\nTitle: \"$p\""
```

### Step 3: Test the Rules

```bash
# See what would change
maki lint --fix --dry-run examples/

# Apply safe fixes only
maki lint --fix examples/

# Review the results
git diff
```

### Step 4: Refine as Needed

Adjust patterns based on actual results:
- Remove overly aggressive patterns
- Combine related patterns
- Add safety conditions

### Step 5: Deploy

Add to your CI/CD:

```yaml
# .github/workflows/lint.yml
- name: Lint FSH
  run: maki lint --fix examples/

- name: Check for changes
  run: git diff --exit-code
```

## Transformation Functions Reference

Use these when defining rewrites:

```gritql
// Case conversion
capitalize($name)              // "bad" → "Bad"
to_kebab_case($name)          // "BadName" → "bad-name"
to_pascal_case($name)         // "bad-name" → "BadName"
to_snake_case($name)          // "BadName" → "bad_name"
lowercase($name)              // "BadName" → "badname"
uppercase($name)              // "badname" → "BADNAME"

// String operations
trim($text)                   // "  text  " → "text"
replace($text, "_", "-")      // "bad_name" → "bad-name"
concat("prefix-", $name)      // Concatenate strings
```

## Common Issues & Solutions

### Issue 1: Pattern Doesn't Match

**Problem**: Your pattern doesn't match anything

**Solution**: Check your pattern syntax
```gritql
# Wrong - missing braces
Profile: $name where $name <: r"^[a-z]"

# Correct - with braces
Profile: $name where {
    $name <: r"^[a-z]"
}
```

### Issue 2: Rewrite Creates Invalid Syntax

**Problem**: Fixed code doesn't parse

**Solution**: Use `--dry-run` to preview
```bash
maki lint --fix --dry-run test.fsh
# Review output before applying
```

### Issue 3: Fix Not Applied

**Problem**: You ran `--fix` but nothing changed

**Causes**:
1. Pattern doesn't match anything
2. Fix is classified as unsafe (use `--unsafe`)
3. Another fix conflicts with it

**Solution**:
```bash
# Check what matches
maki lint test.fsh

# Preview changes
maki lint --fix --dry-run test.fsh

# Apply unsafe fixes if needed
maki lint --fix --unsafe test.fsh
```

### Issue 4: Undefined Variable Error

**Problem**: "Undefined variable: $xyz"

**Solution**: Ensure variable is captured in pattern
```gritql
# Wrong - $id not captured
Profile: $p where { ... } => rewrite($id) => ...

# Correct - capture $id
Profile: $p where { $id == "test" } => rewrite($id) => ...
```

## Safe vs Unsafe Fixes

### Safe Fixes (Applied with `--fix`)
- Case conversions
- Name/ID formatting
- String replacements
- Whitespace fixes

```gritql
=> rewrite($id) => to_kebab_case($id)  // Safe
```

### Unsafe Fixes (Require `--unsafe`)
- Adding/removing fields
- Semantic changes
- Complex transformations

```gritql
=> insert_after($p.name) => "\nTitle: \"New\""  // Unsafe
=> delete($field)                              // Unsafe
```

**Best Practice**: Start with safe fixes, use `--unsafe` only when necessary and reviewed.

## Testing Your Patterns

### Test Strategy

1. **Create test files** with known issues
2. **Preview changes** with `--dry-run`
3. **Review output** carefully
4. **Apply safely** with `--fix`
5. **Verify results** with `git diff`

### Example Test Suite

```
tests/
├── naming/
│   ├── lowercase-profile.fsh      # Test case
│   ├── underscore-id.fsh          # Test case
│   └── missing-title.fsh          # Test case
└── validation/
    ├── valid-output.fsh            # Expected result
    └── conflicts.fsh               # Edge case
```

### Run Tests

```bash
# Run pattern on test files
for file in tests/naming/*.fsh; do
  echo "=== Testing $file ==="
  maki lint --fix --dry-run "$file"
done
```

## Best Practices

### 1. Start Simple
```gritql
# Good: Single, clear transformation
=> rewrite($name) => capitalize($name)

# Avoid: Complex multi-step logic
=> multiple operations together
```

### 2. Document Your Patterns
```gritql
// Fix: Ensure profile names start with uppercase
// Rule: All profile names should follow PascalCase convention
// Safe: Yes, only affects naming, no semantic changes
Profile: $name where {
    $name <: r"^[a-z]"
} => rewrite($name) => capitalize($name)
```

### 3. Test Before Deploying
```bash
# Always preview first
maki lint --fix --dry-run examples/ > /tmp/preview.diff

# Review changes
less /tmp/preview.diff

# Then apply
maki lint --fix examples/
```

### 4. Use Version Control
```bash
# Before applying fixes
git status

# Apply fixes
maki lint --fix examples/

# Review changes
git diff

# Commit if happy
git add examples/
git commit -m "fix: Apply naming convention patterns"
```

### 5. Gradual Rollout
```bash
# Phase 1: Dry run on small subset
maki lint --fix --dry-run examples/single/

# Phase 2: Apply to that subset
maki lint --fix examples/single/

# Phase 3: Expand scope
maki lint --fix examples/
```

## Advanced: Multiple Rules in One File

Organize related patterns together:

```gritql
// ===== NAMING CONVENTIONS =====

// Profile names must use PascalCase
Profile: $name where {
    $name <: r"^[a-z]" or $name <: r"[_-]"
} => rewrite($name) => capitalize($name)

// IDs must use kebab-case
Profile: $p where {
    $p.id <: r"[A-Z_]"
} => rewrite($p.id) => to_kebab_case($p.id)

// ===== MISSING METADATA =====

// Profile without Title
Profile: $p where {
    not $p.title
} => insert_after($p.name) => "\nTitle: \"$p\""

// ValueSet without Title
ValueSet: $v where {
    not $v.title
} => insert_after($v.name) => "\nTitle: \"$v Codes\""
```

## Integration with CI/CD

### GitHub Actions

```yaml
name: Lint & Fix FSH

on: [pull_request, push]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install MAKI
        run: cargo install maki --version 0.0.2

      - name: Lint FSH
        run: maki lint examples/

      - name: Check formatting
        run: |
          maki lint --fix --dry-run examples/ > /tmp/fixes.diff
          git diff --exit-code || {
            echo "⚠️ Found formatting issues. Run 'maki lint --fix'"
            cat /tmp/fixes.diff
            exit 1
          }
```

### GitLab CI

```yaml
lint_fsh:
  stage: lint
  script:
    - cargo install maki --version 0.0.2
    - maki lint examples/
    - maki lint --fix --dry-run examples/
  allow_failure: true
```

## Next Steps

1. **Read the full guide**: [GritQL Autofixes](/docs/guides/gritql/autofixes)
2. **API Reference**: [Complete API Docs](/docs/guides/gritql/api-reference)
3. **Contribute**: [Contributing Guidelines](/guides/contributing)

## Getting Help

- 📖 Check the [documentation](/docs)
- 💬 Ask in [discussions](https://github.com/octofhir/maki/discussions)
- 🐛 Report issues on [GitHub](https://github.com/octofhir/maki/issues)
- 🤝 Contribute patterns to the community

Happy linting! 🎉
