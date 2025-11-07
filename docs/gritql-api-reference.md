# GritQL API Reference

## Complete API Documentation for GritQL Code Rewriting

---

## Effect Module (`rewrite.rs`)

### `Effect` Enum

Represents a code transformation operation.

```rust
pub enum Effect {
    Replace {
        start_offset: usize,
        end_offset: usize,
        replacement: String,
    },
    Insert {
        position: usize,
        text: String,
    },
    Delete {
        start_offset: usize,
        end_offset: usize,
    },
    RewriteField {
        field_name: String,
        start_offset: usize,
        end_offset: usize,
        new_value: String,
    },
}
```

#### Variants

**Replace**
```rust
Effect::Replace {
    start_offset: 0,      // Byte offset where replacement starts
    end_offset: 15,       // Byte offset where replacement ends
    replacement: "new".to_string(),
}
```
Replaces the text between `start_offset` and `end_offset` with `replacement`.

**Insert**
```rust
Effect::Insert {
    position: 10,         // Byte position to insert at
    text: "inserted".to_string(),
}
```
Inserts `text` at `position`. Classified as unsafe (may change semantics).

**Delete**
```rust
Effect::Delete {
    start_offset: 5,      // Start of deletion range
    end_offset: 20,       // End of deletion range
}
```
Removes text between offsets. Classified as unsafe.

**RewriteField**
```rust
Effect::RewriteField {
    field_name: "id".to_string(),  // Field to modify
    start_offset: 5,                // Field value start
    end_offset: 15,                 // Field value end
    new_value: "new-value".to_string(),
}
```
Modifies a specific field value. Safety depends on field type (cosmetic fields are safe).

#### Methods

**`apply`**
```rust
pub fn apply(
    &self,
    source: &str,
    variables: &HashMap<String, String>,
    file_path: &str,
) -> Result<CodeSuggestion>
```
Applies the effect to source code and returns a CodeSuggestion.

**Parameters**:
- `source: &str` - The source code
- `variables: &HashMap<String, String>` - Captured variables for interpolation
- `file_path: &str` - Path for Location information

**Returns**: `Result<CodeSuggestion>` with replacement and applicability

**Example**:
```rust
let effect = Effect::Replace {
    start_offset: 0,
    end_offset: 7,
    replacement: "Changed".to_string(),
};

let mut vars = HashMap::new();
vars.insert("name".to_string(), "NewName".to_string());

let suggestion = effect.apply("Original text", &vars, "file.fsh")?;
assert_eq!(suggestion.replacement, "Changed");
```

**`interpolate_variables`**
```rust
pub fn interpolate_variables(
    template: &str,
    variables: &HashMap<String, String>,
) -> Result<String>
```
Replaces `$variable` references in template with values.

**Parameters**:
- `template: &str` - Template string with `$variable` references
- `variables: &HashMap<String, String>` - Variable values

**Returns**: `Result<String>` with interpolated text

**Example**:
```rust
let template = "Profile: $name with parent $parent";
let mut vars = HashMap::new();
vars.insert("name".to_string(), "MyProfile".to_string());
vars.insert("parent".to_string(), "Patient".to_string());

let result = Effect::interpolate_variables(template, &vars)?;
assert_eq!(result, "Profile: MyProfile with parent Patient");
```

**`is_safe`**
```rust
pub fn is_safe(&self) -> bool
```
Determines if effect is safe to apply automatically.

**Returns**:
- `true` - Safe (Replace, cosmetic RewriteField)
- `false` - Unsafe (Insert, Delete, semantic RewriteField)

**Example**:
```rust
let safe_effect = Effect::Replace {
    start_offset: 0,
    end_offset: 5,
    replacement: "new".to_string(),
};
assert!(safe_effect.is_safe());

let unsafe_effect = Effect::Delete {
    start_offset: 0,
    end_offset: 10,
};
assert!(!unsafe_effect.is_safe());
```

---

## Functions Module (`functions.rs`)

### `FunctionValue` Enum

Represents values that can be returned from transformation functions.

```rust
pub enum FunctionValue {
    String(String),
    Number(i64),
    Boolean(bool),
}
```

#### Methods

