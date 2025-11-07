# Task 29: Complete GritQL Pattern Execution Engine

**Priority**: CRITICAL
**Estimated Effort**: 6-7 weeks
**Status**: Infrastructure Complete, Execution Missing
**Phase**: 1.5 - GritQL Integration (NEW - inserted before Phase 2)

---

## Overview

Complete the GritQL pattern execution engine to enable users to write custom FSH lint rules using GritQL patterns without requiring Rust knowledge. MAKI has excellent GritQL infrastructure (80% complete), but pattern execution is currently hardcoded rather than using the actual `grit-pattern-matcher` engine.

**Current State**:
- ✅ Complete CST adapter layer (FshGritTree, FshGritNode, FshGritCursor)
- ✅ Solid pattern parser and compiler
- ✅ Query context implementation (924 lines)
- ✅ Good documentation and examples
- ✅ 44 unit tests passing
- ❌ **Critical gap**: Pattern execution is hardcoded in `executor.rs`

**Problem**: The `node_matches_pattern()` function contains hand-written pattern matching for 3-4 specific patterns instead of using the compiled GritQL patterns. This means only those hardcoded patterns work, and adding new patterns requires Rust code changes.

```rust
// Current problematic code (executor.rs:261-342)
fn node_matches_pattern(&self, node: &FshGritNode) -> bool {
    // HARDCODED pattern matching
    if self.pattern.contains("Profile:") && self.pattern.contains(r#"r"^[a-z]"#) {
        // Manual regex check implementation
    }
    // Only 3-4 patterns supported this way!
}
```

**Goal**: Replace hardcoded matching with real GritQL execution so any valid GritQL pattern works.

---

## Objectives

### 1. Remove Hardcoded Pattern Matching (Week 1-2)

**Current Issue**: `executor.rs` contains manual pattern matching logic that only supports a few specific patterns.

**Solution**: Integrate with `grit-pattern-matcher`'s actual execution engine.

**Implementation**:

```rust
// crates/maki-rules/src/gritql/executor.rs

use grit_pattern_matcher::{Pattern, State, Logs};
use crate::gritql::query_context::{FshExecContext, FshResolvedPattern};

impl CompiledGritQLPattern {
    /// Execute the pattern using grit-pattern-matcher (NOT hardcoded)
    pub fn execute(&self, source: &str, file_path: &str) -> Result<Vec<GritQLMatch>> {
        // Parse source to GritQL tree
        let tree = FshGritTree::parse(source)?;

        // Create execution context
        let context = FshExecContext::new(
            file_path.to_string(),
            source.to_string(),
        );

        // Initialize state for pattern matching
        let mut state = State::new(&context, Logs::new());

        // Create root binding from tree
        let root_binding = FshResolvedPattern::from_tree(&tree);

        // Execute pattern using grit-pattern-matcher
        let matched = self.compiled_pattern.execute(
            &root_binding,
            &mut state,
            &context,
            &mut state.logs,
        )?;

        // Convert grit-pattern-matcher results to GritQLMatch
        if matched {
            Ok(self.extract_matches(&root_binding, &state)?)
        } else {
            Ok(Vec::new())
        }
    }

    /// Extract match information from successful pattern execution
    fn extract_matches(
        &self,
        binding: &FshResolvedPattern,
        state: &State<FshExecContext>,
    ) -> Result<Vec<GritQLMatch>> {
        let mut matches = Vec::new();

        // Get all nodes that matched the pattern
        for node in binding.matched_nodes(state) {
            let range = node.byte_range();
            let text = node.text().to_string();

            // Extract variable bindings (if any)
            let variables = self.extract_variables(node, state)?;

            matches.push(GritQLMatch {
                range,
                text,
                variables,
                node: node.clone(),
            });
        }

        Ok(matches)
    }
}
```

**Files to Modify**:
- `crates/maki-rules/src/gritql/executor.rs` - Remove hardcoded matching
- `crates/maki-rules/src/gritql/query_context.rs` - Ensure all traits properly implemented

