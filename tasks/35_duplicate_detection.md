# Task 35: Duplicate Detection Rules

**Phase**: 2 (Enhanced Linter - Week 9)
**Time Estimate**: 1-2 days
**Status**: 📝 Planned
**Priority**: Critical
**Dependencies**: Tasks 01-29 (Core linting), Task 31 (Metadata - ID/Name validation)

## Overview

Implement comprehensive duplicate detection rules to identify conflicting entity names, IDs, and rule definitions across FSH projects. Duplicates cause compilation errors or unexpected behavior, so early detection is critical.

**Part of Enhanced Linter Phase**: Critical error-level rules (#7 and #8 in Week 9) for project-wide uniqueness checks.

## Context

FSH projects must have unique identifiers across multiple dimensions:
- **Entity Names**: Profile names, Extension names, ValueSet names, etc.
- **Entity IDs**: Canonical identifiers used in URLs
- **Rule paths**: Cannot define same element path twice with conflicting values
- **Alias names**: Alias definitions must be unique

Common issues:
- Copy-paste errors (duplicate entity definitions)
- Multiple files defining same entity
- Conflicting rule definitions in same profile
- Reused alias names with different meanings

## Goals

1. **Implement duplicate-entity-name rule** - Detect duplicate entity names
2. **Implement duplicate-entity-id rule** - Detect duplicate entity IDs
3. **Implement duplicate-rule rule** - Detect conflicting rules within profiles
4. **Implement duplicate-alias rule** - Detect duplicate alias definitions
5. **Provide clear diagnostics** - Show all occurrences of duplicates

## Technical Specification

### Rule 1: `duplicate-entity-name` (Week 9 - Critical)

**Category**: Blocking (Correctness)
**Severity**: Error
**Autofix**: No (requires user decision on which to keep)

**Validation (Rust - CST-based, single file):**
```rust
use maki_core::cst::ast::{Document, AstNode};
use maki_core::semantic::SemanticModel;
use maki_core::diagnostic::{Diagnostic, Severity};
use std::collections::HashMap;

pub fn check_duplicate_entity_names(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_names: HashMap<String, Vec<String>> = HashMap::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Collect all entity names
    let mut entities_with_names = Vec::new();

    for profile in document.profiles() {
        if let Some(name_node) = profile.name() {
            let name_text = name_node.text().to_string();
            entities_with_names.push((name_text, name_node.clone(), "Profile"));
        }
    }

    for extension in document.extensions() {
        if let Some(name_node) = extension.name() {
            let name_text = name_node.text().to_string();
            entities_with_names.push((name_text, name_node.clone(), "Extension"));
        }
    }

    for valueset in document.valuesets() {
        if let Some(name_node) = valueset.name() {
            let name_text = name_node.text().to_string();
            entities_with_names.push((name_text, name_node.clone(), "ValueSet"));
        }
    }

    // Find duplicates
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for (name, _, _) in &entities_with_names {
        *name_counts.entry(name.clone()).or_insert(0) += 1;
    }

    // Report duplicates
    for (name, node, entity_type) in entities_with_names {
        if name_counts[&name] > 1 {
            let location = model.source_map.node_to_diagnostic_location(node.syntax());

            diagnostics.push(
                Diagnostic::new(
                    "duplicate-entity-name",
                    Severity::Error,
                    format!("Duplicate {} name '{}'", entity_type, name),
                    location,
                )
                .with_note(format!("This name is defined {} times in this file", name_counts[&name]))
                .with_help("Rename one of the definitions to make names unique")
            );
        }
    }

    diagnostics
}
```

**GritQL Alternative (cross-file requires workspace context):**
```gritql
or {
    Profile: $p1,
    Profile: $p2
} where {
    $p1.name == $p2.name and
    $p1 != $p2
}
```

**Workspace Integration (for cross-file duplicates):**
```rust
// For cross-file duplicate detection, iterate over multiple files
pub fn check_duplicate_entity_names_workspace(
    models: &[SemanticModel],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_names: HashMap<String, Vec<(String, DiagnosticLocation)>> = HashMap::new();

    // Collect all names from all files
    for model in models {
        let diagnostics_in_file = check_duplicate_entity_names(model);
        // Additionally track across files...
        // This is a simplified example - actual implementation would be more complex
    }

    diagnostics
}
                    .note(format!("Entity name '{}' is defined {} times", name, occurrences.len()))
                    .help("Each entity must have a unique name - remove or rename duplicates")
                    .build()
            );
        }
    }

    diagnostics
}
```

**Examples:**
```fsh
// File: profiles/Patient.fsh
Profile: MyPatientProfile
Parent: Patient
Id: my-patient-profile

// File: profiles/MorePatients.fsh
Profile: MyPatientProfile  // ❌ Error: Duplicate name!
Parent: Patient
Id: my-patient-profile-2

// Diagnostic:
// error: Duplicate entity name 'MyPatientProfile'
//   --> profiles/Patient.fsh:1:10
//    |
//  1 | Profile: MyPatientProfile
//    |          ^^^^^^^^^^^^^^^^ first definition
//    |
//   --> profiles/MorePatients.fsh:1:10
//    |
//  1 | Profile: MyPatientProfile
//    |          ^^^^^^^^^^^^^^^^ duplicate definition (2)
//    |
//    = note: Entity name 'MyPatientProfile' is defined 2 times
//    = help: Each entity must have a unique name - remove or rename duplicates
```

### Rule 2: `duplicate-entity-id` (Week 9 - Critical)

**Category**: Blocking (Correctness)
**Severity**: Error
**Autofix**: No (requires user decision)

**Validation:**
```rust
pub fn check_duplicate_entity_ids(workspace: &Workspace) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_ids: HashMap<String, Vec<(EntityRef, TextRange)>> = HashMap::new();

    // Collect all entity IDs across all files
    for file in workspace.files() {
        for entity in file.entities() {
            if let Some(id) = entity.id() {
                let id_str = id.value().to_string();
                seen_ids
                    .entry(id_str.clone())
                    .or_default()
                    .push((entity.clone(), id.text_range()));
            }
        }
    }

    // Check for duplicates
    for (id, occurrences) in seen_ids {
        if occurrences.len() > 1 {
            let mut labels = Vec::new();
            for (i, (entity, range)) in occurrences.iter().enumerate() {
                let label_text = if i == 0 {
                    format!("first use ({})", entity.name().map(|n| n.value()).unwrap_or("unnamed"))
                } else {
                    format!("duplicate ({})", entity.name().map(|n| n.value()).unwrap_or("unnamed"))
                };
                labels.push((range.clone(), label_text));
            }

            diagnostics.push(
                DiagnosticBuilder::new(Severity::Error)
                    .message(format!("Duplicate entity ID '{}'", id))
                    .labels(labels)
                    .note(format!("ID '{}' is used by {} entities", id, occurrences.len()))
                    .help("Each entity must have a unique ID - IDs become part of canonical URLs")
                    .build()
            );
        }
    }

    diagnostics
}
```

**Examples:**
```fsh
// File: profiles/Patient1.fsh
Profile: FirstPatientProfile
Parent: Patient
Id: patient-profile  // First use

// File: profiles/Patient2.fsh
Profile: SecondPatientProfile
Parent: Patient
Id: patient-profile  // ❌ Error: Duplicate ID!

// Diagnostic:
// error: Duplicate entity ID 'patient-profile'
//   --> profiles/Patient1.fsh:3:5
//    |
//  3 | Id: patient-profile
//    |     ^^^^^^^^^^^^^^^ first use (FirstPatientProfile)
//    |
//   --> profiles/Patient2.fsh:3:5
//    |
//  3 | Id: patient-profile
//    |     ^^^^^^^^^^^^^^^ duplicate (SecondPatientProfile)
//    |
//    = note: ID 'patient-profile' is used by 2 entities
//    = help: Each entity must have a unique ID - IDs become part of canonical URLs
```

### Rule 3: `duplicate-rule` (Additional Validation)

**Category**: Correctness
**Severity**: Error
**Autofix**: No (requires resolution of conflict)

**Validation:**
```rust
pub fn check_duplicate_rules(profile: &Profile) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_paths: HashMap<String, Vec<(Rule, TextRange)>> = HashMap::new();

    // Collect all rule paths in this profile
    for rule in profile.rules() {
        if let Some(path) = rule.path() {
            seen_paths
                .entry(path.to_string())
                .or_default()
                .push((rule.clone(), rule.syntax().text_range()));
        }
    }

    // Check for conflicting rules on same path
    for (path, rules) in seen_paths {
        // Check if rules conflict (different types or values)
        if has_conflicting_rules(&rules) {
            let mut labels = Vec::new();
            for (i, (rule, range)) in rules.iter().enumerate() {
                labels.push((
                    range.clone(),
                    if i == 0 {
                        format!("first rule: {}", describe_rule(rule))
                    } else {
                        format!("conflicts: {}", describe_rule(rule))
                    }
                ));
            }

            diagnostics.push(
                DiagnosticBuilder::new(Severity::Error)
                    .message(format!("Conflicting rules for path '{}'", path))
                    .labels(labels)
                    .note("Multiple rules define the same element with different constraints")
                    .help("Remove conflicting rules or combine them if possible")
                    .build()
            );
        }
    }

    diagnostics
}

fn has_conflicting_rules(rules: &[(Rule, TextRange)]) -> bool {
    if rules.len() <= 1 {
        return false;
    }

    // Check specific conflicts:
    // - Multiple cardinality rules with different values
    // - Multiple type constraints that conflict
    // - Multiple bindings with different ValueSets

    let cardinality_rules: Vec<_> = rules.iter()
        .filter_map(|(r, _)| r.as_cardinality())
        .collect();

    if cardinality_rules.len() > 1 {
        // Check if all have same cardinality
        let first = cardinality_rules[0];
        if cardinality_rules.iter().any(|r| r.cardinality() != first.cardinality()) {
            return true;
        }
    }

    // Add more conflict detection as needed
    false
}

fn describe_rule(rule: &Rule) -> String {
    match rule {
        Rule::Cardinality(c) => format!("cardinality {}", c.cardinality()),
        Rule::Type(t) => format!("type {}", t.type_name()),
        Rule::Binding(b) => format!("binding {}", b.valueset().value()),
        _ => "rule".to_string(),
    }
}
```

**Examples:**
```fsh
// ❌ Conflicting cardinality rules
Profile: ConflictingProfile
Parent: Patient
* name 1..*  // First rule
* name 0..1  // Error: Conflicts with above!

// ❌ Conflicting type constraints
Profile: TypeConflict
Parent: Observation
* value[x] only string
* value[x] only integer  // Error: Can't be both string and integer!

// ✅ Compatible rules (OK to combine)
Profile: CompatibleRules
Parent: Patient
* name 1..*    // Cardinality
* name MS      // Flags - compatible with cardinality
```

### Rule 4: `duplicate-alias` (Additional Validation)

**Category**: Correctness
**Severity**: Error
**Autofix**: No

**Validation:**
```rust
pub fn check_duplicate_aliases(file: &FshFile) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_aliases: HashMap<String, Vec<(Alias, TextRange)>> = HashMap::new();

    // Collect all aliases in file
    for alias in file.aliases() {
        let name = alias.name().value().to_string();
        seen_aliases
            .entry(name)
            .or_default()
            .push((alias.clone(), alias.syntax().text_range()));
    }

    // Check for duplicates
    for (name, occurrences) in seen_aliases {
        if occurrences.len() > 1 {
            // Check if all point to same value (OK) or different values (error)
            let values: Vec<_> = occurrences.iter()
                .map(|(a, _)| a.value().value())
                .collect();

            let all_same = values.windows(2).all(|w| w[0] == w[1]);

            if !all_same {
                diagnostics.push(
                    DiagnosticBuilder::new(Severity::Error)
                        .message(format!("Duplicate alias '{}' with different values", name))
                        .labels(occurrences.iter().enumerate().map(|(i, (a, range))| {
                            (range.clone(), format!("= {}", a.value().value()))
                        }))
                        .note("Alias name is used multiple times with different values")
                        .help("Remove duplicate or use different alias names")
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
// ❌ Duplicate aliases with different values
Alias: $SCT = http://snomed.info/sct
Alias: $SCT = http://different-url.org  // Error: Different value!

// ✅ Duplicate aliases with same value (OK, but redundant)
Alias: $SCT = http://snomed.info/sct
Alias: $SCT = http://snomed.info/sct  // Warning: Redundant but OK
```

## Implementation Location

**File**: `crates/maki-rules/src/builtin/duplicates.rs`

This file already exists with basic structure. Extend it with comprehensive duplicate detection.

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_duplicate_entity_names() {
    let source = r#"
        Profile: MyProfile
        Parent: Patient
        Id: my-profile-1

        Profile: MyProfile  // Duplicate name!
        Parent: Patient
        Id: my-profile-2
    "#;

    let diagnostics = lint_source(source, &["duplicate-entity-name"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Duplicate entity name"));
}

#[test]
fn test_duplicate_entity_ids() {
    let source = r#"
        Profile: FirstProfile
        Parent: Patient
        Id: same-id

        Profile: SecondProfile
        Parent: Patient
        Id: same-id  // Duplicate ID!
    "#;

    let diagnostics = lint_source(source, &["duplicate-entity-id"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Duplicate entity ID"));
}

#[test]
fn test_conflicting_cardinality_rules() {
    let source = r#"
        Profile: ConflictingProfile
        Parent: Patient
        * name 1..*
        * name 0..1
    "#;

    let diagnostics = lint_source(source, &["duplicate-rule"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Conflicting rules"));
}

#[test]
fn test_duplicate_aliases_different_values() {
    let source = r#"
        Alias: $SCT = http://snomed.info/sct
        Alias: $SCT = http://different-url.org
    "#;

    let diagnostics = lint_source(source, &["duplicate-alias"]);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("different values"));
}

#[test]
fn test_no_duplicates() {
    let source = r#"
        Profile: Profile1
        Parent: Patient
        Id: profile-1

        Profile: Profile2
        Parent: Patient
        Id: profile-2
    "#;

    let diagnostics = lint_source(source, &[
        "duplicate-entity-name",
        "duplicate-entity-id"
    ]);
    assert_eq!(diagnostics.len(), 0);
}
```

### Integration Tests

```bash
# Test duplicate detection across files
maki lint examples/duplicates/ --filter duplicate-entity-name,duplicate-entity-id

# Check specific project for duplicates
maki lint my-project/ --filter duplicate-entity-name,duplicate-entity-id,duplicate-rule
```

### Golden Files

**File**: `crates/maki-core/tests/golden_files/duplicates.fsh`

```fsh
// Various duplicate detection scenarios

// Duplicate entity names
Profile: DuplicateName
Parent: Patient
Id: first-id

Profile: DuplicateName  // Error!
Parent: Patient
Id: second-id

// Duplicate entity IDs
Profile: FirstProfile
Parent: Patient
Id: duplicate-id

Profile: SecondProfile
Parent: Patient
Id: duplicate-id  // Error!

// Conflicting rules
Profile: ConflictingRules
Parent: Patient
* name 1..*
* name 0..1  // Error: Conflicts!

// Duplicate aliases
Alias: $LOINC = http://loinc.org
Alias: $LOINC = http://different.org  // Error!

// Valid (no duplicates)
Profile: UniqueProfile1
Parent: Patient
Id: unique-1

Profile: UniqueProfile2
Parent: Patient
Id: unique-2
```

## Configuration

```toml
[rules.duplicate-entity-name]
enabled = true
severity = "error"
# No autofix - requires user decision

[rules.duplicate-entity-id]
enabled = true
severity = "error"
# No autofix - requires user decision

[rules.duplicate-rule]
enabled = true
severity = "error"

[rules.duplicate-rule.config]
# Allow multiple rules for same path if compatible
allow_compatible_rules = true

[rules.duplicate-alias]
enabled = true
severity = "error"

[rules.duplicate-alias.config]
# Warn (not error) if same value
warn_redundant_same_value = true
```

## CLI Usage

```bash
# Check for all duplicates
maki lint --filter duplicate-entity-name,duplicate-entity-id,duplicate-rule,duplicate-alias

# Scan entire project
maki lint .

# Show all occurrences
maki lint --verbose
```

## Dependencies

### Required Components
- **Workspace** (Task 11): For cross-file duplicate detection
- **CST/AST** (Tasks 03-04): For parsing entities
- **Symbol Table**: For tracking entity definitions

## Acceptance Criteria

- [ ] `duplicate-entity-name` detects duplicate names across all files
- [ ] `duplicate-entity-name` shows all occurrences with clear labels
- [ ] `duplicate-entity-id` detects duplicate IDs across all files
- [ ] `duplicate-entity-id` shows entity names for context
- [ ] `duplicate-rule` detects conflicting rules within profiles
- [ ] `duplicate-rule` distinguishes between conflicts and compatible rules
- [ ] `duplicate-alias` detects duplicate aliases with different values
- [ ] `duplicate-alias` allows (or warns about) same-value duplicates
- [ ] Diagnostics show all occurrences, not just pairs
- [ ] Configuration allows customizing behavior
- [ ] Unit tests cover all duplicate scenarios
- [ ] Integration tests work across multiple files
- [ ] Performance: <50ms for 100-file project

## Edge Cases

1. **Same name, different types**: Profile and ValueSet with same name (still error)
2. **Case sensitivity**: "MyProfile" vs "myprofile" (treat as different)
3. **Across directories**: Duplicates in different subdirectories
4. **Compatible rules**: Multiple MS flags, or flag + cardinality (OK)
5. **Imported duplicates**: Duplicates from included files

## Future Enhancements

1. **Smart deduplication**: Suggest which duplicate to keep based on usage
2. **Rename refactoring**: Automatically rename duplicates
3. **Duplicate prevention**: LSP warns while typing if name exists
4. **Cross-project detection**: Check for conflicts with imported packages

## Resources

- **FHIR Naming**: https://www.hl7.org/fhir/general.html#naming
- **FSH Specification**: https://hl7.org/fhir/uv/shorthand/

## Related Tasks

- **Task 31: Metadata Requirements** - Ensures ID exists before checking duplicates
- **Task 30: Naming Conventions** - Normalizes names before duplicate check
- **Task 70: SUSHI Migrator** - May need to resolve duplicates during migration

---

**Status**: Partially implemented (basic detection exists)
**Estimated Complexity**: Medium
**Priority**: Critical (blocking rule)
**Updated**: 2025-11-03