**`as_string`**
```rust
pub fn as_string(&self) -> Result<String>
```
Converts to string representation.

**Example**:
```rust
let val = FunctionValue::String("hello".to_string());
assert_eq!(val.as_string()?, "hello");

let num = FunctionValue::Number(42);
assert_eq!(num.as_string()?, "42");
```

#### Implementations

**`From<String>`**
```rust
impl From<String> for FunctionValue { ... }
```

**`From<&str>`**
```rust
impl From<&str> for FunctionValue { ... }
```

---

### `RewriteFunc` Type Alias

```rust
pub type RewriteFunc = fn(&[FunctionValue]) -> Result<FunctionValue>;
```

Function signature for transformation functions.

---

### `RewriteFunctionRegistry` Struct

Registry for managing transformation functions.

```rust
pub struct RewriteFunctionRegistry {
    // Private internal storage
}
```

#### Methods

**`new`**
```rust
pub fn new() -> Self
```
Creates a new registry with all 9 built-in functions pre-registered.

**Example**:
```rust
let registry = RewriteFunctionRegistry::new();
assert!(registry.has("capitalize"));
assert!(registry.has("to_kebab_case"));
```

**`register`**
```rust
pub fn register(&mut self, name: &str, func: RewriteFunc)
```
Registers a custom transformation function.

**Parameters**:
- `name: &str` - Function name
- `func: RewriteFunc` - Function implementation

**Example**:
```rust
let mut registry = RewriteFunctionRegistry::new();
registry.register("double", |args| {
    let val = args[0].as_string()?;
    Ok(FunctionValue::String(format!("{}{}", val, val)))
});
```

**`call`**
```rust
pub fn call(&self, name: &str, args: &[FunctionValue]) -> Result<FunctionValue>
```
Calls a registered function by name.

**Parameters**:
- `name: &str` - Function name
- `args: &[FunctionValue]` - Function arguments

**Returns**: `Result<FunctionValue>` from function

**Example**:
```rust
let registry = RewriteFunctionRegistry::new();
let result = registry.call("capitalize", &[FunctionValue::String("hello".into())])?;
assert_eq!(result.as_string()?, "Hello");
```

**`has`**
```rust
pub fn has(&self, name: &str) -> bool
```
Checks if function is registered.

**Example**:
```rust
let registry = RewriteFunctionRegistry::new();
assert!(registry.has("capitalize"));
assert!(!registry.has("unknown"));
```

---

### Built-in Functions

#### `capitalize`
```rust
pub fn capitalize(args: &[FunctionValue]) -> Result<FunctionValue>
```
Capitalizes first letter.

**Args**: 1 string
**Returns**: String with first character uppercased
**Example**: `"badName"` → `"BadName"`

#### `to_kebab_case`
```rust
pub fn to_kebab_case(args: &[FunctionValue]) -> Result<FunctionValue>
```
Converts to kebab-case.

**Args**: 1 string
**Returns**: String in kebab-case
**Example**: `"BadName"` → `"bad-name"`

#### `to_pascal_case`
```rust
pub fn to_pascal_case(args: &[FunctionValue]) -> Result<FunctionValue>
```
Converts to PascalCase.

**Args**: 1 string
**Returns**: String in PascalCase
**Example**: `"bad-name"` → `"BadName"`

#### `to_snake_case`
```rust
pub fn to_snake_case(args: &[FunctionValue]) -> Result<FunctionValue>
```
Converts to snake_case.

**Args**: 1 string
**Returns**: String in snake_case
**Example**: `"BadName"` → `"bad_name"`

#### `trim`
```rust
pub fn trim(args: &[FunctionValue]) -> Result<FunctionValue>
```
Removes leading/trailing whitespace.

**Args**: 1 string
**Returns**: Trimmed string
**Example**: `"  text  "` → `"text"`

#### `replace`
```rust
pub fn replace(args: &[FunctionValue]) -> Result<FunctionValue>
```
String replacement (find and replace).

**Args**: 3 strings (text, old, new)
**Returns**: String with replacements
**Example**: `replace("a_b", "_", "-")` → `"a-b"`

#### `concat`
```rust
pub fn concat(args: &[FunctionValue]) -> Result<FunctionValue>
```
Concatenates strings.