**Tests**:
```rust
#[test]
fn test_real_pattern_execution_not_hardcoded() {
    let pattern = r#"Profile: $name where { $name <: r"^[a-z]" }"#;
    let compiler = GritQLCompiler::new().unwrap();
    let compiled = compiler.compile_pattern(pattern, "test-rule").unwrap();

    let source = r#"
Profile: badName
Parent: Patient
"#;

    let matches = compiled.execute(source, "test.fsh").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].variables.get("name"), Some(&"badName".to_string()));
}
```

---

### 2. Variable Capture & Binding (Week 2-3)

**Current Issue**: Variables are parsed but never bound to matched values.

**Solution**: Implement proper variable binding during pattern execution and make captures available in diagnostics.

**Implementation**:

```rust
// crates/maki-rules/src/gritql/executor.rs

impl CompiledGritQLPattern {
    /// Extract variable bindings from matched state
    fn extract_variables(
        &self,
        node: &FshGritNode,
        state: &State<FshExecContext>,
    ) -> Result<HashMap<String, String>> {
        let mut variables = HashMap::new();

        // Iterate over all variables in the pattern
        for var_name in &self.variable_names {
            // Get binding from state
            if let Some(binding) = state.get_binding(var_name) {
                // Extract text from bound node
                let text = binding.text().to_string();
                variables.insert(var_name.clone(), text);
            }
        }

        Ok(variables)
    }
}

/// Enhanced GritQLMatch with variable captures
#[derive(Debug, Clone)]
pub struct GritQLMatch {
    pub range: (usize, usize),
    pub text: String,
    pub variables: HashMap<String, String>,  // NEW: captured variables
    pub node: FshGritNode,
}
```

**Usage in Diagnostics**:

```rust
// Example: Use captured variables in diagnostic messages
for grit_match in pattern.execute(source, file_path)? {
    let name = grit_match.variables.get("name").unwrap();

    let diagnostic = Diagnostic::new(
        "naming-convention",
        Severity::Warning,
        format!("Profile name '{}' should start with uppercase letter", name),
        location,
    ).with_suggestion(
        CodeSuggestion::safe_fix(
            format!("Rename to '{}'", capitalize(name)),
            capitalize(name),
            location,
        )
    );
}
```

**Tests**:
```rust
#[test]
fn test_variable_capture() {
    let pattern = r#"Profile: $name where { $name <: r"^[a-z]" }"#;
    let source = "Profile: badName\nParent: Patient";

    let matches = execute_pattern(pattern, source)?;
    assert_eq!(matches[0].variables.get("name"), Some(&"badName".to_string()));
}

#[test]
fn test_multiple_variable_capture() {
    let pattern = r#"Profile: $name
                     Parent: $parent
                     where { $name <: r"^[a-z]" }"#;
    let source = "Profile: badName\nParent: Patient";

    let matches = execute_pattern(pattern, source)?;
    assert_eq!(matches[0].variables.get("name"), Some(&"badName".to_string()));
    assert_eq!(matches[0].variables.get("parent"), Some(&"Patient".to_string()));
}
```

---

### 3. Predicate Evaluation (Week 3-4)

**Current Issue**: Predicates are parsed but not evaluated during execution.

**Solution**: Implement full predicate evaluation for all common operators.

**Predicates to Support**:

| Operator | Meaning | Example |
|----------|---------|---------|
| `<:` | Regex match | `$name <: r"^[A-Z]"` |
| `contains` | String contains | `$desc contains "FHIR"` |
| `startsWith` | String starts with | `$id startsWith "myorg-"` |
| `endsWith` | String ends with | `$url endsWith "/Patient"` |
| `==` | Equality | `$parent == "Patient"` |
| `!=` | Inequality | `$status != "draft"` |
| `and` | Logical AND | `$a and $b` |
| `or` | Logical OR | `$a or $b` |
| `not` | Logical NOT | `not $condition` |

**Implementation**:

