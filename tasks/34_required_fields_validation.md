# Task 34: Required Fields Validation

**Phase**: 2 (Enhanced Linter - Weeks 9 & 11)
**Time Estimate**: 3 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Tasks 01-29 (Core linting), Task 29 (FHIR Definitions), Task 31 (Metadata requirements)

## Overview

Implement validation rules to ensure that required fields are present in both profile definitions and instance examples. This includes checking Extension context requirements, validating that instances provide all required elements, and detecting missing fields that would cause compilation or validation failures.

**Part of Enhanced Linter Phase**: Critical rule (#9 in Week 9) for Extensions and semantic validation for instances.

## Context

FHIR requires certain fields based on cardinality constraints:
- **Extensions**: Must specify `Context` keyword to define where they can be used
- **Instances**: Must provide values for all elements with minimum cardinality ≥ 1
- **Profiles**: Define which elements are required through cardinality rules

Common issues:
- Extensions without Context (compilation error)
- Instances missing required fields (validation failure)
- Profile constraints not satisfied by examples

## Goals

1. **Implement extension-context-required rule** - Ensure Extensions have Context
2. **Implement instance-required-fields rule** - Validate instances against their profiles
3. **Implement profile-without-examples rule** - Encourage example instances
4. **Provide autofixes** - Add common contexts, suggest missing fields
5. **Integration with FHIR validator** - Optionally use HAPI FHIR validator

## Technical Specification

### Rule 1: `extension-context-required` (Week 9 - Critical)

**Category**: Blocking (Correctness)
**Severity**: Error
**Autofix**: Yes (add common contexts with comment)

**Validation (Rust - CST-based):**
```rust
use maki_core::cst::ast::{Document, Extension, AstNode};
use maki_core::semantic::SemanticModel;
use maki_core::diagnostic::{Diagnostic, Severity};
use maki_core::autofix::CodeSuggestion;

pub fn check_extension_context_required(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    for extension in document.extensions() {
        // Check if extension has Context keyword
        if extension.context().is_none() {
            let extension_name = extension.name()
                .map(|n| n.text().to_string())
                .unwrap_or_else(|| "this extension".to_string());

            let location = model.source_map.node_to_diagnostic_location(extension.syntax());

            let suggested_context = "* ^context[+].type = #element\n\
                                    * ^context[=].expression = \"Patient\"  // TODO: Update";

            diagnostics.push(
                Diagnostic::new(
                    "extension-context-required",
                    Severity::Error,
                    "Extension must specify Context".to_string(),
                    location.clone(),
                )
                .with_note("Extensions require a Context to define where they can be used")
                .with_help("Add ^context rules to specify where this extension applies")
                .with_suggestion(
                    CodeSuggestion::unsafe_fix(
                        "Add Context keyword with TODO".to_string(),
                        format!("\n{}", suggested_context),
                        location,
                    )
                )
            );
        }
    }

    diagnostics
}
```

**GritQL Alternative:**
```gritql
Extension where {
    not context
}
```

**Examples:**
```fsh
// ❌ Extension without Context
Extension: BirthPlace
Description: "Place of birth"
* value[x] only Address

// After autofix ✅
Extension: BirthPlace
Description: "Place of birth"
* value[x] only Address
* ^context[+].type = #element
* ^context[=].expression = "Patient"  // TODO: Update to correct resource

// ✅ Extension with proper Context
Extension: Ethnicity
Description: "Patient ethnicity"
* value[x] only CodeableConcept
* ^context[+].type = #element
* ^context[=].expression = "Patient"
* ^context[+].type = #element
* ^context[=].expression = "RelatedPerson"
```

### Rule 2: `instance-required-fields` (Semantic Validation)

**Category**: Correctness
**Severity**: Error
**Autofix**: No (requires domain knowledge)

**Requires**: Profile definition with cardinality rules

**Validation:**
```rust
pub fn check_instance_required_fields(
    instance: &Instance,
    workspace: &Workspace,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Get the profile this instance conforms to
    let profile_name = instance.instance_of()?;
    let profile = workspace.get_profile(profile_name)?;

    // Collect required elements (min >= 1) from profile
    let required_elements = collect_required_elements(profile, workspace);

    // Check which required elements are missing in instance
    let provided_elements = collect_instance_assignments(instance);

    for required_elem in required_elements {
        if !provided_elements.contains(&required_elem.path) {
            diagnostics.push(
                DiagnosticBuilder::new(Severity::Error)
                    .message(format!(
                        "Instance missing required element '{}'",
                        required_elem.path
                    ))
                    .label(instance.syntax().text_range(), "incomplete instance")
                    .note(format!(
                        "Profile {} requires {} with cardinality {}",
                        profile_name,
                        required_elem.path,
                        required_elem.cardinality
                    ))
                    .help(format!(
                        "Add assignment: * {} = <value>",
                        required_elem.path
                    ))
                    .build()
            );
        }
    }

    diagnostics
}

/// Collect all required elements from profile and its parents
fn collect_required_elements(
    profile: &Profile,
    workspace: &Workspace,
) -> Vec<RequiredElement> {
    let mut required = Vec::new();

    // Get base resource required elements
    if let Some(parent) = profile.parent() {
        let base_required = workspace.fhir_defs()
            .get_required_elements(parent.value());
        required.extend(base_required);
    }

    // Add profile-specific required elements
    for rule in profile.rules() {
        if let Some(card_rule) = rule.as_cardinality() {
            if card_rule.min() >= 1 {
                required.push(RequiredElement {
                    path: card_rule.path().to_string(),
                    cardinality: format!("{}..{}", card_rule.min(), card_rule.max()),
                });
            }
        }
    }

    required
}

struct RequiredElement {
    path: String,
    cardinality: String,
}
```

**Examples:**
```fsh
// ❌ Instance missing required fields
Profile: MyPatientProfile
Parent: Patient
* name 1..*  // Required
* gender 1..1  // Required

Instance: InvalidPatientExample
InstanceOf: MyPatientProfile
// Missing name and gender!
* birthDate = "1990-01-01"

// Diagnostics:
// error: Instance missing required element 'name'
// error: Instance missing required element 'gender'

// ✅ Instance with all required fields
Instance: ValidPatientExample
InstanceOf: MyPatientProfile
* name.family = "Smith"
* name.given = "John"
* gender = #male
* birthDate = "1990-01-01"
```

### Rule 3: `profile-without-examples` (Week 10 - Warning)

**Category**: Best Practice (Documentation)
**Severity**: Warning
**Autofix**: No (requires creating examples)

**Validation:**
```rust
pub fn check_profile_without_examples(
    profile: &Profile,
    workspace: &Workspace,
) -> Option<Diagnostic> {
    let profile_name = profile.name()?.value();

    // Find instances that use this profile
    let instances = workspace.find_instances_of(profile_name);

    if instances.is_empty() {
        return Some(
            DiagnosticBuilder::new(Severity::Warning)
                .message(format!("Profile '{}' has no example instances", profile_name))
                .label(profile.syntax().text_range(), "no examples")
                .note("Profiles should have at least one example instance")
                .help(format!(
                    "Create an example:\n\
                     Instance: {}Example\n\
                     InstanceOf: {}\n\
                     * // Add required fields here",
                    profile_name, profile_name
                ))
                .build()
        );
    }

    None
}
```

**Example:**
```fsh
// ⚠️ Profile without examples
Profile: MySpecialPatientProfile
Parent: Patient
* name 1..*
* gender 1..1
// No instances defined!

// Should add:
Instance: MySpecialPatientExample
InstanceOf: MySpecialPatientProfile
Usage: #example
* name.family = "Doe"
* name.given = "Jane"
* gender = #female
```

### Rule 4: `required-field-override` (Additional Validation)

**Category**: Correctness
**Severity**: Warning
**Autofix**: No

**Validation:**
```rust
pub fn check_required_field_override(
    profile: &Profile,
    fhir_defs: &FhirDefinitions,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let parent = profile.parent()?.value();

    for rule in profile.rules() {
        if let Some(card_rule) = rule.as_cardinality() {
            let path = card_rule.path();

            // Check if making required field optional
            if let Some(parent_card) = fhir_defs.get_cardinality(parent, path) {
                if parent_card.min >= 1 && card_rule.min() == 0 {
                    diagnostics.push(
                        DiagnosticBuilder::new(Severity::Warning)
                            .message(format!(
                                "Making required element '{}' optional",
                                path
                            ))
                            .label(card_rule.syntax().text_range(), "relaxing requirement")
                            .note(format!(
                                "Parent requires {} ({}), child makes it optional ({})",
                                path,
                                parent_card,
                                card_rule.cardinality()
                            ))
                            .help("This may violate parent profile constraints")
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

**Files**:
- `crates/maki-rules/src/builtin/required_fields.rs` - Already exists
- Extend with Extension context checking and instance validation

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_extension_context_required() {
    let source = r#"
        Extension: MyExtension
        Description: "An extension"
        * value[x] only string
    "#;

    let diagnostics = lint_source(source, &["extension-context-required"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Context"));
}

#[test]
fn test_autofix_adds_context() {
    let source = r#"
        Extension: MyExtension
        * value[x] only string
    "#;

    let fixed = apply_autofixes(source, &["extension-context-required"]);
    assert!(fixed.contains("^context"));
    assert!(fixed.contains("TODO"));
}

#[test]
fn test_instance_missing_required_fields() {
    let source = r#"
        Profile: StrictPatient
        Parent: Patient
        * name 1..*
        * gender 1..1

        Instance: IncompleteExample
        InstanceOf: StrictPatient
        * birthDate = "1990-01-01"
    "#;

    let diagnostics = lint_source(source, &["instance-required-fields"]);
    assert!(diagnostics.len() >= 2);  // Missing name and gender
    assert!(diagnostics.iter().any(|d| d.message.contains("name")));
    assert!(diagnostics.iter().any(|d| d.message.contains("gender")));
}

#[test]
fn test_instance_with_all_required_fields() {
    let source = r#"
        Profile: StrictPatient
        Parent: Patient
        * name 1..*
        * gender 1..1

        Instance: CompleteExample
        InstanceOf: StrictPatient
        * name.family = "Smith"
        * gender = #male
    "#;

    let diagnostics = lint_source(source, &["instance-required-fields"]);
    assert_eq!(diagnostics.len(), 0);
}

#[test]
fn test_profile_without_examples_warns() {
    let source = r#"
        Profile: LonelyProfile
        Parent: Patient
        * name 1..*
        // No instances!
    "#;

    let diagnostics = lint_source(source, &["profile-without-examples"]);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Warning);
}
```

### Integration Tests

```bash
# Test required fields validation
maki lint examples/required-fields/ --filter extension-context-required,instance-required-fields

# Apply autofixes for extensions
maki lint examples/extensions/ --fix-unsafe

# Verify instances are complete
maki lint examples/instances/ --filter instance-required-fields
```

### Golden Files

**File**: `crates/maki-core/tests/golden_files/required_fields.fsh`

```fsh
// Required fields validation errors

// Extension without Context
Extension: NoContextExtension
Description: "Missing context"
* value[x] only string

// Instance missing required fields
Profile: RequiredFieldsProfile
Parent: Patient
* name 1..*
* gender 1..1
* birthDate 1..1

Instance: IncompleteInstance
InstanceOf: RequiredFieldsProfile
* name.family = "Doe"
// Missing gender and birthDate!

// Profile without examples (warning)
Profile: NoExamplesProfile
Parent: Observation
* status 1..1
* code 1..1
// No instances defined

// Complete valid example
Extension: ValidExtension
Description: "Has context"
* value[x] only CodeableConcept
* ^context[+].type = #element
* ^context[=].expression = "Patient"

Instance: CompleteInstance
InstanceOf: RequiredFieldsProfile
* name.family = "Doe"
* name.given = "John"
* gender = #male
* birthDate = "1990-01-01"
```

## Configuration

```toml
[rules.extension-context-required]
enabled = true
severity = "error"
autofix = true

[rules.extension-context-required.config]
# Default context to suggest
default_context_type = "element"
default_context_resource = "Patient"  # With TODO comment

[rules.instance-required-fields]
enabled = true
severity = "error"

[rules.instance-required-fields.config]
# Use HAPI FHIR validator for deep validation
use_hapi_validator = false

[rules.profile-without-examples]
enabled = true
severity = "warning"

[rules.profile-without-examples.config]
# Minimum number of examples per profile
min_examples = 1

[rules.required-field-override]
enabled = true
severity = "warning"
```

## CLI Usage

```bash
# Check required fields
maki lint --filter extension-context-required,instance-required-fields

# Add extension contexts
maki lint examples/extensions/ --fix-unsafe

# Validate instances deeply
maki lint examples/instances/ --filter instance-required-fields

# Show rule details
maki rules --detailed extension-context-required
```

## Dependencies

### Required Components
- **CST/AST** (Tasks 03-04): For parsing Extensions and Instances
- **FHIR Definitions** (Task 29): For base resource requirements
- **Semantic Analyzer**: For profile resolution

### Optional Integration
- **HAPI FHIR Validator**: For comprehensive instance validation

## Acceptance Criteria

- [ ] `extension-context-required` detects Extensions without Context
- [ ] `extension-context-required` autofix adds common context with TODO
- [ ] `instance-required-fields` checks instances against profile requirements
- [ ] `instance-required-fields` detects all missing required elements
- [ ] `instance-required-fields` handles inherited requirements from parent profiles
- [ ] `profile-without-examples` warns about profiles without instances
- [ ] `required-field-override` detects making required fields optional
- [ ] Configuration supports HAPI validator integration
- [ ] Unit tests cover extensions, instances, and profiles
- [ ] Integration tests verify complete validation
- [ ] Performance: <10ms per instance validation

## Edge Cases

1. **Abstract Extensions**: Extensions that are never used directly
2. **Multiple InstanceOf**: Instances conforming to multiple profiles
3. **Sliced elements**: Required slices vs required base element
4. **Choice types**: value[x] with multiple options
5. **Nested required**: Required parent with required children

## Future Enhancements

1. **Smart example generation**: Auto-generate skeleton instances from profiles
2. **FHIR Validator integration**: Deep validation using HAPI FHIR
3. **Context suggestions**: Analyze extension to suggest appropriate contexts
4. **Instance completion**: IntelliSense for missing required fields

## Resources

- **FHIR Extensions**: https://www.hl7.org/fhir/extensibility.html
- **FHIR Instances**: https://www.hl7.org/fhir/instance.html
- **FSH Instances**: https://hl7.org/fhir/uv/shorthand/reference.html#defining-instances

## Related Tasks

- **Task 31: Metadata Requirements** - Similar required field checking
- **Task 32: Cardinality Validation** - Validates cardinality constraints
- **Task 51: Test Runner** - Uses instance validation

---

**Status**: Partially implemented (basic checks exist)
**Estimated Complexity**: High (requires profile analysis)
**Priority**: High (critical for Extensions, important for instances)
**Updated**: 2025-11-03
