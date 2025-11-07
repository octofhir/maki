# GritQL Pattern Reference

Complete reference documentation for GritQL pattern syntax supported in MAKI.

## Table of Contents

1. [Pattern Types](#pattern-types)
2. [Predicates and Conditions](#predicates-and-conditions)
3. [Built-in Functions](#built-in-functions)
4. [Variable Binding](#variable-binding)
5. [Field Access](#field-access)
6. [Examples](#examples)

## Pattern Types

### Node Type Patterns

Match definitions by their type in FSH:

```gritql
Profile          // Match Profile definitions
Extension        // Match Extension definitions
ValueSet         // Match ValueSet definitions
CodeSystem       // Match CodeSystem definitions
Instance         // Match Instance definitions
Invariant        // Match Invariant definitions
Mapping          // Match Mapping definitions
Logical          // Match Logical definitions
Resource         // Match Resource definitions
Alias            // Match Alias definitions
```

## Predicates and Conditions

### Basic Syntax

```gritql
// Simple condition - check if field exists
Profile where { title }

// Negated condition - check if field does NOT exist
Profile where { not title }

// Multiple conditions with AND (all must be true)
Profile where {
  title and
  description and
  parent
}

// Multiple conditions with OR (any must be true)
Profile where {
  title or
  description or
  parent
}

// Complex combinations
Profile where {
  (title and description) or parent
}
```

### Field Existence Checks

Check if a field is present in the node:

```gritql
Profile where { title }           // Has Title field
Profile where { description }     // Has Description field
Profile where { parent }          // Has Parent field
Profile where { id }              // Has Id field
Profile where { url }             // Has Url field
```

### Negation

Use `not` to invert any condition:

```gritql
Profile where { not title }                    // Missing title
Profile where { not title and not description } // Missing both
Profile where { not (title and description) }  // Missing at least one
```

### Logical Operators

#### AND (all conditions must be true)

```gritql
Profile where {
  title and
  description
}

// All three must be true
Extension where {
  title and
  description and
  url
}
```

#### OR (any condition can be true)

```gritql
Profile where {
  title or
  description
}

// At least one must be true
ValueSet where {
  title or
  description or
  url
}
```

### String Operations

Compare string values with operators:

```gritql
// Equality check
Profile where {
  id == "my-profile"
}

// Inequality check
Profile where {
  id != "reserved-id"
}

// Contains substring
Profile where {
  title contains "Patient"
}

// Starts with prefix
Profile where {
  id startsWith "my-"
}

// Ends with suffix
Profile where {
  name endsWith "Profile"
}

// Regex match (ECMAScript regex syntax)
Profile where {
  id <: r"^[a-z]([a-z0-9\-]*[a-z0-9])?$"
}
```

## Built-in Functions

### Node Type Checking Functions

These functions check the type of a node:

```gritql
is_profile(node)       // Returns true if node is a Profile
is_extension(node)     // Returns true if node is an Extension
is_value_set(node)     // Returns true if node is a ValueSet
is_code_system(node)   // Returns true if node is a CodeSystem
```

**Usage in patterns**:
```gritql
Profile where { is_profile($profile) }
```

### Node Property Functions

Check if a node has specific fields or properties:

```gritql
has_comment(node)      // Node has comments
has_title(node)        // Node has Title field
has_description(node)  // Node has Description field
has_parent(node)       // Node has Parent field (Profiles)
```

**Usage**:
```gritql
Profile where { has_title($p) and has_description($p) }
Extension where { not has_comment($e) }
```

### String Validation Functions

Validate if a string follows a specific naming convention:

```gritql
is_kebab_case(text)           // lowercase-with-dashes
is_pascal_case(text)          // PascalCase
is_camel_case(text)           // camelCase
is_screaming_snake_case(text) // SCREAMING_SNAKE_CASE
```

**Patterns**:
- `kebab-case`: `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`
- `PascalCase`: `^[A-Z][a-zA-Z0-9]*$`
- `camelCase`: `^[a-z][a-zA-Z0-9]*$`
- `SCREAMING_SNAKE_CASE`: `^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$`

**Usage**:
```gritql
Profile: $name where {
  is_pascal_case($name)
}

ValueSet where {
  is_kebab_case($id)
}
```

## Variable Binding

### Basic Variable Capture

Capture values from matched nodes using the `$` prefix:

```gritql
// Capture profile name
Profile: $name

// Capture extension name
Extension: $ext

// Capture value set name
ValueSet: $vs
```

### Variable Types

Variables can capture:
- Node names/identifiers from definitions
- Field values from `where` clauses
- String values from predicates

### Using Variables in Predicates

Variables can be used in condition predicates:

```gritql
// Validate captured name format
Profile: $name where {
  is_pascal_case($name)
}

// Check captured value against string
Extension: $ext where {
  $ext != "reserved"
}

// Use variable in contains check
Profile: $name where {
  $name contains "Patient"
}

// Regex match on variable
ValueSet: $vs where {
  $vs <: r"^[a-z]"
}
```

### Multiple Variables

Capture multiple values from a single pattern:

```gritql
Profile: $name where {
  parent == "Patient" and
  is_pascal_case($name)
}
```

## Field Access

### Accessing Node Fields

Access specific fields from matched nodes using dot notation:

```gritql
Profile where {
  name == "MyProfile"
}

Extension where {
  url == "http://example.com/extension"
}

ValueSet where {
  title == "My Value Set"
}
```

### Available Fields by Type

#### Profile Fields
- `name` - Profile name/identifier
- `parent` - Parent profile reference
- `id` - Profile ID
- `title` - Profile title
- `description` - Profile description
- `url` - Profile URL (if defined)

#### Extension Fields
- `name` - Extension name/identifier
- `url` - Extension URL
- `title` - Extension title
- `description` - Extension description

#### ValueSet Fields
- `name` - ValueSet name/identifier
- `id` - ValueSet ID
- `title` - ValueSet title
- `description` - ValueSet description

#### CodeSystem Fields
- `name` - CodeSystem name/identifier
- `id` - CodeSystem ID
- `title` - CodeSystem title
- `description` - CodeSystem description

#### Instance Fields
- `name` - Instance name/identifier
- `instanceOf` - Instance type reference

### Field Access Syntax

```gritql
// Check field directly
Profile where {
  title
}

// Compare field value
Profile where {
  title == "Patient Profile"
}

// Field with string operations
Extension where {
  url contains "hl7.org"
}

// Field with regex
Profile where {
  id <: r"^[a-z]"
}
```

## Examples

### Complete Pattern Examples

```gritql
// Example 1: Find profiles missing documentation
Profile where {
  not title or not description
}

// Example 2: Find extensions with proper naming
Extension: $ext where {
  is_pascal_case($ext) and
  title and
  description
}

// Example 3: Find value sets without titles
ValueSet where {
  not title
}

// Example 4: Validate profile parent
Profile where {
  parent == "Patient"
}

// Example 5: Complex validation
Profile: $name where {
  is_pascal_case($name) and
  title and
  description and
  parent
}

// Example 6: Regex validation on ID
Profile where {
  id <: r"^[a-z][a-z0-9\-]*$"
}

// Example 7: Multiple conditions
Extension where {
  (title and description) or has_comment($e)
}
```

### Real-world Rules

```gritql
// Require all profiles have documentation
Profile where {
  not (title and description)
}

// Enforce naming standards
Profile: $name where {
  not is_pascal_case($name)
}

// Find incomplete extensions
Extension where {
  not url or
  not title
}

// Validate value set completeness
ValueSet where {
  (title or url) and not description
}
```

## Error Messages

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `Expected identifier` | Missing variable/field name | Add identifier after `$` or field name |
| `Expected '}'` | Unbalanced braces | Ensure `where { ... }` is properly closed |
| `Unterminated string` | Missing closing quote | Add `"` to close string literals |
| `Expected word` | Operator not recognized | Use: `and`, `or`, `not`, `<:`, `contains`, `==`, `!=`, `startsWith`, `endsWith` |

### Debugging Patterns

Test your patterns with the `maki lint` command:

```bash
# Test a specific pattern
maki lint --rule-pattern 'Profile where { title }' examples/

# Test with verbose output
maki lint --verbose --rule-pattern 'Profile' examples/
```

## Operator Precedence

Operators are evaluated with the following precedence (highest to lowest):

1. Field access (`.`)
2. Function calls
3. `not` (negation)
4. `<:`, `contains`, `startsWith`, `endsWith`, `==`, `!=` (string operations)
5. `and`
6. `or`

Use parentheses to override precedence:

```gritql
// Without parentheses - and has higher precedence than or
Profile where { a or b and c }  // Equivalent to: (a) or (b and c)

// With parentheses - explicit precedence
Profile where { (a or b) and c }
```

## Performance Considerations

- Patterns are compiled once and reused for all matches
- Variable capture is efficient (text-based extraction)
- Regex patterns are compiled at pattern compile time
- Function calls have minimal overhead

Optimize patterns by:
1. Use specific conditions rather than broad matches
2. Place most restrictive conditions first
3. Use `not` on rare conditions (matching fewer nodes)

Example:

```gritql
// Efficient - specific condition first
Profile where {
  is_pascal_case($name) and
  title
}

// Less efficient - checks all profiles
Profile where {
  title and
  is_pascal_case($name)
}
```

## Versioning

This reference documents GritQL support in MAKI version 0.0.2+.

Future versions may add:
- Custom function definitions
- Variable assignment and binding
- Field modification capabilities
- Integration with FHIR specification constraints