```rust
// crates/maki-rules/src/gritql/compiler.rs

impl GritQLCompiler {
    /// Compile predicate to grit-pattern-matcher Pattern
    fn compile_predicate(&mut self, pred: &Predicate) -> Result<Pattern<FshQueryContext>> {
        match pred {
            Predicate::RegexMatch { variable, regex } => {
                // Compile: $var <: r"pattern"
                let var_pattern = self.get_variable_pattern(variable)?;
                let regex_pattern = Pattern::Regex(regex.clone());
                Pattern::Match {
                    left: Box::new(var_pattern),
                    right: Box::new(regex_pattern),
                }
            }

            Predicate::Contains { variable, substring } => {
                // Compile: $var contains "substring"
                let var_pattern = self.get_variable_pattern(variable)?;
                Pattern::Contains {
                    haystack: Box::new(var_pattern),
                    needle: substring.clone(),
                }
            }

            Predicate::Equality { left, right } => {
                // Compile: $a == $b or $a == "value"
                Pattern::Equal {
                    left: Box::new(self.compile_expr(left)?),
                    right: Box::new(self.compile_expr(right)?),
                }
            }

            Predicate::And { left, right } => {
                Pattern::And(vec![
                    self.compile_predicate(left)?,
                    self.compile_predicate(right)?,
                ])
            }

            Predicate::Or { left, right } => {
                Pattern::Or(vec![
                    self.compile_predicate(left)?,
                    self.compile_predicate(right)?,
                ])
            }

            Predicate::Not { inner } => {
                Pattern::Not(Box::new(self.compile_predicate(inner)?))
            }
        }
    }
}
```

**Tests**:
```rust
#[test]
fn test_regex_predicate() {
    let pattern = r#"Profile: $name where { $name <: r"^[A-Z]" }"#;
    let source_match = "Profile: GoodName\nParent: Patient";
    let source_no_match = "Profile: badName\nParent: Patient";

    assert_eq!(execute_pattern(pattern, source_match)?.len(), 1);
    assert_eq!(execute_pattern(pattern, source_no_match)?.len(), 0);
}

#[test]
fn test_contains_predicate() {
    let pattern = r#"Profile where { description contains "patient" }"#;
    let source = r#"
Profile: MyProfile
Description: "This is a patient profile"
"#;

    assert_eq!(execute_pattern(pattern, source)?.len(), 1);
}

#[test]
fn test_complex_predicate() {
    let pattern = r#"
Profile: $name where {
    $name <: r"^[A-Z]" and
    not ($name contains "Test")
}
"#;
    let source_match = "Profile: GoodProfile";
    let source_no_match = "Profile: TestProfile";

    assert_eq!(execute_pattern(pattern, source_match)?.len(), 1);
    assert_eq!(execute_pattern(pattern, source_no_match)?.len(), 0);
}
```

---

### 4. Field Access Syntax (Week 4-5)

**Current Issue**: Cannot access fields of matched nodes within patterns.

**Solution**: Implement field access syntax like `$profile.name`, `$extension.title`.

**Syntax**:
```gritql
Profile: $p where {
    $p.name <: r"^[A-Z]",
    $p.parent == "Patient",
    $p.title contains "Example"
}

Extension: $e where {
    $e.context.type == "element",
    $e.url startsWith "http://example.org/"
}
```

**Implementation**:

```rust
// crates/maki-rules/src/gritql/parser.rs

#[derive(Debug, Clone)]
pub enum Expression {
    Variable(String),
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },
    Literal(String),
}

impl Parser {
    fn parse_expression(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary_expression()?;

        // Handle field access: $var.field or $var.field.subfield
        while self.current_token() == Token::Dot {
            self.consume(Token::Dot)?;
            let field_name = self.expect_identifier()?;
            expr = Expression::FieldAccess {
                object: Box::new(expr),
                field: field_name,
            };
        }

        Ok(expr)
    }
}
```

**Compiler**:

