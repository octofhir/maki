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

Configure custom rule directories in `maki.json`:

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

### Variable Binding with Nested Where Clauses

**New in Phase 3**: You can now use nested where clauses with variable constraints to create more powerful rules:

```gritql
// Match profiles where the name starts with uppercase
profile_declaration: $name where {
    $name <: r"^[A-Z]"
}
```

This syntax allows you to:
1. **Bind a variable** to a field value (e.g., `$name` captures the profile name)
2. **Apply constraints** to that variable using nested predicates
3. **Use the captured value** in messages and fixes

#### Examples

**Enforce naming conventions**:
```gritql
// Profile names must start with uppercase
profile_declaration: $name where {
    $name <: r"^[A-Z]"
}
```

**Combine with logical operators**:
```gritql
// Profile names must be PascalCase (uppercase start, no underscores/hyphens)
profile_declaration: $name where {
    and {
        $name <: r"^[A-Z]",
        not $name <: r"[_-]"
    }
}
```

**Check multiple variables**:
```gritql
// Both profile name and ID must follow naming rules
Profile: $name where {
    and {
        $name <: r"^[A-Z][a-zA-Z0-9]*$",
        $parent == "Patient"
    }
}
```

#### Syntax Requirements

⚠️ **Important**: Nested where clauses require braces `{ }`:

```gritql
// ✅ Correct - braces after 'where'
$name where { $name <: r"^[A-Z]" }

// ❌ Wrong - missing braces
$name where $name <: r"^[A-Z]"
```

This distinguishes variable patterns from regular predicates:
- `$var where { predicate }` - Variable pattern with constraints
- `where $var == value` - Regular predicate

See the [GritQL Getting Started Guide](/guides/gritql/getting-started/) for more examples.

## Testing Custom Rules

Test your rules before deploying:

```bash
# Run only custom rules
maki lint --only-custom-rules **/*.fsh

# Test specific rule
maki lint --rule custom/require-ms-flag test.fsh
```

## Best Practices

1. **Start Simple** - Begin with basic pattern matching
2. **Test Thoroughly** - Test on various FSH files
3. **Provide Clear Messages** - Help users understand the issue
4. **Include Fixes** - Provide automated fixes when possible
5. **Document Rules** - Explain why the rule exists

## Built-in Functions

MAKI provides 12 specialized built-in functions for FSH pattern matching:

### Node Type Checking Functions

Check the type of a matched node:

```gritql
is_profile($node)       // Returns true if node is a Profile
is_extension($node)     // Returns true if node is an Extension
is_value_set($node)     // Returns true if node is a ValueSet
is_code_system($node)   // Returns true if node is a CodeSystem
```

**Example:**
```gritql
Profile: $name where { is_profile($name) }
```

### Node Property Functions

Check if a node has specific fields or properties:

```gritql
has_comment($node)      // Node has comments
has_title($node)        // Node has Title field
has_description($node)  // Node has Description field
has_parent($node)       // Node has Parent field (Profiles only)
```

**Example:**
```gritql
Profile where { has_title($p) and has_description($p) }
Extension where { not has_comment($e) }
```

### String Validation Functions

Validate if a string follows a specific naming convention:

```gritql
is_kebab_case($text)           // lowercase-with-dashes
is_pascal_case($text)          // PascalCase
is_camel_case($text)           // camelCase
is_screaming_snake_case($text) // SCREAMING_SNAKE_CASE
```

**Patterns:**
- **kebab-case**: `my-profile`, `patient-id`, `value-set-name`
- **PascalCase**: `MyProfile`, `PatientRecord`, `ValueSetName`
- **camelCase**: `myProfile`, `patientRecord`, `valueSetName`
- **SCREAMING_SNAKE_CASE**: `MY_PROFILE`, `PATIENT_ID`, `VALUE_SET_NAME`

**Example:**
```gritql
// Enforce PascalCase naming for profiles
Profile: $name where {
  is_pascal_case($name)
}

// Enforce kebab-case IDs
ValueSet where {
  is_kebab_case($id)
}
```

## Field Conditions

Check for field existence and compare field values:

```gritql
// Check if field exists
Profile where { title }

// Check if field doesn't exist
Profile where { not title }

// Compare field values
Profile where { parent == "Patient" }

// String operations on fields
Extension where { url contains "hl7.org" }

// Multiple conditions with AND
Profile where {
  title and
  description and
  parent
}

// Multiple conditions with OR
Profile where {
  not title or
  not description
}
```

## Common Built-in Function Examples

### Enforce Documentation Standards

```gritql
// Find profiles without titles
Profile where { not title }

// Find profiles missing descriptions
Profile where { not description }

// Find profiles with complete documentation
Profile where {
  title and
  description and
  parent
}
```

### Enforce Naming Conventions

```gritql
// Validate profile names use PascalCase
Profile: $name where {
  is_pascal_case($name)
}

// Find IDs that don't use kebab-case
ValueSet where {
  not is_kebab_case($id)
}

// Enforce consistent extension naming
Extension: $ext where {
  is_pascal_case($ext) and
  title and
  description
}
```

### Enforce Metadata Requirements

```gritql
// Extensions must have titles and descriptions
Extension where {
  has_title($e) and
  has_description($e)
}

// Profiles with comments
Profile where {
  has_comment($p)
}

// ValueSets with parent relationships
ValueSet where {
  has_parent($vs)
}
```

## See Also

- [GritQL Documentation](https://docs.grit.io/) - Complete GritQL reference
- [Built-in Rules](/rules/) - Examples of rule implementation
- [Custom Rules Guide](/guides/custom-rules/) - Detailed guide
