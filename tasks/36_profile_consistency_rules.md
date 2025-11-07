# Task 36: Profile Consistency Rules

**Phase**: 2 (Enhanced Linter - Week 11)
**Time Estimate**: 2-3 days
**Status**: 📝 Planned
**Priority**: Medium
**Dependencies**: Tasks 01-29, Tasks 30-35 (Other Phase 2 rules), Task 29 (FHIR Definitions)

## Overview

Implement comprehensive profile consistency checks to ensure profiles follow FHIR best practices and maintain logical consistency across constraints. This includes type constraint validation, reference target validation, and detection of logical inconsistencies that could cause runtime issues.

**Part of Enhanced Linter Phase**: Semantic validation rules from Week 11 (#22-27) focusing on complex cross-element validation.

## Context

Profiles can have subtle consistency issues that don't violate FHIR syntax but cause problems:
- **Type conflicts**: Constraining element to incompatible types
- **Invalid references**: References to non-existent resource types
- **Slice naming**: Slice names that conflict with element names
- **Extension cardinality**: Extension constraints that don't match usage patterns
- **MustSupport propagation**: Inconsistent MS flag usage in element hierarchies

## Goals

1. **Implement type-constraint-conflicts rule** - Validate type constraints against parent
2. **Implement reference-target-validation rule** - Ensure reference targets exist
3. **Implement slice-name-collision rule** - Detect slice naming conflicts
4. **Implement must-support-propagation rule** - Check MS consistency
5. **Implement extension-cardinality rule** - Validate extension usage patterns

## Technical Specification

### Rule 1: `type-constraint-conflicts` (Week 11 - Semantic)

**Category**: Correctness (Semantic)
**Severity**: Error
**Autofix**: No (requires understanding data model)

**Validation (Rust - with FHIR definitions):**
```rust
use maki_core::cst::ast::{Document, Profile, TypeRule, AstNode};
use maki_core::semantic::SemanticModel;
use maki_core::canonical::CanonicalManager;
use maki_core::diagnostic::{Diagnostic, Severity};

pub fn check_type_constraint_conflicts(
    model: &SemanticModel,
    manager: &CanonicalManager,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    for profile in document.profiles() {
        let parent_type = match profile.parent() {
            Some(p) => p.text().to_string(),
            None => continue,
        };

        for type_rule in profile.type_rules() {
            let element_path = type_rule.path();
            let child_types = type_rule.types();

            // Get parent allowed types from FHIR definitions
            match manager.get_element_types(&parent_type, &element_path) {
                Ok(parent_types) => {
                    // Check if child types are subset of parent types
                    for child_type in child_types {
                        if !is_type_compatible(&child_type, &parent_types) {
                            let location = model.source_map.node_to_diagnostic_location(type_rule.syntax());

                            diagnostics.push(
                                Diagnostic::new(
                                    "type-constraint-conflicts",
                                    Severity::Error,
                                    format!(
                                        "Type constraint '{}' is incompatible with parent types",
                                        child_type
                                    ),
                                    location,
                                )
                                .with_note(format!(
                                    "Parent allows: {}, child constrains to: {}",
                                    format_types(&parent_types),
                                    child_type
                                ))
                                .with_help("Type constraints must be a subset of parent types")
                            );
                        }
                    }
                }
                Err(_) => {
                    // Parent type not found - warn but don't error
                    let location = model.source_map.node_to_diagnostic_location(profile.syntax());
                    diagnostics.push(
                        Diagnostic::new(
                            "type-constraint-conflicts",
                            Severity::Warning,
                            format!("Cannot validate types: parent '{}' not found in definitions", parent_type),
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
    type_constraint_violates_parent($p)
}
```

fn is_type_compatible(child_type: &FhirType, parent_types: &[FhirType]) -> bool {
    parent_types.iter().any(|parent_type| {
        // Check if child_type is same as or derived from parent_type
        child_type == parent_type || is_subtype_of(child_type, parent_type)
    })
}
```

**Examples:**
```fsh
// ❌ Type conflict
Profile: InvalidTypeProfile
Parent: Observation  // value[x] can be Quantity | CodeableConcept | string | ...
* value[x] only boolean  // Error: boolean not in parent's allowed types!

// ✅ Valid type restriction
Profile: ValidTypeProfile
Parent: Observation
* value[x] only Quantity  // OK: Quantity is one of parent's allowed types

// ✅ More specific type
Profile: SpecificQuantityProfile
Parent: Observation
* valueQuantity only SimpleQuantity  // OK: SimpleQuantity is subtype of Quantity
```

### Rule 2: `reference-target-validation` (Week 11 - Semantic)

**Category**: Correctness
**Severity**: Error
**Autofix**: No

**Validation:**
```rust
pub fn check_reference_target_validation(
    reference_rule: &ReferenceRule,
    workspace: &Workspace,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for target in reference_rule.targets() {
        let target_name = target.value();

        // Check if target exists in FHIR or project
        if !workspace.has_resource_type(target_name) &&
           !workspace.has_profile(target_name) {
            diagnostics.push(
                DiagnosticBuilder::new(Severity::Error)
                    .message(format!(
                        "Reference target '{}' does not exist",
                        target_name
                    ))
                    .label(target.syntax().text_range(), "undefined target")
                    .note("Reference targets must be valid FHIR resource types or defined profiles")
                    .help(format!(
                        "Check spelling of '{}' or define the profile",
                        target_name
                    ))
                    .build()
            );
        }
    }

    diagnostics
}
```

**Examples:**
```fsh
// ❌ Invalid reference target
Profile: MyProfile
Parent: Observation
* subject only Reference(NonExistentResource)  // Error: No such resource!

// ✅ Valid reference targets
Profile: ValidProfile
Parent: Observation
* subject only Reference(Patient | Group)  // OK: Both exist in FHIR

// ✅ Reference to project profile
Profile: MyPatientProfile
Parent: Patient

Profile: MyObservationProfile
Parent: Observation
* subject only Reference(MyPatientProfile)  // OK: Profile defined in project
```

### Rule 3: `slice-name-collision` (Week 11 - Semantic)

**Category**: Correctness
**Severity**: Error
**Autofix**: No

**Validation:**
```rust
pub fn check_slice_name_collision(
    profile: &Profile,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for slice_rule in profile.slice_rules() {
        let base_path = slice_rule.base_path();
        let slice_name = slice_rule.slice_name();

        // Check if slice name conflicts with existing element names
        if is_element_name_at_path(base_path, slice_name, profile) {
            diagnostics.push(
                DiagnosticBuilder::new(Severity::Error)
                    .message(format!(
                        "Slice name '{}' conflicts with element name",
                        slice_name
                    ))
                    .label(slice_rule.syntax().text_range(), "name collision")
                    .note(format!(
                        "There is already an element named '{}' at path '{}'",
                        slice_name, base_path
                    ))
                    .help("Choose a different slice name that doesn't conflict")
                    .build()
            );
        }
    }

    diagnostics
}
```

**Examples:**
```fsh
// ❌ Slice name collision
Profile: SliceCollision
Parent: Patient
* identifier contains code 1..1  // Error: 'code' is already an element name!

// ✅ Valid slice names
Profile: ValidSlicing
Parent: Patient
* identifier contains
    MRN 1..1 and
    SSN 0..1
* identifier[MRN].system = "http://hospital.org/mrn"
* identifier[SSN].system = "http://ssa.gov/ssn"
```

### Rule 4: `must-support-propagation` (Week 11 - Semantic)

**Category**: Best Practice
**Severity**: Warning
**Autofix**: Suggest adding MS to children

**Validation:**
```rust
pub fn check_must_support_propagation(
    profile: &Profile,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Find elements marked as MS
    let ms_elements = find_ms_elements(profile);

    for ms_elem in ms_elements {
        // Check if child elements exist without MS flag
        let children = find_child_elements(ms_elem, profile);

        for child in children {
            if !has_ms_flag(&child) {
                diagnostics.push(
                    DiagnosticBuilder::new(Severity::Warning)
                        .message(format!(
                            "Child element '{}' of MS element should typically be MS",
                            child.path
                        ))
                        .label(child.range, "consider adding MS")
                        .note(format!(
                            "Parent element '{}' is marked Must Support",
                            ms_elem.path
                        ))
                        .help("Add MS flag if this child element should also be supported")
                        .autofix(Fix {
                            range: child.range.end(),
                            replacement: " MS".to_string(),
                            safety: FixSafety::Safe,
                            description: "Add MS flag",
                        })
                        .build()
                );
            }
        }
    }

    diagnostics
}
```

**Examples:**
```fsh
// ⚠️ MS propagation warning
Profile: InconsistentMS
Parent: Patient
* name 1..* MS           // MS flag
* name.family 1..1       // Warning: Parent is MS, should this be MS too?
* name.given 1..* MS     // Good: Also MS

// ✅ Consistent MS usage
Profile: ConsistentMS
Parent: Patient
* name 1..* MS
* name.family 1..1 MS
* name.given 1..* MS
```

### Rule 5: `extension-cardinality` (Week 11 - Semantic)

**Category**: Best Practice
**Severity**: Warning
**Autofix**: No

**Validation:**
```rust
pub fn check_extension_cardinality(
    profile: &Profile,
    workspace: &Workspace,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Find extension rules
    for ext_rule in profile.extension_rules() {
        let ext_url = ext_rule.extension_url();
        let cardinality = ext_rule.cardinality();

        // Get extension definition
        if let Some(extension) = workspace.get_extension_by_url(ext_url) {
            // Check if cardinality matches extension's intended usage
            if let Some(suggested_card) = extension.suggested_cardinality() {
                if cardinality != suggested_card {
                    diagnostics.push(
                        DiagnosticBuilder::new(Severity::Warning)
                            .message(format!(
                                "Extension cardinality {} may not match intended usage",
                                cardinality
                            ))
                            .label(ext_rule.syntax().text_range(), "unusual cardinality")
                            .note(format!(
                                "Extension '{}' is typically used with cardinality {}",
                                extension.name(),
                                suggested_card
                            ))
                            .help("Verify this cardinality matches your use case")
                            .build()
                    );
                }
            }
        }
    }

    diagnostics
}
```

## Implementation Location

**File**: `crates/maki-rules/src/builtin/profile.rs`

Create comprehensive profile consistency checking module.

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_type_constraint_conflicts() {
    let source = r#"
        Profile: BadTypes
        Parent: Observation
        * value[x] only boolean
    "#;

    let diagnostics = lint_source_with_fhir(source, &["type-constraint-conflicts"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("incompatible"));
}

#[test]
fn test_reference_target_validation() {
    let source = r#"
        Profile: BadReference
        Parent: Observation
        * subject only Reference(NonExistentResource)
    "#;

    let diagnostics = lint_source(source, &["reference-target-validation"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("does not exist"));
}

#[test]
fn test_slice_name_collision() {
    let source = r#"
        Profile: SliceCollision
        Parent: Patient
        * identifier contains code 1..1
    "#;

    let diagnostics = lint_source(source, &["slice-name-collision"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("collision"));
}

#[test]
fn test_must_support_propagation() {
    let source = r#"
        Profile: InconsistentMS
        Parent: Patient
        * name 1..* MS
        * name.family 1..1
    "#;

    let diagnostics = lint_source(source, &["must-support-propagation"]);
    assert!(diagnostics.len() > 0);
    assert!(diagnostics[0].severity == Severity::Warning);
}
```

## Configuration

```toml
[rules.type-constraint-conflicts]
enabled = true
severity = "error"

[rules.reference-target-validation]
enabled = true
severity = "error"

[rules.slice-name-collision]
enabled = true
severity = "error"

[rules.must-support-propagation]
enabled = true
severity = "warning"
autofix = true

[rules.must-support-propagation.config]
# Check children up to N levels deep
max_depth = 2

[rules.extension-cardinality]
enabled = true
severity = "warning"
```

## Dependencies

### Required Components
- **FHIR Definitions** (Task 29): For type hierarchies and element definitions
- **Semantic Analyzer**: For profile inheritance analysis
- **Workspace**: For cross-profile reference validation

## Acceptance Criteria

- [ ] `type-constraint-conflicts` validates type constraints against parent
- [ ] `reference-target-validation` checks all reference targets exist
- [ ] `slice-name-collision` detects naming conflicts
- [ ] `must-support-propagation` suggests MS for child elements
- [ ] `extension-cardinality` warns about unusual cardinality patterns
- [ ] All rules integrate with FHIR definitions
- [ ] Configuration supports customization
- [ ] Unit tests cover all scenarios
- [ ] Performance: <10ms per profile

## Related Tasks

- **Task 32: Cardinality Validation** - Works together with these consistency checks
- **Task 33: Binding Strength** - Similar semantic validation approach
- **Task 34: Required Fields** - Checks related constraints

---

**Status**: Ready for implementation
**Estimated Complexity**: High (requires deep FHIR knowledge)
**Priority**: Medium (improves profile quality)
**Updated**: 2025-11-03