```rust
// crates/maki-rules/src/gritql/compiler.rs

impl GritQLCompiler {
    fn compile_expr(&mut self, expr: &Expression) -> Result<Pattern<FshQueryContext>> {
        match expr {
            Expression::Variable(name) => {
                self.get_variable_pattern(name)
            }

            Expression::FieldAccess { object, field } => {
                // Compile $var.field to field access pattern
                let obj_pattern = self.compile_expr(object)?;
                Pattern::FieldAccess {
                    object: Box::new(obj_pattern),
                    field_name: field.clone(),
                }
            }

            Expression::Literal(value) => {
                Pattern::Literal(value.clone())
            }
        }
    }
}
```

**CST Adapter Integration**:

```rust
// crates/maki-rules/src/gritql/cst_adapter.rs

impl FshGritNode {
    /// Get field value by name (used by FieldAccess pattern)
    pub fn get_field(&self, field_name: &str) -> Option<FshGritNode> {
        // Use typed AST to access fields
        match self.kind() {
            SyntaxKind::Profile => {
                let profile = Profile::cast(self.syntax_node.clone())?;
                match field_name {
                    "name" => profile.name().map(|n| FshGritNode::new(n.syntax().clone())),
                    "id" => profile.id().map(|id| FshGritNode::new(id.syntax().clone())),
                    "parent" => profile.parent().map(|p| FshGritNode::new(p.syntax().clone())),
                    "title" => profile.title().map(|t| FshGritNode::new(t.syntax().clone())),
                    _ => None,
                }
            }
            SyntaxKind::Extension => {
                let ext = Extension::cast(self.syntax_node.clone())?;
                match field_name {
                    "name" => ext.name().map(|n| FshGritNode::new(n.syntax().clone())),
                    "id" => ext.id().map(|id| FshGritNode::new(id.syntax().clone())),
                    "url" => ext.url().map(|u| FshGritNode::new(u.syntax().clone())),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
```

**Tests**:
```rust
#[test]
fn test_field_access() {
    let pattern = r#"Profile: $p where { $p.name <: r"^[A-Z]" }"#;
    let source = "Profile: GoodName\nParent: Patient";

    let matches = execute_pattern(pattern, source)?;
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_nested_field_access() {
    let pattern = r#"Extension: $e where { $e.context.type == "element" }"#;
    let source = r#"
Extension: MyExtension
Context: Patient.name
"#;

    let matches = execute_pattern(pattern, source)?;
    assert_eq!(matches.len(), 1);
}
```

---

### 5. Custom Built-in Functions (Week 5-6)

**Current Issue**: Built-in functions are defined (`is_profile()`, `has_comment()`, etc.) but not callable from patterns.

**Solution**: Hook built-ins into grit-pattern-matcher's function registry.

**Built-ins to Support**:

| Function | Purpose | Example |
|----------|---------|---------|
| `is_profile($n)` | Check if node is Profile | `is_profile($node)` |
| `is_extension($n)` | Check if node is Extension | `is_extension($node)` |
| `is_value_set($n)` | Check if node is ValueSet | `is_value_set($node)` |
| `has_comment($n)` | Check if node has comments | `has_comment($profile)` |
| `has_url($n)` | Check if extension has URL | `has_url($extension)` |
| `is_kebab_case($s)` | Validate kebab-case | `is_kebab_case($id)` |
| `is_pascal_case($s)` | Validate PascalCase | `is_pascal_case($name)` |

**Implementation**:

