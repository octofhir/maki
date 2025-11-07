---
title: Writing Custom Rules
description: Create custom lint rules for your FSH project
---

Learn how to write custom lint rules using GritQL for project-specific validation.

## Getting Started

### Prerequisites

- Understanding of FSH syntax
- Basic pattern matching concepts
- GritQL syntax basics (see [GritQL Rules](/configuration/gritql/))

### Setup

Create a directory for custom rules:

```bash
mkdir custom-rules
```

Configure FSH Lint to load custom rules:

```jsonc
{
  "linter": {
    "ruleDirectories": ["./custom-rules"]
  }
}
```

## Rule Structure

A GritQL rule file (`.grit`) contains:

1. Metadata (language, description, severity)
2. Pattern to match
3. Conditions (optional)
4. Message
5. Fix (optional)

## Example: Enforce Profile Naming

Create `custom-rules/profile-naming.grit`:

```gritql
language fsh
description "Profiles must end with 'Profile'"
severity error

pattern {
  Profile: $name
}

where {
  !ends_with($name, "Profile")
}

message "Profile name '${name}' should end with 'Profile'"

fix {
  Profile: `${name}Profile`
}
```

## Example: Require MS Flags

Create `custom-rules/require-ms.grit`:

```gritql
language fsh
description "Required fields must have MS flag"
severity warning

pattern {
  Profile: $_
  * $path 1..1 $flags
}

where {
  !contains($flags, "MS")
}

message "Add MS flag to required field: ${path}"

fix {
  * $path 1..1 MS
}
```

## Example: Enforce Descriptions

Create `custom-rules/require-description.grit`:

```gritql
language fsh
description "All profiles must have descriptions"
severity warning

pattern {
  Profile: $name
  $...content
}

where {
  !any_match($content, "Description:")
}

message "Profile '${name}' is missing a description"
```

## Testing Custom Rules

Test your rules before deploying:

```bash
# Test on specific files
maki lint --rule custom/profile-naming test.fsh

# Run only custom rules
maki lint --only-custom **/*.fsh
```

## Using Built-in Functions

MAKI provides 12 built-in functions for powerful pattern matching. Here are practical examples:

### Example: Enforce PascalCase Profile Names

```gritql
language fsh
description "Profile names must use PascalCase"
severity error

pattern {
  Profile: $name
}

where {
  not is_pascal_case($name)
}

message "Profile name '${name}' should use PascalCase (e.g., MyProfile)"
```

### Example: Enforce kebab-case ValueSet IDs

```gritql
language fsh
description "ValueSet IDs must use kebab-case"
severity error

pattern {
  ValueSet: $vs_name where { id }
}

where {
  not is_kebab_case($id)
}

message "ValueSet ID should use kebab-case (e.g., my-value-set)"
```

### Example: Require Complete Documentation

```gritql
language fsh
description "Profiles must have title, description, and parent"
severity warning

pattern {
  Profile: $name where {
    not (title and description and parent)
  }
}

message "Profile '${name}' is missing required documentation (title, description, parent)"
```

### Example: Check Profile Properties

```gritql
language fsh
description "Profiles must inherit from Patient"
severity error

pattern {
  Profile: $name where {
    parent != "Patient" and
    parent != "DomainResource"
  }
}

message "Profile '${name}' should inherit from Patient or DomainResource"
```

### Example: Validate Extension Documentation

```gritql
language fsh
description "Extensions must have title, description, and URL"
severity warning

pattern {
  Extension: $ext where {
    not (has_title($e) and has_description($e) and url)
  }
}

message "Extension '${ext}' is missing documentation"
```

### Example: Find Undocumented Elements

```gritql
language fsh
description "All definitions should have descriptions"
severity info

pattern {
  (Profile or Extension or ValueSet or CodeSystem): $name where {
    not has_description($d)
  }
}

message "Element '${name}' should include a description"
```

## Advanced Patterns

### Complex Conditions with Built-ins

```gritql
language fsh
description "Profile naming and documentation validation"
severity error

pattern {
  Profile: $name where {
    is_pascal_case($name) and
    title and
    description and
    parent
  }
}

message "Profile '${name}' meets all requirements"
```

### Multiple Type Validation

```gritql
language fsh
description "Validate naming across definition types"
severity warning

pattern {
  or {
    { Profile: $p where { is_pascal_case($p) } }
    { Extension: $e where { is_pascal_case($e) } }
    { ValueSet: $vs where { not is_kebab_case($vs) } }
  }
}

message "Definition naming convention mismatch"
```

### Composite Rules

```gritql
language fsh
description "Complete validation of profiles"
severity error

pattern {
  Profile: $name where {
    is_pascal_case($name) and
    has_title($p) and
    has_description($p) and
    has_parent($p) and
    has_comment($p)
  }
}

message "Profile '${name}' is well-documented and follows conventions"
```

## Best Practices

1. **Start Simple** - Begin with basic patterns
2. **Test Thoroughly** - Test on various FSH files
3. **Clear Messages** - Help users understand violations
4. **Provide Fixes** - Automate fixes when possible
5. **Document Rules** - Explain the reasoning
6. **Performance** - Avoid overly complex patterns

## Organization-Wide Rules

Share rules across projects:

```bash
# Create shared rules repository
git clone https://github.com/yourorg/maki-rules.git

# Reference in config
{
  "linter": {
    "ruleDirectories": [
      "./maki-rules"
    ]
  }
}
```

## Quick Reference: Built-in Functions

MAKI provides 12 specialized functions for FSH validation:

| Category | Functions |
|----------|-----------|
| **Node Type Checks** | `is_profile()`, `is_extension()`, `is_value_set()`, `is_code_system()` |
| **Node Properties** | `has_comment()`, `has_title()`, `has_description()`, `has_parent()` |
| **String Validation** | `is_kebab_case()`, `is_pascal_case()`, `is_camel_case()`, `is_screaming_snake_case()` |

See [GritQL Rules](/configuration/gritql/) for complete documentation of all built-in functions.

## See Also

- [GritQL Rules](/configuration/gritql/) - Complete built-in functions reference
- [GritQL Documentation](https://docs.grit.io/)
- [Built-in Rules](/rules/) - Examples
