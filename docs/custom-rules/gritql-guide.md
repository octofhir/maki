# Writing Custom GritQL Rules for FSH Lint

This guide explains how to write custom linting rules for FHIR Shorthand (FSH) files using GritQL pattern matching.

## Table of Contents

- [Quick Start](#quick-start)
- [GritQL Basics](#gritql-basics)
- [FSH-Specific Patterns](#fsh-specific-patterns)
- [Rule File Structure](#rule-file-structure)
- [Testing Your Rules](#testing-your-rules)
- [Advanced Patterns](#advanced-patterns)
- [Best Practices](#best-practices)

## Quick Start

### 1. Create a Rule Directory

```bash
mkdir -p rules/custom
```

### 2. Write Your First Rule

Create `rules/custom/my-rule.grit`:

```gritql
// Profiles must have a description
Profile: $name where {
    not contains "Description:"
}
```

### 3. Configure maki

Add to `.makirc.json`:

```json
{
  "linter": {
    "ruleDirectories": ["rules/custom"]
  }
}
```

### 4. Run the Linter

```bash
maki lint my-profile.fsh
```

## GritQL Basics

GritQL is a declarative pattern matching language. It lets you find code patterns and optionally transform them.

### Basic Syntax

```gritql
pattern where {
    condition1,
    condition2
}
```

### Variables

Capture parts of the match using `$variable`:

```gritql
Profile: $profileName where {
    // $profileName contains the profile's name
}
```

### Conditions

#### Regex Matching

Use `<:` for regex patterns:

```gritql
$name <: r"^[a-z]"  // Name starts with lowercase
```

#### Contains

Check if text contains a substring:

```gritql
contains "^url"  // Has a URL assignment
not contains "Description:"  // Missing description
```

#### Logical Operators

Combine conditions:

```gritql
and {
    condition1,
    condition2
}

or {
    condition1,
    condition2
}

not condition
```

## FSH-Specific Patterns

### Resource Types

Match specific FSH resource declarations:

```gritql
// Profiles
Profile: $name where { ... }

// Extensions
Extension: $name where { ... }

// Value Sets
ValueSet: $name where { ... }

// Code Systems
CodeSystem: $name where { ... }

// Instances
Instance: $name where { ... }
```

### Common FSH Patterns

#### Check Naming Conventions

```gritql
// Profile names should be PascalCase
Profile: $name where {
    $name <: r"^[a-z]"  // Starts with lowercase (violation)
}
```

#### Require Metadata Fields

```gritql
// Extensions must have URL
Extension: $name where {
    not contains "^url"
}

// Profiles must have status
Profile: $name where {
    not contains "^status"
}
```

#### Detect Patterns

```gritql
// Find profiles extending specific base
Profile: $name where {
    contains "Parent: Patient"
}
```

## Rule File Structure

### Complete Example

```gritql
// Rule: extension-url-required
//
// **Category**: Correctness
// **Severity**: Error
//
// Extensions must define a canonical URL via ^url assignment.
// This is required for proper FHIR validation.
//
// ## Examples
//
// ❌ Bad:
// Extension: MyExtension
// * value[x] only string
//
// ✅ Good:
// Extension: MyExtension
// * ^url = "http://example.org/..."
// * value[x] only string

Extension: $name where {
    not contains "^url"
}
```

### Documentation Sections

1. **Title**: Short description of the rule
2. **Category**: correctness, style, documentation, etc.
3. **Severity**: error, warning, info
4. **Description**: Why this rule matters
5. **Examples**: Show violations and fixes
6. **Pattern**: The actual GritQL pattern

## Testing Your Rules

### Manual Testing

```bash
# Create test file
cat > test.fsh << 'EOFF'
Extension: MyExtension
* value[x] only string
EOFF

# Run linter
maki lint test.fsh

# Should show violation from your custom rule
```

### Verify Rule Loading

```bash
# List all rules (including custom)
maki rules --detailed

# Filter custom rules
maki rules --category custom

# Search for your rule
maki rules search "my-rule"
```

### Debug Mode

Run with verbose logging:

```bash
maki lint test.fsh -vv 2>&1 | grep -i "custom\|gritql"
```

## Advanced Patterns

### Multiple Conditions

```gritql
Profile: $name where {
    and {
        // Name starts with "US"
        $name <: r"^US",
        // Has experimental flag
        contains "^experimental = true",
        // Missing publisher
        not contains "^publisher"
    }
}
```

### Alternative Patterns

```gritql
// Match profiles with invalid status values
Profile: $name where {
    or {
        contains "^status = #invalid",
        contains "^status = #unknown",
        contains "^status = #deprecated"
    }
}
```

### Complex Regex

```gritql
// Find profiles with poorly formatted IDs
Profile: $name where {
    // ID has spaces or special characters
    $name <: r"[^a-zA-Z0-9-]"
}
```

### Nested Patterns

```gritql
// ValueSets without proper code system references
ValueSet: $name where {
    and {
        contains "* include codes from system",
        not contains "http://"  // Missing proper URL
    }
}
```

## Best Practices

### 1. Write Clear Documentation

Every rule should explain:
- What it checks
- Why it matters
- How to fix violations

### 2. Provide Examples

Show both violations and correct code:

```markdown
## Examples

❌ **Bad**:
```fsh
Profile: myProfile  // lowercase
```

✅ **Good**:
```fsh
Profile: MyProfile  // PascalCase
```
```

### 3. Use Descriptive Names

Rule file names should be:
- **kebab-case**: `profile-naming-convention.grit`
- **Descriptive**: Name indicates what it checks
- **Specific**: Not too broad or vague

Good names:
- `extension-url-required.grit`
- `profile-pascal-case.grit`
- `valueset-missing-description.grit`

Bad names:
- `rule1.grit`
- `check.grit`
- `validation.grit`

### 4. Choose Appropriate Severity

- **Error**: Critical issues that break functionality
- **Warning**: Potential problems or style violations
- **Info**: Suggestions or documentation reminders

### 5. Test Thoroughly

Test your rules against:
- Valid FSH files (should not trigger)
- Invalid FSH files (should trigger)
- Edge cases (unusual but valid syntax)

### 6. Consider Performance

- Keep patterns simple when possible
- Avoid overly complex regex
- Test on large files

## Examples Library

### Style Rules

```gritql
// Profile names should use PascalCase
Profile: $name where {
    $name <: r"^[a-z]"
}
```

### Correctness Rules

```gritql
// Extensions must have context
Extension: $name where {
    not contains "^context"
}
```

### Documentation Rules

```gritql
// Profiles should have descriptions
Profile: $name where {
    not contains "Description:"
}
```

### Project-Specific Rules

```gritql
// Profiles must follow org naming convention
Profile: $name where {
    not {
        $name <: r"^(US|CA|UK)"
    }
}
```

## Troubleshooting

### Rule Not Loading

1. Check file extension is `.grit`
2. Verify `ruleDirectories` in config
3. Run with `-vv` to see loading messages
4. Check for syntax errors in pattern

### Rule Not Triggering

1. Verify pattern matches your FSH syntax
2. Test pattern on simple example
3. Check regex is correct
4. Use verbose mode to see rule execution

### False Positives

1. Make pattern more specific
2. Add additional conditions
3. Use `and` to narrow matches
4. Test against valid files

## Additional Resources

- [GritQL Documentation](https://docs.grit.io/)
- [FSH Specification](https://hl7.org/fhir/uv/shorthand/)
- [Example Rules](/examples/gritql/)
- [maki Configuration Guide](/docs/configuration.md)

## Contributing

Have you written a useful rule? Consider contributing it back!

1. Add comprehensive documentation
2. Include test cases
3. Submit a pull request
4. Help others learn from your work

---

**Need Help?** Open an issue on [GitHub](https://github.com/octofhir/maki-rs/issues)
