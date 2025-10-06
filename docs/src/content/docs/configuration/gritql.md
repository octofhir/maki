---
title: GritQL Custom Rules
description: Write custom lint rules using GritQL
---

GritQL is a powerful pattern-matching language that allows you to write custom lint rules for your FSH project.

## What is GritQL?

GritQL is a query language designed for code analysis and transformation. It lets you:

- Match code patterns using intuitive syntax
- Capture matched values in variables
- Apply complex predicates and conditions
- Generate diagnostic messages and fixes

## Basic Syntax

### Simple Pattern Matching

Match a profile definition:

```gritql
Profile: $name
```

This matches any profile and captures its name in `$name`.

### Field Matching

Match profiles with specific properties:

```gritql
Profile: $name
Parent: Patient
```

### Pattern Variables

Variables capture matched content:

- `$name` - Captures an identifier
- `$_` - Matches anything (anonymous)
- `$...rest` - Captures remaining items

## Writing Custom Rules

### Rule File Structure

Create `.grit` files in your custom rules directory:

```gritql
// custom-rules/require-ms-flag.grit

// Rule metadata
language fsh
description "Profile constraints should have MS flag"
severity warning

// Pattern to match
pattern {
  Profile: $profile_name
  * $path $card
}

// Condition
where {
  // Check if MS flag is missing
  !contains($card, "MS")
}

// Message
message "Add MS (Must Support) flag to constraint"
```

### Loading Custom Rules

Configure custom rule directories in `fsh-lint.json`:

```jsonc
{
  "linter": {
    "ruleDirectories": [
      "./custom-rules",
      "./org-rules"
    ]
  }
}
```

## Example Rules

### Require Must Support Flags

```gritql
language fsh
description "Profile constraints should use MS flag"
severity warning

pattern {
  Profile: $_
  * $path 1..1
}

where {
  !contains($path, "MS")
}

message "Add MS flag to required elements"
```

### Enforce Naming Patterns

```gritql
language fsh
description "ValueSets must end with 'VS'"
severity error

pattern {
  ValueSet: $name
}

where {
  !ends_with($name, "VS")
}

message "ValueSet names should end with 'VS'"
fix {
  ValueSet: `${name}VS`
}
```

### Detect Missing Descriptions

```gritql
language fsh
description "Profiles must have descriptions"
severity warning

pattern {
  Profile: $name
  $...body
}

where {
  !contains($body, "Description:")
}

message "Profile '${name}' is missing a description"
```

## Advanced Patterns

### Using Wildcards

```gritql
// Match any resource type
$resource_type: $name
where {
  $resource_type in ["Profile", "Extension", "ValueSet"]
}
```

### Nested Patterns

```gritql
Profile: $name
* $path from $valueset (required)
where {
  !exists($valueset)
}
message "ValueSet '${valueset}' not found"
```

### Multiple Conditions

```gritql
Profile: $name
* $path $card
where {
  $card == "1..1" and
  !contains($path, "MS") and
  !contains($path, "^")
}
```

## Testing Custom Rules

Test your rules before deploying:

```bash
# Run only custom rules
fsh-lint lint --only-custom-rules **/*.fsh

# Test specific rule
fsh-lint lint --rule custom/require-ms-flag test.fsh
```

## Best Practices

1. **Start Simple** - Begin with basic pattern matching
2. **Test Thoroughly** - Test on various FSH files
3. **Provide Clear Messages** - Help users understand the issue
4. **Include Fixes** - Provide automated fixes when possible
5. **Document Rules** - Explain why the rule exists

## See Also

- [GritQL Documentation](https://docs.grit.io/) - Complete GritQL reference
- [Built-in Rules](/rules/) - Examples of rule implementation
- [Custom Rules Guide](/guides/custom-rules/) - Detailed guide
