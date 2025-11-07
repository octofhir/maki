# GritQL Pattern Cookbook for FSH Linting

This cookbook provides practical examples of GritQL patterns for writing FSH linting rules without needing Rust knowledge.

## Quick Start

GritQL patterns are used in `.maki` configuration files and rule definitions. Basic syntax:

```gritql
// Match any Profile
Profile

// Match Profile with variable capture
Profile: $name

// Match with conditions (predicates)
Profile where { description }

// Match with complex conditions
Profile where {
  title and
  parent == "Patient"
}
```

## Pattern Basics

### 1. Simple Node Matching

Match any definition of a specific type:

```gritql
// Find all profiles
Profile

// Find all extensions
Extension

// Find all value sets
ValueSet

// Find all code systems
CodeSystem
```

### 2. Variable Capture

Capture the name or other properties:

```gritql
// Capture profile name
Profile: $name

// Capture extension name and URL
Extension: $ext where { url }

// Capture value set with title
ValueSet: $vs where { title }
```

### 3. Using Built-in Functions

Built-in functions allow you to check node properties and validate naming conventions:

#### Node Type Checks

```gritql
// These are implicitly checked by the pattern type, but can be used with variables:
is_profile($node)      // Check if node is Profile
is_extension($node)    // Check if node is Extension
is_value_set($node)    // Check if node is ValueSet
is_code_system($node)  // Check if node is CodeSystem
```

#### Node Property Checks

```gritql
// Check if a node has specific fields defined
has_title($profile)              // Profile has Title field
has_description($extension)      // Extension has Description field
has_parent($profile)             // Profile has Parent field
has_comment($valueset)           // ValueSet has comments
```

#### String Validation

```gritql
// Validate naming conventions on captured values
is_kebab_case($name)             // lowercase-with-dashes
is_pascal_case($name)            // PascalCase
is_camel_case($name)             // camelCase
is_screaming_snake_case($name)   // SCREAMING_SNAKE_CASE
```

## Common Rules

### Documentation Rules

#### 1. Require Titles on Profiles

Find profiles without titles:

```gritql
Profile where { not title }
```

This will match profiles missing the `Title:` field.

#### 2. Require Descriptions on Profiles

Find profiles lacking descriptions:

```gritql
Profile where { not description }
```

#### 3. Require Both Title and Description

```gritql
Profile where { not title and not description }
```

Or more explicitly:

```gritql
Profile where {
  not has_title($p) or
  not has_description($p)
}
```

### Naming Convention Rules

#### 1. Validate Profile Names (PascalCase)

```gritql
Profile: $name where {
  is_pascal_case($name)
}
```

This will match only profiles where the name follows PascalCase.

#### 2. Validate Profile IDs (kebab-case)

```gritql
Profile where {
  is_kebab_case($id)
}
```

#### 3. Enforce Consistent Extension Naming

```gritql
Extension: $ext where {
  is_pascal_case($ext)
}
```

### Profile Rules

#### 1. Find Profiles Without Parents

```gritql
Profile where { not parent }
```

Find profiles that don't explicitly inherit from another profile.

#### 2. Find Profiles Extending Patient

```gritql
Profile where {
  parent == "Patient"
}
```

#### 3. Find Profiles with Comments

```gritql
Profile where {
  has_comment($p)
}
```

### Value Set Rules

#### 1. ValueSets Missing Titles

```gritql
ValueSet where { not title }
```

#### 2. ValueSets Missing Descriptions

```gritql
ValueSet where { not description }
```

#### 3. ValueSets with Both Title and Description

```gritql
ValueSet where {
  title and description
}
```

### Extension Rules

#### 1. Extensions Without URLs

```gritql
Extension where { not url }
```

#### 2. Extensions Lacking Descriptions

```gritql
Extension where { not description }
```

#### 3. Extensions with Titles

```gritql
Extension where { title }
```

## Advanced Patterns

### Combining Conditions (AND)

Use `and` to require multiple conditions:

```gritql
Profile where {
  title and
  description and
  parent
}
```

This matches profiles that have ALL three: title, description, and parent.

### Combining Conditions (OR)

Use `or` to match if ANY condition is true:

```gritql
Profile where {
  not title or
  not description
}
```

This matches profiles missing EITHER title OR description (or both).

### Negation

Use `not` to invert conditions:

```gritql
// Profiles without comments
Profile where { not has_comment($p) }

// Profiles NOT in PascalCase
Profile: $name where { not is_pascal_case($name) }

// Profiles without BOTH title and description
Profile where {
  not (title and description)
}
```