```rust
// crates/maki-rules/src/gritql/builtins.rs

use grit_pattern_matcher::FunctionRegistry;

pub struct FshBuiltins;

impl FshBuiltins {
    /// Register all FSH-specific built-in functions
    pub fn register(registry: &mut FunctionRegistry<FshQueryContext>) {
        registry.register("is_profile", Self::is_profile);
        registry.register("is_extension", Self::is_extension);
        registry.register("has_comment", Self::has_comment);
        registry.register("has_url", Self::has_url);
        registry.register("is_kebab_case", Self::is_kebab_case);
        registry.register("is_pascal_case", Self::is_pascal_case);
    }

    fn is_profile(args: &[Value]) -> Result<bool> {
        let node = args[0].as_node()?;
        Ok(node.kind() == SyntaxKind::Profile)
    }

    fn has_comment(args: &[Value]) -> Result<bool> {
        let node = args[0].as_node()?;

        // Check if node has preceding comments (trivia)
        for token in node.syntax_node.descendants_with_tokens() {
            if let Some(token) = token.as_token() {
                for trivia in token.leading_trivia() {
                    if trivia.kind() == SyntaxKind::Comment {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    fn is_kebab_case(args: &[Value]) -> Result<bool> {
        let text = args[0].as_string()?;
        let regex = Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$")?;
        Ok(regex.is_match(&text))
    }
}
```

**Usage**:

```gritql
// Example: Find profiles without comments
Profile: $p where {
    not has_comment($p)
}

// Example: Find extensions missing URL
Extension: $e where {
    not has_url($e)
}

// Example: Validate naming conventions
Profile: $p where {
    is_pascal_case($p.name) and
    is_kebab_case($p.id)
}
```

**Tests**:
```rust
#[test]
fn test_builtin_has_comment() {
    let pattern = r#"Profile where { has_comment(.) }"#;

    let source_with_comment = r#"
// This is a comment
Profile: MyProfile
"#;
    let source_without = "Profile: MyProfile";

    assert_eq!(execute_pattern(pattern, source_with_comment)?.len(), 1);
    assert_eq!(execute_pattern(pattern, source_without)?.len(), 0);
}

#[test]
fn test_builtin_is_kebab_case() {
    let pattern = r#"Profile: $p where { is_kebab_case($p.id) }"#;

    let source_good = "Profile: MyProfile\nId: my-profile";
    let source_bad = "Profile: MyProfile\nId: MyProfile";

    assert_eq!(execute_pattern(pattern, source_good)?.len(), 1);
    assert_eq!(execute_pattern(pattern, source_bad)?.len(), 0);
}
```

---

### 6. Testing & Documentation (Week 6-7)

**Enable All Integration Tests**:

```rust
// crates/maki-rules/tests/gritql_integration_test.rs

// RE-ENABLE these tests (currently commented out)
#[test]
fn test_where_clause_field_exists() {
    let pattern = r#"Profile where { title }"#;
    let source = r#"
Profile: MyProfile
Title: "Example Profile"
"#;

    let matches = execute_pattern(pattern, source).unwrap();
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_where_clause_negation() {
    let pattern = r#"Profile where { not title }"#;
    let source = "Profile: MyProfile";

    let matches = execute_pattern(pattern, source).unwrap();
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_variable_binding() {
    let pattern = r#"Profile: $name where { $name <: r"^[A-Z]" }"#;
    let source = "Profile: GoodName\nParent: Patient";

    let matches = execute_pattern(pattern, source).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].variables.get("name"), Some(&"GoodName".to_string()));
}

#[test]
fn test_builtin_functions() {
    let pattern = r#"Profile: $p where { has_comment($p) }"#;
    let source = r#"
// This profile represents a patient
Profile: PatientProfile
"#;

    let matches = execute_pattern(pattern, source).unwrap();
    assert_eq!(matches.len(), 1);
}
```

**Pattern Cookbook** (`examples/gritql/COOKBOOK.md`):

Create comprehensive cookbook with 20+ patterns:

```markdown
# GritQL Pattern Cookbook

## Naming Conventions

### Uppercase Profile Names
```gritql
Profile: $name where {
    $name <: r"^[A-Z]"
}
```

### Kebab-case IDs
```gritql
Profile: $p where {
    is_kebab_case($p.id)
}
```

## Metadata Requirements

### Missing Title
```gritql
Profile where {
    not title
}
```

### Missing Description
```gritql
or {
    Profile where { not description },
    Extension where { not description },
    ValueSet where { not description }
}
```

## Documentation Quality

### Profiles Without Comments
```gritql
Profile: $p where {
    not has_comment($p)
}
```

### Missing URL in Extension
```gritql
Extension: $e where {
    not has_url($e)
}
```

## Complex Patterns

### Invalid Parent Type
```gritql
Profile: $p where {
    $p.parent <: r"^[a-z]",  // Parent should be PascalCase
    not is_builtin_type($p.parent)
}
```

### Inconsistent Naming
```gritql
Profile: $p where {
    not ($p.name == capitalize($p.id))
}
```

... (20+ total examples)
```

**Complete GritQL Reference** (`examples/gritql/REFERENCE.md`):

```markdown
# GritQL Reference for FSH

## Syntax

### Basic Pattern
```gritql
NodeType: $variable where { predicate }
```

### Node Types
- `Profile` - FSH Profile definitions
- `Extension` - FSH Extension definitions
- `ValueSet` - FSH ValueSet definitions
- `CodeSystem` - FSH CodeSystem definitions
- `Instance` - FSH Instance definitions
- `RuleSet` - FSH RuleSet definitions

### Variables
- `$name` - Capture variable
- `$variable` - Any identifier after `$`

### Predicates
- `$var <: r"regex"` - Regex match
- `$var contains "text"` - String contains
- `$var == "value"` - Equality
- `$var != "value"` - Inequality
- `not predicate` - Negation
- `pred1 and pred2` - Conjunction
- `pred1 or pred2` - Disjunction

### Field Access
- `$var.field` - Access field
- `$var.field.subfield` - Nested access

### Built-in Functions
- `is_profile($n)` - Check node type
- `has_comment($n)` - Check for comments
- `is_kebab_case($s)` - Validate naming
...
```

---

## CST/AST Integration

### GritQL Tree Adapter Architecture

MAKI's GritQL integration is built on the Rowan CST, providing lossless pattern matching:

```
FSH Source → Rowan CST → FshGritTree → GritQL Patterns
                                ↓
                         FshGritNode (matches)
                                ↓
                         Variable bindings
```

**Key Components**:

1. **FshGritTree** (`cst_tree.rs`):
   - Implements `Ast` trait from grit-pattern-matcher
   - Wraps Rowan green tree
   - Provides lossless text extraction

2. **FshGritNode** (`cst_adapter.rs`):
   - Implements `AstNode` trait
   - Lightweight wrapper (Arc-based, cheap cloning)
   - Navigation: parent, children, siblings
   - Trivia access: comments, whitespace

3. **FshGritCursor** (`cst_adapter.rs`):
   - Tree traversal cursor
   - Efficient iteration over matches

4. **FshQueryContext** (`query_context.rs`):
   - Execution context for pattern matching
   - Variable bindings
   - File handling

### Using Typed AST for Field Access

```rust
// crates/maki-rules/src/gritql/cst_adapter.rs

impl FshGritNode {
    pub fn get_field(&self, field_name: &str) -> Option<FshGritNode> {
        // Use typed AST (from cst/ast.rs)
        if let Some(profile) = Profile::cast(self.syntax_node.clone()) {
            match field_name {
                "name" => profile.name().map(|n| FshGritNode::new(n.syntax().clone())),
                "parent" => profile.parent().map(|p| FshGritNode::new(p.syntax().clone())),
                "title" => profile.title().map(|t| FshGritNode::new(t.syntax().clone())),
                "description" => profile.description().map(|d| FshGritNode::new(d.syntax().clone())),
                _ => None,
            }
        } else {
            None
        }
    }
}
```

---

## Performance Considerations

### Pattern Compilation Caching

