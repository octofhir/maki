# Task 32: Cardinality Validation Rules

**Phase**: 2 (Enhanced Linter - Weeks 9 & 11)
**Time Estimate**: 3-4 days
**Status**: 📝 Planned
**Priority**: Critical
**Dependencies**: Tasks 01-29 (Core linting), Task 29 (FHIR Definitions for semantic validation)

## Overview

Implement cardinality validation rules to ensure that element cardinality constraints are logically valid and compatible with parent definitions. This includes basic syntax validation (min ≤ max) and advanced semantic validation (child cardinality must be subset of parent).

**Part of Enhanced Linter Phase**: Critical error-level rule (#5 in Week 9) and complex semantic rule (#21 in Week 11).

## Context

Cardinality constraints define how many times an element can appear (e.g., `0..1`, `1..*`, `0..0`). Common issues:
- **Syntax errors**: Min > Max (e.g., `5..3`)
- **Semantic errors**: Child more permissive than parent (e.g., parent `0..1`, child `0..*`)
- **Logic errors**: Making required element optional or vice versa

SUSHI catches some of these during compilation, but MAKI provides earlier detection with clear diagnostics and autofixes.

## Goals

1. **Implement valid-cardinality rule** - Basic syntax validation (min ≤ max)
2. **Implement cardinality-conflicts rule** - Semantic validation against parent
3. **Implement cardinality-too-restrictive rule** - Warn about potentially problematic restrictions
4. **Provide autofixes** - Swap min/max when reversed, suggest fixes for conflicts
5. **Support array assignment validation** - Detect missing explicit indices (#1484)

## Technical Specification

### Rule 1: `valid-cardinality` (Week 9 - Critical)

**Category**: Blocking (Correctness)
**Severity**: Error
**Autofix**: Yes (swap min and max if reversed)

**Validation (Rust - CST-based):**
```rust
use maki_core::cst::ast::{Document, Profile, CardinalityRule, AstNode};
use maki_core::semantic::SemanticModel;
use maki_core::diagnostic::{Diagnostic, Severity};
use maki_core::autofix::CodeSuggestion;

pub fn check_valid_cardinality(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    for profile in document.profiles() {
        // Find all cardinality rules
        for rule in profile.cardinality_rules() {
            let (min, max) = rule.cardinality();

            // Check if min > max
            if let (Some(min_val), Some(max_val)) = (min, max) {
                if min_val > max_val {
                    let location = model.source_map.node_to_diagnostic_location(rule.syntax());

                    diagnostics.push(
                        Diagnostic::new(
                            "valid-cardinality",
                            Severity::Error,
                            format!("Invalid cardinality: minimum ({}) > maximum ({})", min_val, max_val),
                            location.clone(),
                        )
                        .with_note("Cardinality must be MIN..MAX where MIN ≤ MAX")
                        .with_help("Valid examples: 0..1, 1..*, 0..0, 2..5")
                        .with_suggestion(
                            CodeSuggestion::safe_fix(
                                format!("Swap to {}..{}", max_val, min_val),
                                format!("{}..{}", max_val, min_val),
                                location,
                            )
                        )
                    );
                }
            }

            // Check for 0..0 (prohibited)
            if min == Some(0) && max == Some(0) {
                let location = model.source_map.node_to_diagnostic_location(rule.syntax());

                diagnostics.push(
                    Diagnostic::new(
                        "valid-cardinality",
                        Severity::Warning,
                        "Cardinality 0..0 explicitly prohibits this element".to_string(),
                        location,
                    )
                    .with_note("0..0 means this element must not be present")
                    .with_help("This is valid but unusual - confirm this is intentional")
                );
            }
        }
    }

    diagnostics
}
```

**GritQL Alternative:**
```gritql
CardinalityRule: $rule where {
    $rule.min > $rule.max
} => {
    rewrite($rule) => format("{}..{}", $rule.max, $rule.min)
}
```

**Examples:**
```fsh
// ❌ Invalid: min > max
Profile: MyProfile
Parent: Patient
* name 5..3  // Error: 5 > 3

// After autofix ✅
* name 3..5  // Swapped

// ⚠️ Unusual but valid
* extension 0..0  // Explicitly prohibit extensions (warning)

// ✅ Valid cardinalities
* name 0..1      // Optional, max one
* name 1..1      // Required, exactly one
* name 1..*      // Required, at least one
* name 0..*      // Optional, any number
* telecom 2..5   // At least 2, max 5
```

### Rule 2: `cardinality-conflicts` (Week 11 - Semantic)

**Category**: Correctness (Semantic)
**Severity**: Error
**Autofix**: No (requires FHIR knowledge and user decision)

**Requires**: FHIR definitions via canonical-manager to check parent cardinality

**Validation (Rust - with FHIR definitions):**
```rust
use maki_core::cst::ast::{Document, Profile, AstNode};
use maki_core::semantic::SemanticModel;
use maki_core::canonical::CanonicalManager;
use maki_core::diagnostic::{Diagnostic, Severity};

pub fn check_cardinality_conflicts(
    model: &SemanticModel,
    manager: &CanonicalManager,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    for profile in document.profiles() {
        // Get parent resource type
        let Some(parent_node) = profile.parent() else {
            continue;
        };
        let parent_type = parent_node.text().to_string();

        // Find all cardinality rules
        for rule in profile.cardinality_rules() {
            let element_path = rule.element_path();
            let (child_min, child_max) = rule.cardinality();

            // Query FHIR definitions for parent cardinality
            match manager.get_element_cardinality(&parent_type, &element_path) {
                Ok((parent_min, parent_max)) => {
                    // Check if child is valid subset of parent
                    if !is_cardinality_subset(
                        (child_min, child_max),
                        (parent_min, parent_max),
                    ) {
                        let location = model.source_map.node_to_diagnostic_location(rule.syntax());

                        diagnostics.push(
                            Diagnostic::new(
                                "cardinality-conflicts",
                                Severity::Error,
                                format!(
                                    "Cardinality {:?} conflicts with parent cardinality {:?}",
                                    (child_min, child_max),
                                    (parent_min, parent_max)
                                ),
                                location,
                            )
                            .with_note(format!(
                                "Parent ({}.{}) requires {:?}, but this profile restricts to {:?}",
                                parent_type, element_path, (parent_min, parent_max), (child_min, child_max)
                            ))
                            .with_help("Child cardinality must be more restrictive than parent")
                        );
                    }
                }
                Err(_) => {
                    // Parent type not found in definitions - warn but don't error
                    let location = model.source_map.node_to_diagnostic_location(profile.syntax());
                    diagnostics.push(
                        Diagnostic::new(
                            "cardinality-conflicts",
                            Severity::Warning,
                            format!("Cannot validate cardinality: parent type '{}' not found in definitions", parent_type),
                            location,
                        )
                    );
                }
            }
        }
    }

    diagnostics
}
```

**GritQL Alternative:**
```gritql
Profile: $p where {
    $p.cardinality_violates_parent()
}
```

**FHIR Definitions Integration:**
```rust
// Initialize canonical manager (in rule engine)
let manager = CanonicalManager::new()?;
manager.ensure_packages(&["hl7.fhir.r4.core@4.0.1"])?;

// Pass to cardinality checker
let diagnostics = check_cardinality_conflicts(&model, &manager)?;
```

/// Check if child cardinality is a valid subset of parent cardinality
fn is_cardinality_subset(
    child: (Option<u32>, Option<u32>),
    parent: (Option<u32>, Option<u32>),
) -> bool {
    let (child_min, child_max) = child;
    let (parent_min, parent_max) = parent;

    // Child min must be >= parent min
    let min_ok = match (child_min, parent_min) {
        (Some(c), Some(p)) => c >= p,
        (None, _) => false,  // Child must specify min
        (Some(_), None) => true,  // Parent allows any, child restricts
    };

    // Child max must be <= parent max
    let max_ok = match (child_max, parent_max) {
        (Some(c), Some(p)) => c <= p,
        (None, None) => true,  // Both allow unbounded
        (Some(_), None) => true,  // Parent unbounded, child bounded
        (None, Some(_)) => false,  // Parent bounded, child unbounded - conflict!
    };

    min_ok && max_ok
}
```

**Examples:**
```fsh
// ❌ Cardinality conflict
Profile: RestrictivePatientProfile
Parent: Patient  // Patient.name is 0..*
* name 5..*  // Error: Requires at least 5, but parent allows 0

// ❌ Another conflict
Profile: MyPatientProfile
Parent: Patient  // Patient.birthDate is 0..1
* birthDate 0..*  // Error: Allows multiple, but parent allows max 1

// ✅ Valid restrictions
Profile: MyPatientProfile
Parent: Patient
* name 1..*      // ✅ Parent is 0..*, child requires at least 1
* gender 1..1    // ✅ Parent is 0..1, child makes it required
* birthDate 0..1 // ✅ Same as parent (no restriction, but valid)
```

### Rule 3: `cardinality-too-restrictive` (Week 10 - Warning)

**Category**: Best Practice
**Severity**: Warning
**Autofix**: No (requires domain knowledge)

**Validation:**
```rust
pub fn check_cardinality_too_restrictive(
    element_path: &str,
    cardinality: (Option<u32>, Option<u32>),
    parent_card: (Option<u32>, Option<u32>),
) -> Option<Diagnostic> {
    let (min, max) = cardinality;
    let (parent_min, parent_max) = parent_card;

    // Warn about making optional elements required (0..X → 1..X)
    if parent_min == Some(0) && min == Some(1) {
        return Some(
            DiagnosticBuilder::new(Severity::Warning)
                .message("Making optional element required")
                .label(range, format!("was {}", format_cardinality(parent_card)))
                .note("This profile requires an element that the base resource allows to be optional")
                .help("Ensure this restriction is intentional and documented")
                .build()
        );
    }

    // Warn about 1..1 on optional base elements
    if parent_min == Some(0) && min == Some(1) && max == Some(1) {
        return Some(
            DiagnosticBuilder::new(Severity::Warning)
                .message("Setting optional element to exactly one (1..1)")
                .label(range, "very restrictive")
                .note("Base resource allows this to be missing, but this profile requires exactly one")
                .help("Consider if 1..* would be more appropriate")
                .build()
        );
    }

    None
}
```

### Rule 4: `array-assignment-missing-index` (Issue #1484)

**Category**: Correctness
**Severity**: Error
**Autofix**: Suggest adding [0] index

**Validation:**
```rust
pub fn check_array_assignment(assignment: &Assignment) -> Option<Diagnostic> {
    let path = assignment.path();

    // Check if path is to a multi-cardinality element
    if is_array_element(path) && !has_explicit_index(assignment) {
        return Some(
            DiagnosticBuilder::new(Severity::Error)
                .message("Array element assignment missing explicit index")
                .label(assignment.syntax().text_range(), "needs index")
                .note("Elements with cardinality > 1 require explicit indices")
                .help("Add [0] or another index: '* name[0].given = \"John\"'")
                .autofix(Fix {
                    range: find_path_end(assignment),
                    replacement: "[0]".to_string(),
                    safety: FixSafety::Safe,
                    description: "Add [0] index to array element",
                })
                .build()
        );
    }

    None
}
```

**Example:**
```fsh
// ❌ Missing array index
Profile: MyPatientProfile
Parent: Patient
* name.given = "John"  // Error: name is 0..*, needs index

// After autofix ✅
* name[0].given = "John"  // Explicit index added
```

## Implementation Location

**Files**:
- `crates/maki-rules/src/builtin/cardinality.rs` - Already exists
- Extend with advanced semantic checks using FHIR definitions

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_valid_cardinality_detects_reversed() {
    let source = r#"
        Profile: MyProfile
        Parent: Patient
        * name 5..3
    "#;

    let diagnostics = lint_source(source, &["valid-cardinality"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("5") && diagnostics[0].message.contains("3"));
    assert_eq!(diagnostics[0].severity, Severity::Error);
}

#[test]
fn test_autofix_swaps_cardinality() {
    let source = "* name 5..3";
    let fixed = apply_autofixes(source, &["valid-cardinality"]);
    assert!(fixed.contains("3..5"));
}

#[test]
fn test_cardinality_conflicts_parent() {
    // Requires FHIR definitions loaded
    let source = r#"
        Profile: MyProfile
        Parent: Patient
        * birthDate 0..*  // Patient.birthDate is 0..1
    "#;

    let diagnostics = lint_source_with_fhir(source, &["cardinality-conflicts"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("conflict"));
}

#[test]
fn test_valid_cardinality_restrictions() {
    let source = r#"
        Profile: MyProfile
        Parent: Patient
        * name 1..*   // Valid: parent is 0..*, child requires >= 1
        * gender 1..1 // Valid: parent is 0..1, child makes required
    "#;

    let diagnostics = lint_source_with_fhir(source, &["cardinality-conflicts"]);
    assert_eq!(diagnostics.len(), 0);
}
```

### Integration Tests

```bash
# Test cardinality validation
maki lint examples/cardinality/ --filter valid-cardinality,cardinality-conflicts

# Apply fixes
maki lint examples/cardinality/ --fix

# Verify fixes
maki lint examples/cardinality/
```

### Golden Files

**File**: `crates/maki-core/tests/golden_files/cardinality_errors.fsh`

```fsh
// Various cardinality validation errors

// Reversed cardinality
Profile: ReversedCardinality
Parent: Patient
* name 5..3  // min > max

// Conflict with parent
Profile: ConflictingCardinality
Parent: Patient
* birthDate 0..*  // Parent is 0..1, can't make it 0..*

// Too restrictive (warning)
Profile: TooRestrictive
Parent: Patient
* name 1..1  // Parent is 0..*, making it exactly 1 is very restrictive

// Missing array index
Instance: PatientExample
InstanceOf: Patient
* name.given = "John"  // name is 0..*, needs [0]
```

## Configuration

```toml
[rules.valid-cardinality]
enabled = true
severity = "error"
autofix = true

[rules.cardinality-conflicts]
enabled = true
severity = "error"
# Requires FHIR definitions loaded

[rules.cardinality-too-restrictive]
enabled = true
severity = "warning"

[rules.cardinality-too-restrictive.config]
# Warn when making optional required
warn_optional_to_required = true
# Warn when restricting to exactly 1
warn_exactly_one = true

[rules.array-assignment-missing-index]
enabled = true
severity = "error"
autofix = true
```

## Dependencies

### Required Components
- **CST/AST** (Tasks 03-04): For parsing cardinality rules
- **FHIR Definitions** (Task 29): For parent cardinality lookup
- **Semantic Analyzer**: For element resolution

## Acceptance Criteria

- [ ] `valid-cardinality` detects min > max errors
- [ ] `valid-cardinality` autofix swaps min and max
- [ ] `valid-cardinality` warns about 0..0 (prohibited elements)
- [ ] `cardinality-conflicts` detects parent/child conflicts
- [ ] `cardinality-conflicts` shows clear diagnostic with parent info
- [ ] `is_cardinality_subset()` correctly validates all combinations
- [ ] `cardinality-too-restrictive` warns about restrictive changes
- [ ] `array-assignment-missing-index` detects missing indices
- [ ] `array-assignment-missing-index` autofix adds [0]
- [ ] Configuration supports enabling/disabling checks
- [ ] Unit tests cover all cardinality scenarios
- [ ] Integration tests verify fixes
- [ ] Performance: <5ms per element (with FHIR defs loaded)

## Related Tasks

- **Task 36: Profile Consistency** - Uses cardinality validation
- **Task 21: Type Constraints** - Similar conflict detection logic
- **Task 23: Binding Strength** - Similar parent/child validation

---

**Status**: Partially implemented (basic validation exists)
**Estimated Complexity**: High (requires FHIR definitions integration)
**Priority**: Critical (blocking rule)
**Updated**: 2025-11-03