**Args**: 2+ strings
**Returns**: Concatenated string
**Example**: `concat("hello", " ", "world")` → `"hello world"`

#### `lowercase`
```rust
pub fn lowercase(args: &[FunctionValue]) -> Result<FunctionValue>
```
Converts to lowercase.

**Args**: 1 string
**Returns**: Lowercase string
**Example**: `"BadName"` → `"badname"`

#### `uppercase`
```rust
pub fn uppercase(args: &[FunctionValue]) -> Result<FunctionValue>
```
Converts to uppercase.

**Args**: 1 string
**Returns**: Uppercase string
**Example**: `"badname"` → `"BADNAME"`

---

## Validator Module (`validator.rs`)

### `ValidationResult` Enum

Result of validation operation.

```rust
pub enum ValidationResult {
    Safe,
    Unsafe { issues: Vec<String> },
}
```

#### Methods

**`is_safe`**
```rust
pub fn is_safe(&self) -> bool
```
Returns whether validation passed.

**`issues`**
```rust
pub fn issues(&self) -> Vec<&str>
```
Returns list of issues found.

---

### `RewriteValidator` Struct

Validates rewrites for safety and correctness.

```rust
pub struct RewriteValidator;
```

#### Methods

**`validate`**
```rust
pub fn validate(original: &str, rewritten: &str) -> Result<ValidationResult>
```
Validates that rewrite is safe and doesn't break code.

**Parameters**:
- `original: &str` - Original source code
- `rewritten: &str` - Rewritten source code

**Returns**: `Result<ValidationResult>` (Safe or Unsafe with issues)

**Checks**:
- Syntax validity (balanced brackets)
- Not producing empty content
- Reasonable line count changes

**Example**:
```rust
let original = "Profile: badName";
let rewritten = "Profile: BadName";
let result = RewriteValidator::validate(original, rewritten)?;
assert!(result.is_safe());
```

**`preview`**
```rust
pub fn preview(
    original: &str,
    replacement: &str,
    range: (usize, usize),
) -> String
```
Shows what code would look like after applying replacement.

**Parameters**:
- `original: &str` - Original source
- `replacement: &str` - Replacement text
- `range: (usize, usize)` - (start_offset, end_offset)

**Returns**: String with replacement applied

**Example**:
```rust
let original = "Hello World";
let result = RewriteValidator::preview(original, "Rust", (6, 11));
assert_eq!(result, "Hello Rust");
```

**`check_conflict`**
```rust
pub fn check_conflict(
    range1: (usize, usize),
    range2: (usize, usize),
) -> bool
```
Checks if two ranges overlap (conflict).

**Parameters**:
- `range1: (usize, usize)` - First range (start, end)
- `range2: (usize, usize)` - Second range (start, end)

**Returns**: `true` if ranges overlap, `false` otherwise

**Example**:
```rust
assert!(RewriteValidator::check_conflict((0, 10), (5, 15)));
assert!(!RewriteValidator::check_conflict((0, 5), (10, 15)));
```

---

## Executor Module (`executor.rs`)

### `GritQLMatchWithFix` Struct

Pairs a pattern match with optional autofix.

```rust
pub struct GritQLMatchWithFix {
    pub match_data: GritQLMatch,
    pub fix: Option<CodeSuggestion>,
}
```

#### Fields

- `match_data: GritQLMatch` - The original pattern match
- `fix: Option<CodeSuggestion>` - Optional autofix suggestion

---

### CompiledGritQLPattern Extensions

New methods for autofix generation:

**`execute_with_fixes`**
```rust
pub fn execute_with_fixes(
    &self,
    source: &str,
    file_path: &str,
) -> Result<Vec<GritQLMatchWithFix>>
```
Executes pattern and generates autofixes for all matches.

**Parameters**:
- `source: &str` - Source code to analyze
- `file_path: &str` - File path for diagnostics

**Returns**: `Result<Vec<GritQLMatchWithFix>>` with matches and fixes

**Example**:
```rust
let pattern = compiler.compile_pattern("Profile: $name", "rule")?;
let matches = pattern.execute_with_fixes(source, "test.fsh")?;

for match_fix in matches {
    if let Some(fix) = match_fix.fix {
        println!("Suggestion: {}", fix.message);
    }
}
```