```rust
// Cache compiled patterns to avoid re-compilation
pub struct PatternCache {
    cache: Arc<Mutex<HashMap<String, Arc<CompiledGritQLPattern>>>>,
}

impl PatternCache {
    pub fn get_or_compile(&self, pattern: &str) -> Result<Arc<CompiledGritQLPattern>> {
        let mut cache = self.cache.lock().unwrap();

        if let Some(compiled) = cache.get(pattern) {
            return Ok(Arc::clone(compiled));
        }

        let compiler = GritQLCompiler::new()?;
        let compiled = Arc::new(compiler.compile_pattern(pattern, "cached")?);
        cache.insert(pattern.to_string(), Arc::clone(&compiled));

        Ok(compiled)
    }
}
```

### Execution Targets

- **Simple pattern** (e.g., `Profile where { not title }`): < 10ms per file
- **Complex pattern** (e.g., multiple predicates, field access): < 50ms per file
- **Pattern compilation**: < 5ms (amortized via caching)

### Optimization Strategies

1. **Cache compiled patterns** - Avoid re-parsing pattern strings
2. **Parallel execution** - Run multiple patterns concurrently
3. **Incremental matching** - Only re-match changed subtrees (future)
4. **Early termination** - Stop on first match when appropriate

---

## Dependencies

### Crates
- `grit-pattern-matcher` (0.5.1) - Core pattern matching engine
- `grit-util` (0.5.1) - Utilities for GritQL
- `regex` - Regular expression support
- `maki-core` - CST, AST, and semantic model

### Internal Modules
- `crates/maki-core/src/cst/` - Rowan-based CST
- `crates/maki-core/src/cst/ast.rs` - Typed AST layer
- `crates/maki-rules/src/gritql/` - GritQL integration

---

## Testing Strategy

### Unit Tests

Test each component in isolation:

```rust
// cst_adapter.rs
#[test]
fn test_node_navigation() { ... }

// compiler.rs
#[test]
fn test_predicate_compilation() { ... }

// executor.rs
#[test]
fn test_pattern_execution() { ... }
```

### Integration Tests

Test end-to-end pattern execution:

```rust
// crates/maki-rules/tests/gritql_integration_test.rs

#[test]
fn test_complex_pattern() {
    let pattern = r#"
Profile: $p where {
    $p.name <: r"^[A-Z]",
    not has_comment($p),
    $p.parent == "Patient"
}
"#;

    let source = r#"
Profile: PatientProfile
Parent: Patient
"#;

    let matches = execute_pattern(pattern, source).unwrap();
    assert_eq!(matches.len(), 1);
}
```

### Golden File Tests

Test patterns against real-world FSH files:

```rust
#[test]
fn test_gritql_on_uscore() {
    let pattern = load_pattern("examples/gritql/rules/naming.grit");
    let sources = load_golden_files("crates/maki-core/tests/golden_files/");

    for (path, source) in sources {
        let matches = pattern.execute(&source, &path).unwrap();
        // Validate matches
    }
}
```

---

## Examples

### Example 1: Basic Pattern Matching

**Pattern** (`examples/gritql/rules/missing-title.grit`):
```gritql
Profile where {
    not title
}
```

**Test FSH**:
```fsh
Profile: MissingTitle
Parent: Patient
Description: "This profile has no title"
```

**Expected**: 1 match (Profile without title)

---

### Example 2: Variable Capture

**Pattern**:
```gritql
Profile: $name where {
    $name <: r"^[a-z]"
}
```

**Test FSH**:
```fsh
Profile: badName
Parent: Patient
```

**Expected**: 1 match with `variables.name = "badName"`

---

### Example 3: Field Access

**Pattern**:
```gritql
Profile: $p where {
    $p.name <: r"^[A-Z]",
    $p.id <: r"^[a-z-]+$"
}
```

**Test FSH**:
```fsh
Profile: GoodProfile
Id: good-profile
```

**Expected**: 1 match (both predicates satisfied)

---

### Example 4: Built-in Functions

**Pattern**:
```gritql
Extension: $e where {
    not has_url($e)
}
```

**Test FSH**:
```fsh
Extension: MissingUrl
Description: "This extension has no URL"
```

**Expected**: 1 match (Extension without URL)

---

## Acceptance Criteria

