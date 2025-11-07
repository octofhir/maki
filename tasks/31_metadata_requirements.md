# Task 31: Metadata Requirements Rules

**Phase**: 2 (Enhanced Linter - Week 9)
**Time Estimate**: 2-3 days
**Status**: 📝 Planned
**Priority**: Critical
**Dependencies**: Tasks 01-29 (Core linting infrastructure), Task 30 (Naming conventions for ID generation)

## Overview

Implement critical metadata requirement rules for FSH definitions to ensure all entities have required metadata fields (Parent, Id, Title, Description). These are **blocking rules** that must pass before other rules run, as they validate fundamental requirements for FHIR resource definitions.

**Part of Enhanced Linter Phase**: These are among the first 10 critical rules (errors) being implemented for Week 9.

## Context

FHIR StructureDefinitions and other conformance resources require specific metadata fields for proper publication and interoperability:
- **Parent**: Required for Profiles to specify base resource
- **Id**: Canonical identifier used in URLs and references
- **Title**: Human-readable name for display
- **Description**: Documentation for users

SUSHI allows FSH files without these fields, leading to compilation errors or incomplete resources. MAKI enforces these requirements with helpful autofixes.

## Goals

1. **Implement required-parent rule** - Ensure all Profiles have Parent keyword
2. **Implement required-id rule** - Ensure all entities have Id with autofix from Name
3. **Implement required-title rule** - Ensure all entities have Title with autofix from Name
4. **Implement required-description rule** - Ensure all entities have Description with placeholder generation
5. **Provide intelligent autofixes** - Generate metadata from entity Name using naming conventions
6. **Configure severity levels** - Errors for Parent/Id/Title, Warning for Description

## Technical Specification

### Rule 1: `required-parent`

**Category**: Blocking (Correctness)
**Severity**: Error
**Autofix**: No (requires user decision on which base resource to use)

**Implementation (Rust - CST-based):**
```rust
use maki_core::cst::ast::{Document, Profile, AstNode};
use maki_core::semantic::SemanticModel;
use maki_core::diagnostic::{Diagnostic, Severity};

/// Check that all Profiles have a Parent keyword
pub fn check_required_parent(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    for profile in document.profiles() {
        if profile.parent().is_none() {
            let location = model.source_map.node_to_diagnostic_location(profile.syntax());

            diagnostics.push(
                Diagnostic::new(
                    "required-parent",
                    Severity::Error,
                    "Profile must specify a Parent".to_string(),
                    location,
                )
                .with_note("All Profiles must inherit from a base FHIR resource or another profile")
                .with_help("Add 'Parent: <ResourceType>' after the Profile declaration")
            );
        }
    }

    diagnostics
}
```

**GritQL Alternative:**
```gritql
Profile where {
    not parent
}
```

**Example:**
```fsh
// ❌ Missing Parent
Profile: MyPatientProfile
Id: my-patient-profile
Title: "My Patient Profile"

// ✅ Has Parent
Profile: MyPatientProfile
Parent: Patient  // Required!
Id: my-patient-profile
Title: "My Patient Profile"
```

### Rule 2: `required-id`

**Category**: Blocking (Correctness)
**Severity**: Error
**Autofix**: Yes (generate from Name using kebab-case)

**Implementation (Rust - CST-based):**
```rust
use maki_core::cst::ast::{Document, Profile, Extension, AstNode};
use maki_core::semantic::SemanticModel;
use maki_core::diagnostic::{Diagnostic, Severity};
use maki_core::autofix::CodeSuggestion;

/// Check that entities have an Id, generate from Name if missing
pub fn check_required_id(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        if profile.id().is_none() {
            // Generate ID from Name using kebab-case conversion
            let generated_id = if let Some(name_node) = profile.name() {
                NamingStyle::KebabCase.convert(&name_node.text().to_string())
            } else {
                "unnamed-profile".to_string()
            };

            let location = model.source_map.node_to_diagnostic_location(profile.syntax());

            diagnostics.push(
                Diagnostic::new(
                    "required-id",
                    Severity::Error,
                    "Profile must have an Id".to_string(),
                    location.clone(),
                )
                .with_note("All FSH entities require a unique identifier (Id)")
                .with_help(format!("Add 'Id: {}' to this profile", generated_id))
                .with_suggestion(
                    CodeSuggestion::safe_fix(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        location,
                    )
                )
            );
        }
    }

    // Check extensions (same pattern)
    for extension in document.extensions() {
        if extension.id().is_none() {
            let generated_id = if let Some(name_node) = extension.name() {
                NamingStyle::KebabCase.convert(&name_node.text().to_string())
            } else {
                "unnamed-extension".to_string()
            };

            let location = model.source_map.node_to_diagnostic_location(extension.syntax());

            diagnostics.push(
                Diagnostic::new(
                    "required-id",
                    Severity::Error,
                    "Extension must have an Id".to_string(),
                    location.clone(),
                )
                .with_suggestion(
                    CodeSuggestion::safe_fix(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
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
or {
    Profile where { not id },
    Extension where { not id },
    ValueSet where { not id }
} => {
    insert("Id: {{to_kebab_case(name)}}")
}
```