**`to_diagnostic`**
```rust
pub fn to_diagnostic(
    &self,
    match_with_fix: GritQLMatchWithFix,
    file_path: &str,
) -> Diagnostic
```
Converts match with fix to Diagnostic object.

**Parameters**:
- `match_with_fix: GritQLMatchWithFix` - Match with optional fix
- `file_path: &str` - File path

**Returns**: `Diagnostic` with location and suggestion

**Example**:
```rust
let diagnostic = pattern.to_diagnostic(match_fix, "test.fsh");
println!("{}: {}", diagnostic.rule_id, diagnostic.message);
if let Some(suggestion) = diagnostic.suggestions.first() {
    println!("Suggestion: {}", suggestion.message);
}
```

#### New Fields

**`effect: Option<Effect>`**
Optional transformation effect for this pattern.

**`severity: Option<Severity>`**
Severity level for diagnostics (default: Warning).

**`message: Option<String>`**
Custom message for diagnostics.

---

## Type Aliases

### `RewriteFunc`
```rust
pub type RewriteFunc = fn(&[FunctionValue]) -> Result<FunctionValue>;
```
Function pointer type for transformation functions.

---

## Error Handling

All public APIs return `Result<T>` for proper error handling:

```rust
// Effect operations
effect.apply(source, &vars, file)?      // Returns Result<CodeSuggestion>
Effect::interpolate_variables(t, &v)?   // Returns Result<String>

// Function operations
registry.call("fn_name", &args)?        // Returns Result<FunctionValue>

// Validation operations
RewriteValidator::validate(o, r)?       // Returns Result<ValidationResult>

// Executor operations
pattern.execute_with_fixes(s, f)?       // Returns Result<Vec<...>>
```

Error types:
- `MakiError` - MAKI-specific error with context
- Message format: `"{rule_id}: {message}"`

**Example Error Handling**:
```rust
match effect.apply(source, &variables, file_path) {
    Ok(suggestion) => println!("Suggestion: {}", suggestion.message),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Performance Characteristics

### Time Complexity

| Operation | Time | Notes |
|-----------|------|-------|
| Effect creation | < 1μs | Memory allocation only |
| Variable interpolation | 10-50μs | Regex matching per variable |
| Function call | < 1μs | Direct function pointer call |
| Validation | 100-500μs | Bracket counting, range checking |
| Overall effect | < 1ms | Total overhead per autofix |

### Space Complexity

- `Effect`: 40-80 bytes depending on variant
- `FunctionValue`: 24-40 bytes depending on content
- `ValidationResult`: 24 bytes + issue strings
- `GritQLMatchWithFix`: 64 bytes + nested data

---

## Threading & Safety

All types are **thread-safe**:
- `Effect`: Immutable, safe to share
- `RewriteFunctionRegistry`: Can be shared via `Arc`
- `RewriteValidator`: Stateless, fully reusable
- `CompiledGritQLPattern`: Immutable after compilation

---

## Examples

### Complete Workflow
```rust
use maki_rules::gritql::*;
use std::collections::HashMap;

// 1. Create registry
let registry = RewriteFunctionRegistry::new();

// 2. Define effect
let effect = Effect::Replace {
    start_offset: 0,
    end_offset: 10,
    replacement: "$name".to_string(),
};

// 3. Prepare variables
let mut vars = HashMap::new();
vars.insert("name".to_string(), "NewValue".to_string());

// 4. Apply effect
let suggestion = effect.apply("OldValue123", &vars, "file.fsh")?;
println!("Message: {}", suggestion.message);
println!("Replacement: {}", suggestion.replacement);
println!("Safe: {}", effect.is_safe());

// 5. Validate
let preview = RewriteValidator::preview("OldValue123", "NewValue", (0, 10));
let validation = RewriteValidator::validate("OldValue123", &preview)?;
println!("Valid: {}", validation.is_safe());
```

---

## See Also

- [GritQL Autofixes Guide](./gritql-autofixes.md)
- [GritQL Patterns](./gritql-patterns.md)
- [MAKI API Documentation](./api.md)