### Week 1-2: Execution Engine
- [ ] Remove all hardcoded pattern matching from `executor.rs`
- [ ] Integrate with `grit-pattern-matcher.execute()`
- [ ] Pattern execution returns correct matches
- [ ] At least 3 basic patterns work (not hardcoded)

### Week 2-3: Variable Binding
- [ ] Variables are captured during execution
- [ ] `GritQLMatch.variables` contains all bound variables
- [ ] Variables usable in diagnostic messages
- [ ] Test: multiple variable capture works

### Week 3-4: Predicates
- [ ] Regex matching (`<:`) works
- [ ] String operations (contains, startsWith, endsWith) work
- [ ] Equality/inequality (`==`, `!=`) works
- [ ] Logical operators (and, or, not) work
- [ ] Test: complex predicate combinations work

### Week 4-5: Field Access
- [ ] `$var.field` syntax parsed correctly
- [ ] Field access works for Profile, Extension, ValueSet
- [ ] Nested field access (`$var.field.subfield`) works
- [ ] Test: field access in predicates works

### Week 5-6: Built-in Functions
- [ ] All built-ins registered in function registry
- [ ] Built-ins callable from patterns
- [ ] FSH-specific built-ins work (is_profile, has_comment, etc.)
- [ ] Test: patterns using built-ins work

### Week 6-7: Testing & Docs
- [ ] All integration tests enabled and passing
- [ ] Pattern cookbook with 20+ examples created
- [ ] Complete GritQL reference documentation
- [ ] Video tutorial recorded (optional)

### Overall Success
- [ ] Users can write custom FSH rules in GritQL
- [ ] No hardcoded pattern matching remains
- [ ] Performance targets met (< 50ms per pattern per file)
- [ ] Documentation complete and clear

---

## Risks & Mitigation

### Risk 1: grit-pattern-matcher API Limitations

**Risk**: grit-pattern-matcher may not expose all needed APIs for execution.

**Likelihood**: MEDIUM
**Impact**: HIGH (blocks entire task)

**Mitigation**:
- Review grit-pattern-matcher source code before starting
- Consider forking if necessary
- Fallback: Use AST pattern system (already working) for complex cases

### Risk 2: Performance Degradation

**Risk**: Real GritQL execution may be slower than hardcoded matching.

**Likelihood**: LOW
**Impact**: MEDIUM

**Mitigation**:
- Implement pattern caching early
- Benchmark against hardcoded version
- Optimize hot paths if needed

### Risk 3: Complex Pattern Debugging

**Risk**: Users may struggle to debug broken patterns.

**Likelihood**: HIGH
**Impact**: MEDIUM

**Mitigation**:
- Provide excellent error messages
- Build pattern debugger (Task 29.7)
- Create comprehensive cookbook

---

## Follow-up Tasks

This task enables:

- **Task 29.5**: GritQL Code Rewriting & Autofixes (depends on this task)
- **Task 29.7**: GritQL Developer Experience (depends on this task)
- **Task 30-39**: Enhanced linter rules (can use GritQL patterns)
- **All future custom rules**: Users can write without Rust knowledge

---

## Notes

### Why This Task Is Critical

MAKI has invested ~100+ hours in GritQL infrastructure, but it's not usable because execution is hardcoded. Completing this task unlocks:

1. **User empowerment**: Write custom rules without Rust
2. **Reduced maintenance**: Patterns vs code
3. **Community contributions**: Users can share patterns
4. **Competitive advantage**: Best-in-class FSH linting

### Current Code Quality

The existing GritQL infrastructure is **excellent**:
- Well-tested (44 tests passing)
- Well-documented (good inline comments)
- Well-architected (clean separation of concerns)
- **Just needs execution to work!**

### Estimated ROI

**Investment**: 6-7 weeks
**Return**:
- 10x faster rule development
- Community-driven rule library
- Reduced support burden
- Market differentiation

**Verdict**: HIGH ROI, should be top priority for Phase 1.5
