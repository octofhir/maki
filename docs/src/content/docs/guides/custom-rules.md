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
fsh-lint lint --rule custom/profile-naming test.fsh

# Run only custom rules
fsh-lint lint --only-custom **/*.fsh
```

## Advanced Patterns

### Complex Conditions

```gritql
pattern {
  Profile: $name
  * $path from $vs (required)
}

where {
  ends_with($path, "code") and
  !is_uri($vs)
}

message "Code bindings should use full URI: ${vs}"
```

### Multiple Patterns

```gritql
pattern {
  or {
    { ValueSet: $name }
    { CodeSystem: $name }
  }
}

where {
  !contains($name, "VS") and
  !contains($name, "CS")
}
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
git clone https://github.com/yourorg/fsh-lint-rules.git

# Reference in config
{
  "linter": {
    "ruleDirectories": [
      "./fsh-lint-rules"
    ]
  }
}
```

## See Also

- [GritQL Documentation](https://docs.grit.io/)
- [GritQL Rules](/configuration/gritql/)
- [Built-in Rules](/rules/) - Examples
