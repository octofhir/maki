# Task 33: Binding Strength Rules

**Phase**: 2 (Enhanced Linter - Weeks 9-11)
**Time Estimate**: 2-3 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Tasks 01-29 (Core linting), Task 29 (FHIR Definitions)

## Overview

Implement validation rules for ValueSet binding strengths to ensure that bindings are properly specified and that child profile bindings don't weaken parent constraints. This includes checking for missing binding strengths, detecting inconsistencies, and validating against parent profiles.

**Part of Enhanced Linter Phase**: Critical rule (#6 in Week 9), best practice (#17 in Week 10), and semantic validation (#23 in Week 11).

## Context

FHIR binding strengths control how strictly coded values must conform to a ValueSet:
- **required**: Must use a code from the ValueSet
- **extensible**: Should use a code from the ValueSet if applicable
- **preferred**: Suggested ValueSet, but not enforced
- **example**: Example ValueSet for guidance only

Common issues:
- **Missing strength**: Binding without specifying strength (e.g., `* gender from GenderValueSet`)
- **Weakening**: Child profile uses weaker strength than parent (e.g., parent `required`, child `preferred`)
- **Inconsistency**: Similar elements use different strengths without clear rationale

## Goals

1. **Implement binding-strength-required rule** - Ensure all bindings specify strength
2. **Implement binding-strength-weakening rule** - Prevent weakening parent bindings
3. **Implement binding-strength-inconsistent rule** - Detect suspicious inconsistencies
4. **Provide autofixes** - Add default strength (required) when missing
5. **Support configuration** - Allow customizing default strength

## Technical Specification

### Rule 1: `binding-strength-required` (Week 9 - Critical)

**Category**: Blocking (Correctness)
**Severity**: Error
**Autofix**: Yes (add default strength - typically `required`)

**Validation (Rust - CST-based):**
```rust
use maki_core::cst::ast::{Document, Profile, AstNode, BindingRule};
use maki_core::semantic::SemanticModel;
use maki_core::diagnostic::{Diagnostic, Severity};
use maki_core::autofix::CodeSuggestion;

pub fn check_binding_strength_required(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    for profile in document.profiles() {
        for binding in profile.binding_rules() {
            // Check if binding has strength specified
            if binding.strength().is_none() {
                let location = model.source_map.node_to_diagnostic_location(binding.syntax());

                diagnostics.push(
                    Diagnostic::new(
                        "binding-strength-required",
                        Severity::Error,
                        "Binding must specify strength (required, extensible, preferred, example)".to_string(),
                        location.clone(),
                    )
                    .with_note("ValueSet bindings require a strength modifier")
                    .with_help("Add strength: (required), (extensible), (preferred), or (example)")
                    .with_suggestion(
                        CodeSuggestion::unsafe_fix(
                            "Add (required) binding strength",
                            " (required)".to_string(),
                            location,
                        )
                    )
                );
            }
        }
    }

    diagnostics
}
```

**GritQL Alternative:**
```gritql
Profile: $p where {
    binding_without_strength($p)
} => {
    insert(" (required)")
}
```

**Examples:**
```fsh
// ❌ Missing binding strength
Profile: MyPatientProfile
Parent: Patient
* gender from AdministrativeGender  // Error: no strength

// After autofix ✅
* gender from AdministrativeGender (required)

// ✅ Valid bindings with strength
* gender from AdministrativeGender (required)
* maritalStatus from MaritalStatus (extensible)
* communication.language from LanguageCodes (preferred)
* identifier.use from IdentifierUse (example)
```

### Rule 2: `binding-strength-weakening` (Week 11 - Semantic)

**Category**: Correctness (Semantic)
**Severity**: Error
**Autofix**: No (requires understanding of use case)

**Requires**: Loaded FHIR definitions to check parent binding

**Strength Hierarchy** (from strongest to weakest):
1. required
2. extensible
3. preferred
4. example

**Validation:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BindingStrength {
    Required = 4,
    Extensible = 3,
    Preferred = 2,
    Example = 1,
}

pub fn check_binding_strength_weakening(
    profile: &Profile,
    element_path: &str,
    child_binding: &BindingRule,
    fhir_defs: &FhirDefinitions,
) -> Option<Diagnostic> {
    let child_strength = child_binding.strength()?;

    // Get parent binding from FHIR definitions
    let parent_binding = fhir_defs.get_element_binding(
        profile.parent()?.value(),
        element_path
    )?;

    // Check if child weakens parent binding
    if child_strength < parent_binding.strength {
        return Some(
            DiagnosticBuilder::new(Severity::Error)
                .message(format!(
                    "Binding strength ({}) is weaker than parent ({})",
                    child_strength.as_str(),
                    parent_binding.strength.as_str()
                ))
                .label(child_binding.syntax().text_range(), "weakened binding")
                .note(format!(
                    "Parent ({}) uses {} binding, child cannot weaken to {}",
                    profile.parent()?.value(),
                    parent_binding.strength.as_str(),
                    child_strength.as_str()
                ))
                .help("Child profiles can only strengthen bindings, not weaken them")
                .example(
                    "Parent: (required)    → Child: (extensible) ❌\n\
                     Parent: (extensible)  → Child: (required)    ✅\n\
                     Parent: (preferred)   → Child: (preferred)   ✅"
                )
                .build()
        );
    }

    // Also check if ValueSet is different (related but different issue)
    if child_binding.valueset().value() != parent_binding.valueset {
        return Some(
            DiagnosticBuilder::new(Severity::Warning)
                .message("Binding uses different ValueSet than parent")
                .label(child_binding.valueset_token().text_range(), "different ValueSet")
                .note(format!(
                    "Parent uses {}, child uses {}",
                    parent_binding.valueset,
                    child_binding.valueset().value()
                ))
                .help("Ensure the new ValueSet is appropriate for this context")
                .build()
        );
    }

    None
}
```

**Examples:**
```fsh
// ❌ Weakening binding strength
Profile: MyPatientProfile
Parent: USCorePatientProfile  // Uses gender (required)
* gender from AdministrativeGender (preferred)  // Error: weakens required to preferred

// ❌ Another weakening
Profile: MyObservationProfile
Parent: Observation  // Uses code (extensible)
* code from LoincCodes (example)  // Error: weakens extensible to example

// ✅ Valid: Strengthening binding
Profile: StrictPatientProfile
Parent: Patient  // Uses gender (extensible)
* gender from AdministrativeGender (required)  // OK: strengthens to required

// ✅ Valid: Same strength
Profile: MyPatientProfile
Parent: Patient
* gender from AdministrativeGender (required)  // OK: same as parent
```

### Rule 3: `binding-strength-inconsistent` (Week 10 - Warning)

**Category**: Best Practice
**Severity**: Info (suggestion)
**Autofix**: No (requires domain analysis)

**Validation:**
```rust
pub fn check_binding_strength_inconsistent(
    profile: &Profile,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let bindings = collect_bindings(profile);

    // Group bindings by element type or naming pattern
    let groups = group_similar_elements(&bindings);

    for (pattern, elements) in groups {
        let strengths: Vec<_> = elements.iter()
            .map(|e| e.binding().and_then(|b| b.strength()))
            .collect();

        // Check if similar elements use different strengths
        if has_multiple_strengths(&strengths) {
            diagnostics.push(
                DiagnosticBuilder::new(Severity::Info)
                    .message(format!("Inconsistent binding strengths for {} elements", pattern))
                    .labels(elements.iter().map(|e| {
                        (e.syntax().text_range(), format!("uses {}", e.binding()?.strength()?.as_str()))
                    }))
                    .note("Similar elements typically use consistent binding strengths")
                    .help("Consider if these elements should use the same binding strength")
                    .build()
            );
        }
    }

    diagnostics
}

fn group_similar_elements(bindings: &[ElementBinding]) -> HashMap<String, Vec<ElementBinding>> {
    let mut groups = HashMap::new();

    for binding in bindings {
        // Group by element name patterns (e.g., all "code" elements)
        let pattern = extract_element_pattern(binding.path());
        groups.entry(pattern).or_insert_with(Vec::new).push(binding.clone());
    }

    groups
}
```

**Example:**
```fsh
// ⚠️ Inconsistent binding strengths
Profile: MyObservationProfile
Parent: Observation
* code from LoincCodes (required)           // required
* component.code from SnomedCodes (preferred) // preferred
* category from ObservationCategories (example) // example

// Note: All "code" elements use different strengths - is this intentional?
```

### Rule 4: `binding-without-valueset` (Additional)

**Category**: Correctness
**Severity**: Error
**Autofix**: No

**Validation:**
```rust
pub fn check_binding_without_valueset(
    binding: &BindingRule,
    workspace: &Workspace,
) -> Option<Diagnostic> {
    let valueset_name = binding.valueset().value();

    // Check if ValueSet exists in FHIR definitions or project
    if !workspace.has_valueset(valueset_name) {
        return Some(
            DiagnosticBuilder::new(Severity::Error)
                .message(format!("Binding references non-existent ValueSet '{}'", valueset_name))
                .label(binding.valueset_token().text_range(), "undefined ValueSet")
                .note("ValueSet must be defined in FHIR spec or in this project")
                .help(format!("Define ValueSet {} or check for typos", valueset_name))
                .build()
        );
    }

    None
}
```

## Implementation Location

**File**: `crates/maki-rules/src/builtin/binding.rs`

Create new file for binding-specific rules.

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_binding_strength_required_detects_missing() {
    let source = r#"
        Profile: MyProfile
        Parent: Patient
        * gender from AdministrativeGender
    "#;

    let diagnostics = lint_source(source, &["binding-strength-required"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("strength"));
}

#[test]
fn test_autofix_adds_required_strength() {
    let source = "* gender from AdministrativeGender";
    let fixed = apply_autofixes(source, &["binding-strength-required"]);
    assert!(fixed.contains("(required)"));
}

#[test]
fn test_binding_strength_weakening_detects() {
    let source = r#"
        Profile: MyProfile
        Parent: USCorePatient  // gender is (required)
        * gender from GenderValueSet (preferred)
    "#;

    let diagnostics = lint_source_with_fhir(source, &["binding-strength-weakening"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("weaker"));
}

#[test]
fn test_binding_strength_comparison() {
    assert!(BindingStrength::Required > BindingStrength::Extensible);
    assert!(BindingStrength::Extensible > BindingStrength::Preferred);
    assert!(BindingStrength::Preferred > BindingStrength::Example);
}

#[test]
fn test_strengthening_binding_allowed() {
    let source = r#"
        Profile: StrictProfile
        Parent: Patient  // gender is (extensible)
        * gender from GenderValueSet (required)
    "#;

    let diagnostics = lint_source_with_fhir(source, &["binding-strength-weakening"]);
    assert_eq!(diagnostics.len(), 0);
}
```

### Integration Tests

```bash
# Test binding strength validation
maki lint examples/bindings/ --filter binding-strength-required,binding-strength-weakening

# Apply fixes for missing strengths
maki lint examples/bindings/ --fix-unsafe  # Unsafe because adds semantic constraint

# Verify no violations
maki lint examples/bindings/
```

### Golden Files

**File**: `crates/maki-core/tests/golden_files/binding_errors.fsh`

```fsh
// Binding strength validation errors

// Missing binding strength
Profile: MissingBindingStrength
Parent: Patient
* gender from AdministrativeGender  // No strength specified

// Weakening parent binding
Profile: WeakenedBinding
Parent: USCorePatient  // Has required binding
* gender from GenderValueSet (preferred)  // Weakens to preferred

// Inconsistent strengths
Profile: InconsistentBindings
Parent: Observation
* code from LoincCodes (required)
* component.code from SnomedCodes (example)  // Different strength

// Non-existent ValueSet
Profile: InvalidValueSet
Parent: Patient
* gender from NonExistentValueSet (required)

// Valid bindings
Profile: ValidBindings
Parent: Patient
* gender from AdministrativeGender (required)
* maritalStatus from MaritalStatus (extensible)
```

## Configuration

```toml
[rules.binding-strength-required]
enabled = true
severity = "error"
autofix = true

[rules.binding-strength-required.config]
# Default strength to add when missing
default_strength = "required"  # Options: required, extensible, preferred, example

[rules.binding-strength-weakening]
enabled = true
severity = "error"
# Requires FHIR definitions

[rules.binding-strength-inconsistent]
enabled = true
severity = "info"

[rules.binding-strength-inconsistent.config]
# Only check elements with same name pattern
check_same_name_only = true

[rules.binding-without-valueset]
enabled = true
severity = "error"
```

## CLI Usage

```bash
# Check binding strength rules
maki lint --filter binding-strength-required,binding-strength-weakening

# Add missing strengths (unsafe fix)
maki lint --fix-unsafe

# Show rule details
maki rules --detailed binding-strength-required
```

## Dependencies

### Required Components
- **CST/AST** (Tasks 03-04): For parsing binding rules
- **FHIR Definitions** (Task 29): For parent binding lookup
- **Semantic Analyzer**: For ValueSet resolution

## Acceptance Criteria

- [ ] `binding-strength-required` detects missing strength specifications
- [ ] `binding-strength-required` autofix adds default strength (configurable)
- [ ] `BindingStrength` enum with proper ordering (Required > Extensible > Preferred > Example)
- [ ] `binding-strength-weakening` detects child weakening parent binding
- [ ] `binding-strength-weakening` shows clear diagnostic with parent info
- [ ] `binding-strength-weakening` allows strengthening (required is OK for extensible parent)
- [ ] `binding-strength-inconsistent` detects patterns of inconsistency
- [ ] `binding-strength-inconsistent` groups similar elements
- [ ] `binding-without-valueset` detects undefined ValueSets
- [ ] Configuration supports custom default strength
- [ ] Unit tests cover all strength combinations
- [ ] Integration tests verify with FHIR definitions
- [ ] Performance: <2ms per binding

## Edge Cases

1. **No parent binding**: Element doesn't have binding in parent - child can add any strength
2. **Multiple bindings**: Element has multiple slices with different bindings
3. **Extension bindings**: Extensions define their own bindings
4. **Same strength, different ValueSet**: Allowed but warning recommended

## Future Enhancements

1. **Smart strength suggestions**: Analyze element type to suggest appropriate strength
2. **ValueSet compatibility**: Check if new ValueSet is compatible with parent
3. **Binding inheritance**: Track binding changes through multiple inheritance levels

## Resources

- **FHIR Binding Strengths**: https://www.hl7.org/fhir/terminologies.html#strength
- **FSH Binding Syntax**: https://hl7.org/fhir/uv/shorthand/reference.html#binding-rules

## Related Tasks

- **Task 32: Cardinality Validation** - Similar parent/child validation logic
- **Task 48: ValueSet Expansion** - Validates ValueSet availability
- **Task 36: Profile Consistency** - Uses binding validation

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium-High (requires FHIR definitions)
**Priority**: High (critical + semantic rules)
**Updated**: 2025-11-03