**Configurable Template Hooks** (from issue #1570):
```toml
# .makirc.toml
[rules.required-id]
# Template for generating IDs from names
id_template = "{{org}}-{{name}}"  # e.g., "acme-patient-profile"
org_prefix = "acme"                # Organization prefix

# Custom transform function
transform = "kebab-case"           # Options: kebab-case, snake_case, etc.
```

**Example:**
```fsh
// ❌ Missing Id
Profile: MyPatientProfile
Parent: Patient
Title: "My Patient Profile"

// After autofix ✅
Profile: MyPatientProfile
Parent: Patient
Id: my-patient-profile  // Generated from Name
Title: "My Patient Profile"
```

### Rule 3: `required-title`

**Category**: Blocking (Correctness)
**Severity**: Error
**Autofix**: Yes (copy from Name with spaces)

**Implementation:**
```rust
/// Check that entity has a Title, generate from Name if missing
pub fn check_required_title(entity: &dyn Entity) -> Option<Diagnostic> {
    if entity.title().is_none() {
        // Generate Title from Name by adding spaces before capitals
        let generated_title = entity.name()
            .map(|n| add_spaces_to_pascal_case(n.value()))
            .unwrap_or_else(|| "Untitled Entity".to_string());

        let insert_pos = find_metadata_insertion_point(entity);

        Some(
            DiagnosticBuilder::new(Severity::Error)
                .message("Entity must have a Title")
                .label(entity.syntax().text_range(), "missing Title keyword")
                .note("All FSH entities require a human-readable title")
                .help(format!("Add 'Title: \"{}\"' to this entity", generated_title))
                .autofix(Fix {
                    range: TextRange::new(insert_pos, insert_pos),
                    replacement: format!("\nTitle: \"{}\"", generated_title),
                    safety: FixSafety::Safe,
                    description: format!("Add Title: \"{}\"", generated_title),
                })
                .build()
        )
    } else {
        None
    }
}

/// Convert PascalCase to "Pascal Case" with spaces
fn add_spaces_to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;

    for c in s.chars() {
        if c.is_uppercase() && prev_was_lower {
            result.push(' ');
        }
        result.push(c);
        prev_was_lower = c.is_lowercase();
    }

    result
}
```

**Example:**
```fsh
// ❌ Missing Title
Profile: MyPatientProfile
Parent: Patient
Id: my-patient-profile

// After autofix ✅
Profile: MyPatientProfile
Parent: Patient
Id: my-patient-profile
Title: "My Patient Profile"  // Generated from Name
```

### Rule 4: `required-description`

**Category**: Best Practice
**Severity**: Warning (not blocking)
**Autofix**: Yes (generate placeholder with TODO)

**Implementation:**
```rust
/// Check that entity has a Description, generate placeholder if missing
pub fn check_required_description(entity: &dyn Entity) -> Option<Diagnostic> {
    if entity.description().is_none() {
        let entity_type = entity.entity_type(); // Profile, Extension, etc.
        let name = entity.name().map(|n| n.value()).unwrap_or("this entity");

        let placeholder = format!(
            "TODO: Add description for {} {}",
            entity_type,
            name
        );

        let insert_pos = find_metadata_insertion_point(entity);

        Some(
            DiagnosticBuilder::new(Severity::Warning)
                .message("Entity should have a Description")
                .label(entity.syntax().text_range(), "missing Description keyword")
                .note("Descriptions help users understand the purpose of this entity")
                .help("Add a Description explaining when and how to use this entity")
                .autofix(Fix {
                    range: TextRange::new(insert_pos, insert_pos),
                    replacement: format!("\nDescription: \"{}\"", placeholder),
                    safety: FixSafety::Safe,
                    description: "Add placeholder Description",
                })
                .build()
        )
    } else {
        None
    }
}
```

**Example:**
```fsh
// ❌ Missing Description
Profile: MyPatientProfile
Parent: Patient
Id: my-patient-profile
Title: "My Patient Profile"

// After autofix ✅
Profile: MyPatientProfile
Parent: Patient
Id: my-patient-profile
Title: "My Patient Profile"
Description: "TODO: Add description for Profile MyPatientProfile"  // Placeholder
```

## Implementation Location

**File**: `crates/maki-rules/src/builtin/metadata.rs`

This file already exists with basic structure. Extend it with the four rules above.

## Testing Requirements

### Unit Tests

**File**: `crates/maki-rules/tests/metadata_test.rs`

```rust
#[test]
fn test_required_parent_detects_missing() {
    let source = r#"
        Profile: MyProfile
        Id: my-profile
    "#;

    let diagnostics = lint_source(source, &["required-parent"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Parent"));
    assert_eq!(diagnostics[0].severity, Severity::Error);
}

#[test]
fn test_required_id_autofix() {
    let source = r#"
        Profile: MyPatientProfile
        Parent: Patient
    "#;

    let fixed = apply_autofixes(source, &["required-id"]);
    assert!(fixed.contains("Id: my-patient-profile"));
}

#[test]
fn test_required_title_generation() {
    let source = r#"
        Profile: MyPatientProfile
        Parent: Patient
        Id: my-patient-profile
    "#;

    let fixed = apply_autofixes(source, &["required-title"]);
    assert!(fixed.contains("Title: \"My Patient Profile\""));
}

#[test]
fn test_required_description_placeholder() {
    let source = r#"
        Profile: MyPatientProfile
        Parent: Patient
        Id: my-patient-profile
        Title: "My Patient Profile"
    "#;

    let fixed = apply_autofixes(source, &["required-description"]);
    assert!(fixed.contains("Description:"));
    assert!(fixed.contains("TODO"));
}

#[test]
fn test_all_metadata_complete_no_errors() {
    let source = r#"
        Profile: MyPatientProfile
        Parent: Patient
        Id: my-patient-profile
        Title: "My Patient Profile"
        Description: "A profile for patient demographics"
    "#;

    let diagnostics = lint_source(source, &[
        "required-parent",
        "required-id",
        "required-title",
        "required-description"
    ]);

    assert_eq!(diagnostics.len(), 0);
}
```

### Integration Tests

```bash
# Test metadata rules
maki lint examples/metadata/ --filter required-parent,required-id,required-title,required-description

# Test autofixes
maki lint examples/metadata/ --fix

# Verify all metadata added
maki lint examples/metadata/
```

### Golden Files

**File**: `crates/maki-core/tests/golden_files/metadata_missing.fsh`

```fsh
// Missing various metadata fields

// Missing Parent
Profile: ProfileWithoutParent
Id: profile-without-parent
Title: "Profile Without Parent"

// Missing Id
Profile: ProfileWithoutId
Parent: Patient
Title: "Profile Without Id"

// Missing Title
Profile: ProfileWithoutTitle
Parent: Patient
Id: profile-without-title

// Missing Description
Profile: ProfileWithoutDescription
Parent: Patient
Id: profile-without-description
Title: "Profile Without Description"

// Complete metadata (should have no errors)
Profile: CompleteProfile
Parent: Patient
Id: complete-profile
Title: "Complete Profile"
Description: "A profile with all required metadata"
```

## Configuration

### Rule Configuration

```toml
[rules.required-parent]
enabled = true
severity = "error"
# No autofix available

[rules.required-id]
enabled = true
severity = "error"
autofix = true

[rules.required-id.config]
# Template for generating IDs
template = "{{name}}"      # Default: use name directly
transform = "kebab-case"   # Transform function
prefix = ""                # Optional prefix (e.g., "org-")

[rules.required-title]
enabled = true
severity = "error"
autofix = true

[rules.required-title.config]
# How to generate titles from names
add_spaces = true          # Convert "MyProfile" to "My Profile"

[rules.required-description]
enabled = true
severity = "warning"       # Warning, not error
autofix = true

[rules.required-description.config]
# Placeholder template
template = "TODO: Add description for {{type}} {{name}}"
```

### CLI Usage

```bash
# Check metadata requirements
maki lint --filter required-parent,required-id,required-title,required-description

# Auto-generate missing metadata
maki lint --fix

# Show rule details
maki rules --detailed required-id
```

## Edge Cases

1. **Entity without Name**: Use "unnamed-entity" as fallback for ID generation
2. **Insertion point**: Find correct location after entity declaration for metadata
3. **Existing partial metadata**: Don't duplicate existing fields
4. **Multiple entities**: Handle each entity independently
5. **Extension Parent**: Extensions don't require Parent (different rule)
6. **Instance Parent**: Instances use InstanceOf, not Parent

## Dependencies

### Required Components
- **CST/AST** (Task 03-04): For parsing entity definitions
- **Diagnostic System** (Task 06): For reporting violations
- **Autofix Engine**: For generating metadata
- **NamingStyle** (Task 30): For ID generation using kebab-case

### Integration Points
- **Task 30 (Naming Conventions)**: Uses NamingStyle for ID generation
- **Task 37 (Autofix Engine)**: Classifies these as safe fixes

## Acceptance Criteria

- [ ] `required-parent` detects missing Parent in Profiles
- [ ] `required-parent` shows Error severity
- [ ] `required-parent` provides helpful error message
- [ ] `required-id` detects missing Id in all entities
- [ ] `required-id` autofix generates kebab-case ID from Name
- [ ] `required-id` supports configurable templates (prefix, transform)
- [ ] `required-title` detects missing Title in all entities
- [ ] `required-title` autofix generates title with spaces from PascalCase Name
- [ ] `required-description` detects missing Description
- [ ] `required-description` shows Warning severity (not Error)
- [ ] `required-description` autofix generates TODO placeholder
- [ ] All rules work with Profile, Extension, ValueSet, CodeSystem, Instance
- [ ] Autofixes insert metadata at correct location
- [ ] Configuration file support for customization
- [ ] Unit tests pass for all rules
- [ ] Integration tests verify complete metadata generation
- [ ] Golden files demonstrate various missing metadata scenarios
- [ ] Performance: <1ms per entity

## Example: Complete Workflow

**Input** (missing everything):
```fsh
Profile: MyPatientProfile
```

**After `maki lint --fix`**:
```fsh
Profile: MyPatientProfile
Parent: ???  // ❌ Still needs manual input
Id: my-patient-profile
Title: "My Patient Profile"
Description: "TODO: Add description for Profile MyPatientProfile"
```

**User adds Parent**:
```fsh
Profile: MyPatientProfile
Parent: Patient  // User decision required
Id: my-patient-profile
Title: "My Patient Profile"
Description: "TODO: Add description for Profile MyPatientProfile"
```

**User updates Description**:
```fsh
Profile: MyPatientProfile
Parent: Patient
Id: my-patient-profile
Title: "My Patient Profile"
Description: "A profile for patient demographics in our system"  // ✅ Complete!
```

## Future Enhancements

1. **Smart Parent Suggestions**: Analyze profile content to suggest likely Parent resource
2. **Description Templates**: Entity-type-specific description templates
3. **Metadata Consistency**: Ensure Title/Description align with Id/Name
4. **Version Metadata**: Check for version, status, publisher in sushi-config.yaml

## Resources

- **FHIR StructureDefinition**: https://www.hl7.org/fhir/structuredefinition.html
- **FSH Specification**: https://hl7.org/fhir/uv/shorthand/
- **SUSHI Issue #1570**: ID template hooks

## Related Tasks

- **Task 30: Naming Conventions** - Provides NamingStyle for ID generation
- **Task 35: Duplicate Detection** - Checks for duplicate IDs after generation
- **Task 37: Autofix Engine** - Classifies metadata fixes as safe
- **Task 14: Short Description** - Validates description length

---

**Status**: Ready for implementation (already partially implemented in metadata.rs)
**Estimated Complexity**: Medium
**Priority**: Critical (blocking rules)
**Updated**: 2025-11-03
