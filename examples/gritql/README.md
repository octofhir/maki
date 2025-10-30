# Custom GritQL Rules for FSH Lint

This directory contains example custom GritQL rules that can be used with `maki` to enforce project-specific conventions and requirements.

## Using Custom Rules

To use these rules (or your own custom rules), add the directory to your `.makirc.json`:

```json
{
  "linter": {
    "enabled": true,
    "ruleDirectories": ["examples/gritql"],
    "rules": {
      "recommended": true
    }
  }
}
```

## Example Rules

### 1. `profile-naming-uppercase.grit`

**Severity**: Warning  
**Category**: Style

Enforces that Profile names follow PascalCase convention (must start with uppercase letter).

### 2. `extension-url-required.grit`

**Severity**: Error  
**Category**: Correctness

Ensures all Extension definitions include a canonical URL (`^url` assignment).

## Writing Your Own GritQL Rules

GritQL is a pattern matching language for code. Here's the basic structure of a `.grit` file:

```gritql
// Comments describing what the rule does

ResourceType: $name where {
    // Conditions that must match
    condition1,
    condition2
}
```

### Common Patterns

#### Match by Regex
```gritql
Profile: $name where {
    $name <: r"^[a-z]"  // Name starts with lowercase
}
```

#### Check for Missing Fields
```gritql
Extension: $name where {
    not contains "^url"  // Missing URL assignment
}
```

#### Combine Multiple Conditions
```gritql
ValueSet: $name where {
    and {
        $name <: r"^VS",  // Name starts with "VS"
        not contains "^status"  // Missing status
    }
}
```

### Available Operators

- `<:` - Regex match
- `contains` - Check if text contains substring
- `not` - Negate condition
- `and` / `or` - Combine conditions
- `where` - Filter matches

### FSH-Specific Patterns

Match different FSH resource types:
- `Profile: $name where { ... }`
- `Extension: $name where { ... }`
- `ValueSet: $name where { ... }`
- `CodeSystem: $name where { ... }`
- `Instance: $name where { ... }`

## Rule File Format

Each `.grit` file should:

1. **Start with comments** explaining the rule
2. **Include metadata** (Category, Severity)
3. **Provide examples** of violations and fixes
4. **Explain the pattern** for learning purposes

## Testing Your Rules

Test your custom rules:

```bash
# Lint with custom rules
maki lint --config .makirc.json your-file.fsh

# List all loaded rules (including custom)
maki rules --detailed

# Check specific custom rule
maki rules --category custom
```

## Rule Naming Convention

Custom rule files should:
- Use kebab-case: `profile-naming-uppercase.grit`
- Be descriptive: name should indicate what it checks
- Match the rule ID (becomes `gritql/profile-naming-uppercase`)

## Additional Resources

- [GritQL Documentation](https://docs.grit.io/)
- [FSH Specification](https://hl7.org/fhir/uv/shorthand/)
- [maki Documentation](https://octofhir.github.io/maki/)

## Contributing

Found a useful pattern? Consider contributing it back to the project!