### Complex Logical Expressions

```gritql
// Profiles that need documentation (missing description AND missing title)
Profile where {
  (not title and not description) or
  (title and not description)
}

// Extensions with proper naming and documentation
Extension: $ext where {
  is_pascal_case($ext) and
  description and
  title
}

// ValueSets that follow ALL best practices
ValueSet: $vs where {
  is_kebab_case($vs) and
  title and
  description and
  has_comment($vs)
}
```

## Naming Convention Details

### Kebab-Case (`is_kebab_case`)

Lowercase letters and numbers separated by dashes:

- ✅ Valid: `my-profile`, `patient-id`, `value-set-name`
- ❌ Invalid: `MyProfile`, `my_profile`, `myProfile`

Used for: IDs in FSH

### PascalCase (`is_pascal_case`)

Starts with uppercase, no separators, mixed case:

- ✅ Valid: `MyProfile`, `PatientRecord`, `ValueSetName`
- ❌ Invalid: `myProfile`, `my-profile`, `my_profile`

Used for: Names in FSH (profiles, extensions, etc.)

### camelCase (`is_camel_case`)

Starts with lowercase, no separators, mixed case:

- ✅ Valid: `myProfile`, `patientRecord`, `valueSetName`
- ❌ Invalid: `MyProfile`, `my-profile`, `my_profile`

Used for: Variable/property names (less common in FSH)

### SCREAMING_SNAKE_CASE (`is_screaming_snake_case`)

All uppercase with underscores:

- ✅ Valid: `MY_PROFILE`, `PATIENT_ID`, `VALUE_SET_NAME`
- ❌ Invalid: `MyProfile`, `my_profile`, `myProfile`

Used for: Constants or identifiers (uncommon in FSH)

## Troubleshooting

### Pattern Not Matching

**Problem**: Your pattern isn't matching anything
- Ensure you're using correct node types: `Profile`, `Extension`, `ValueSet`, `CodeSystem`
- Check that boolean conditions use correct syntax: `field` (exists), `not field`, `and`, `or`
- Remember that field names are case-sensitive

**Solution**:
```gritql
// ✅ Correct
Profile where { title }

// ❌ Wrong - no braces
Profile where title

// ❌ Wrong - incorrect field name
Profile where { Title }
```

### Complex Conditions

**Problem**: Multiple conditions aren't working as expected
- Remember: `and` requires ALL conditions true
- Remember: `or` requires ANY condition true
- Use parentheses for clarity: `(a and b) or (c and d)`

**Example**:
```gritql
// Profiles missing documentation
Profile where {
  not title or not description
}

// Profiles with naming issues
Profile: $name where {
  not is_pascal_case($name) and
  not is_kebab_case($name)
}
```

## Best Practices

1. **Be Specific**: Use precise conditions to avoid false positives
   ```gritql
   // Good - specific condition
   Profile where { title and description and parent }

   // Bad - too general
   Profile
   ```

2. **Use Built-ins**: Leverage string validation functions
   ```gritql
   // Good - validates naming
   Profile: $name where { is_pascal_case($name) }

   // Bad - manual text check
   Profile: $name where { name }
   ```

3. **Clear Logic**: Use parentheses to clarify complex expressions
   ```gritql
   // Good - clear intent
   Extension where {
     (title and description) or
     (has_comment($e) and description)
   }

   // Bad - ambiguous
   Extension where {
     title and description or has_comment($e) and description
   }
   ```

4. **Test Your Patterns**: Verify patterns match expected files
   ```bash
   # Test pattern against FSH file
   maki lint --rule-pattern 'Profile where { title }' examples/
   ```

## Examples by Use Case

### Enforce Metadata Standards

```gritql
// All profiles must have title, description, and parent
Profile where {
  title and
  description and
  parent
}
```

### Enforce Naming Conventions

```gritql
// All profiles must use PascalCase names
Profile: $name where {
  is_pascal_case($name)
}

// All value sets must use kebab-case IDs
ValueSet where {
  is_kebab_case($id)
}
```

### Find Documentation Gaps

```gritql
// Profiles missing any documentation
Profile where {
  not title or
  not description
}
```

### Find Undocumented Elements

```gritql
// Profiles with no comments
Profile where {
  not has_comment($p)
}

// Extensions without descriptions
Extension where {
  not description
}
```

## Reference

See [REFERENCE.md](./REFERENCE.md) for complete syntax documentation and all available built-in functions.
