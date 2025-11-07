# Task 30: Naming Convention Rules

**Phase**: 2 (Enhanced Linter - Week 10)
**Time Estimate**: 2-3 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Tasks 01-29 (Core linting infrastructure)

## Overview

Implement comprehensive naming convention rules for FSH definitions to enforce consistent naming patterns across FHIR Implementation Guides. This task focuses on ensuring that IDs, Names, and entity identifiers follow established conventions (kebab-case for IDs, PascalCase for Names) with automatic fixes where possible.

**Part of Enhanced Linter Phase**: This is one of 50+ lint rules being implemented to provide excellent error messages and autofixes for FSH projects.

## Context

FSH projects often have inconsistent naming conventions, leading to:
- Difficulty finding resources (inconsistent casing)
- Publishing issues (IDs with spaces or invalid characters)
- Interoperability problems (non-standard canonical URLs)
- Poor readability and maintainability

SUSHI doesn't enforce naming conventions, allowing projects to develop inconsistent patterns. MAKI will provide configurable rules with autofixes to maintain consistency.

## Goals

1. **Implement naming-conventions rule** - Check ID and Name formatting
2. **Support multiple naming styles** - kebab-case, PascalCase, camelCase, snake_case
3. **Provide autofix capability** - Convert between naming styles automatically
4. **Make rules configurable** - Allow projects to customize conventions
5. **Integrate with existing rules** - Works with required-id and required-title rules

## Technical Specification

### Rule: `naming-conventions`

**Category**: Style
**Severity**: Warning (configurable)
**Autofix**: Yes (converts case automatically)

**Checks:**
1. **ID Format**: Should use kebab-case (e.g., `my-patient-profile`)
   - No spaces or special characters except hyphens
   - All lowercase
   - Hyphens between words
   - No leading/trailing hyphens

2. **Name Format**: Should use PascalCase (e.g., `MyPatientProfile`)
   - No spaces
   - Capitalize first letter of each word
   - No underscores or hyphens

3. **Entity Names**: Follow FHIR naming patterns
   - Profiles: Descriptive names ending in profile context (e.g., `PatientProfile`)
   - Extensions: Clear purpose (e.g., `BirthPlace`, `EthnicityExtension`)
   - ValueSets: Descriptive with "VS" suffix optional (e.g., `MaritalStatusVS`)
   - CodeSystems: Descriptive with "CS" suffix optional (e.g., `ConditionCategoriesCS`)

### Implementation Location

**File**: `crates/maki-rules/src/builtin/naming.rs`

This file already exists with basic structure. Extend it with comprehensive naming checks.

### Rule Implementation (Rust - CST-based)

**Note**: This shows the actual implementation using MAKI's Rowan-based CST API. For simpler pattern-based rules, see the GritQL alternative below.

```rust
use maki_core::cst::ast::{Document, Profile, Extension, ValueSet, CodeSystem, AstNode};
use maki_core::semantic::SemanticModel;
use maki_core::diagnostic::{Diagnostic, DiagnosticLocation, Severity};
use maki_core::autofix::CodeSuggestion;
use regex::Regex;

/// Naming convention styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamingStyle {
    KebabCase,   // my-entity-id
    PascalCase,  // MyEntityName
    CamelCase,   // myEntityName
    SnakeCase,   // my_entity_name
}

impl NamingStyle {
    /// Check if a string conforms to this naming style
    pub fn matches(&self, s: &str) -> bool {
        match self {
            NamingStyle::KebabCase => {
                s.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '-')
                    && !s.starts_with('-')
                    && !s.ends_with('-')
                    && !s.contains("--")
            }
            NamingStyle::PascalCase => {
                !s.is_empty()
                    && s.chars().next().unwrap().is_uppercase()
                    && s.chars().all(|c| c.is_alphanumeric())
                    && !s.contains('_')
                    && !s.contains('-')
            }
            NamingStyle::CamelCase => {
                !s.is_empty()
                    && s.chars().next().unwrap().is_lowercase()
                    && s.chars().all(|c| c.is_alphanumeric())
                    && !s.contains('_')
                    && !s.contains('-')
            }
            NamingStyle::SnakeCase => {
                s.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
                    && !s.starts_with('_')
                    && !s.ends_with('_')
                    && !s.contains("__")
            }
        }
    }

    /// Convert a string to this naming style
    pub fn convert(&self, s: &str) -> String {
        let words = Self::split_into_words(s);
        match self {
            NamingStyle::KebabCase => words.join("-").to_lowercase(),
            NamingStyle::PascalCase => words
                .iter()
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().chain(chars.map(|c| c.to_lowercase())).collect::<String>(),
                        None => String::new(),
                    }
                })
                .collect(),
            NamingStyle::CamelCase => {
                let mut result = String::new();
                for (i, word) in words.iter().enumerate() {
                    if i == 0 {
                        result.push_str(&word.to_lowercase());
                    } else {
                        let mut chars = word.chars();
                        if let Some(c) = chars.next() {
                            result.push(c.to_uppercase().next().unwrap());
                            result.push_str(&chars.as_str().to_lowercase());
                        }
                    }
                }
                result
            }
            NamingStyle::SnakeCase => words.join("_").to_lowercase(),
        }
    }

    /// Split string into words, handling various separators
    fn split_into_words(s: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut current = String::new();
        let mut prev_was_upper = false;

        for (i, c) in s.chars().enumerate() {
            if c == '-' || c == '_' || c == ' ' {
                if !current.is_empty() {
                    words.push(current.clone());
                    current.clear();
                }
                prev_was_upper = false;
            } else if c.is_uppercase() {
                // Handle PascalCase/camelCase word boundaries
                if !current.is_empty() && !prev_was_upper {
                    words.push(current.clone());
                    current.clear();
                }
                current.push(c);
                prev_was_upper = true;
            } else {
                current.push(c);
                prev_was_upper = false;
            }
        }

        if !current.is_empty() {
            words.push(current);
        }

        words
    }
}

/// Check naming conventions for all entities in the document
///
/// **ACTUAL API**: Uses SemanticModel + Document::cast() pattern (not Entity trait)
pub fn check_naming_conventions(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Cast CST root to typed Document node
    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check all profiles
    for profile in document.profiles() {
        // Check profile name (should be PascalCase)
        if let Some(name_node) = profile.name() {
            let name_text = name_node.text().to_string();
            if !NamingStyle::PascalCase.matches(&name_text) {
                let converted = NamingStyle::PascalCase.convert(&name_text);
                let location = model.source_map.node_to_diagnostic_location(name_node.syntax());

                diagnostics.push(
                    Diagnostic::new(
                        "naming-conventions",
                        Severity::Warning,
                        format!("Profile name '{}' should use PascalCase", name_text),
                        location.clone(),
                    )
                    .with_note("Names should use PascalCase (capitalize first letter of each word)")
                    .with_help("Use PascalCase for Names: 'MyEntityName'")
                    .with_suggestion(
                        CodeSuggestion::safe_fix(
                            format!("Convert to PascalCase: '{}'", converted),
                            converted,
                            location,
                        )
                    )
                );
            }
        }

        // Check profile ID (should be kebab-case)
        if let Some(id_node) = profile.id() {
            let id_text = id_node.text().to_string();
            if !NamingStyle::KebabCase.matches(&id_text) {
                let converted = NamingStyle::KebabCase.convert(&id_text);
                let location = model.source_map.node_to_diagnostic_location(id_node.syntax());

                diagnostics.push(
                    Diagnostic::new(
                        "naming-conventions",
                        Severity::Warning,
                        format!("Profile ID '{}' should use kebab-case", id_text),
                        location.clone(),
                    )
                    .with_note("IDs should use kebab-case (lowercase with hyphens)")
                    .with_help("Use kebab-case for IDs: 'my-entity-id'")
                    .with_suggestion(
                        CodeSuggestion::safe_fix(
                            format!("Convert to kebab-case: '{}'", converted),
                            converted,
                            location,
                        )
                    )
                );
            }
        }
    }

    // Check all extensions (same pattern)
    for extension in document.extensions() {
        // Check name
        if let Some(name_node) = extension.name() {
            let name_text = name_node.text().to_string();
            if !NamingStyle::PascalCase.matches(&name_text) {
                let converted = NamingStyle::PascalCase.convert(&name_text);
                let location = model.source_map.node_to_diagnostic_location(name_node.syntax());

                diagnostics.push(
                    Diagnostic::new(
                        "naming-conventions",
                        Severity::Warning,
                        format!("Extension name '{}' should use PascalCase", name_text),
                        location.clone(),
                    )
                    .with_suggestion(
                        CodeSuggestion::safe_fix(
                            format!("Convert to PascalCase: '{}'", converted),
                            converted,
                            location,
                        )
                    )
                );
            }
        }

        // Check ID
        if let Some(id_node) = extension.id() {
            let id_text = id_node.text().to_string();
            if !NamingStyle::KebabCase.matches(&id_text) {
                let converted = NamingStyle::KebabCase.convert(&id_text);
                let location = model.source_map.node_to_diagnostic_location(id_node.syntax());

                diagnostics.push(
                    Diagnostic::new(
                        "naming-conventions",
                        Severity::Warning,
                        format!("Extension ID '{}' should use kebab-case", id_text),
                        location.clone(),
                    )
                    .with_suggestion(
                        CodeSuggestion::safe_fix(
                            format!("Convert to kebab-case: '{}'", converted),
                            converted,
                            location,
                        )
                    )
                );
            }
        }
    }

    // Similar checks for ValueSets, CodeSystems, etc.
    // ... (same pattern)

    diagnostics
}
```

### GritQL Alternative (Pattern-based)

For users who prefer pattern-based rules without writing Rust code, the same checks can be expressed in GritQL:

**File**: `examples/gritql/rules/naming-conventions.grit`

```gritql
// Check Profile names are PascalCase
Profile: $name where {
    $name <: r"^[a-z]"  // Starts with lowercase = violation
} => {
    rewrite($name) => capitalize($name)
}

// Check Profile IDs are kebab-case
Profile: $p where {
    $p.id <: r"[A-Z_]"  // Has uppercase or underscore = violation
} => {
    rewrite($p.id) => to_kebab_case($p.id)
}

// Check Extension names are PascalCase
Extension: $name where {
    $name <: r"^[a-z]"
} => {
    rewrite($name) => capitalize($name)
}

// Check Extension IDs are kebab-case
Extension: $e where {
    $e.id <: r"[A-Z_]"
} => {
    rewrite($e.id) => to_kebab_case($e.id)
}
```

**Advantages of GritQL approach**:
- No Rust code needed
- Easier to customize for project-specific conventions
- Autofixes built into pattern
- Can be shared as `.grit` files

**When to use Rust vs GritQL**:
- **Rust**: Complex logic, performance-critical, needs semantic analysis
- **GritQL**: Simple patterns, project-specific rules, rapid iteration

**Note**: GritQL support requires Task 29 (GritQL Execution Engine) to be complete.

### Configuration Support

Allow projects to customize naming conventions in `.makirc.toml`:

```toml
[rules.naming-conventions]
enabled = true
severity = "warning"

[rules.naming-conventions.config]
# Naming style for IDs
id_style = "kebab-case"  # Options: kebab-case, snake_case, camelCase, PascalCase

# Naming style for Names
name_style = "PascalCase"  # Options: PascalCase, camelCase

# Allow exceptions (regex patterns)
id_exceptions = [
    "^hl7-.*",      # Allow HL7 prefixes
    "^fhir-.*"      # Allow FHIR prefixes
]

# Require specific suffixes for entity types
require_suffixes = true
suffixes = { Profile = "Profile", Extension = "Extension" }
```

### Integration with Existing Rules

This rule works alongside:
- **`required-id`** (Task 31) - Ensures ID exists before checking format
- **`required-title`** (Task 31) - Can use naming conventions to generate titles
- **`duplicate-entity-id`** (Task 35) - Checks for duplicate IDs

## Examples

### Example 1: Invalid ID Format

**Input FSH:**
```fsh
Profile: MyPatientProfile
Id: My_Patient_Profile    // ❌ Wrong: underscore and uppercase
Title: "My Patient Profile"
```

**Diagnostic:**
```
warning: ID 'My_Patient_Profile' should use kebab-case
  --> profiles/MyPatient.fsh:2:5
   |
 2 | Id: My_Patient_Profile
   |     ^^^^^^^^^^^^^^^^^^ expected 'my-patient-profile'
   |
   = note: IDs should use kebab-case (lowercase with hyphens)
   = help: Use kebab-case for IDs: 'my-entity-id'
```

**After Autofix:**
```fsh
Profile: MyPatientProfile
Id: my-patient-profile    // ✅ Fixed to kebab-case
Title: "My Patient Profile"
```

### Example 2: Invalid Name Format

**Input FSH:**
```fsh
Profile: my_patient_profile    // ❌ Wrong: snake_case
Parent: Patient
Id: my-patient-profile
```

**Diagnostic:**
```
warning: Name 'my_patient_profile' should use PascalCase
  --> profiles/MyPatient.fsh:1:10
   |
 1 | Profile: my_patient_profile
   |          ^^^^^^^^^^^^^^^^^^ expected 'MyPatientProfile'
   |
   = note: Names should use PascalCase (capitalize first letter of each word)
   = help: Use PascalCase for Names: 'MyEntityName'
```

**After Autofix:**
```fsh
Profile: MyPatientProfile    // ✅ Fixed to PascalCase
Parent: Patient
Id: my-patient-profile
```

### Example 3: Complex Name Conversion

**Input:** `patient_BIRTH-place_Extension`

**Conversions:**
- **kebab-case**: `patient-birth-place-extension`
- **PascalCase**: `PatientBirthPlaceExtension`
- **camelCase**: `patientBirthPlaceExtension`
- **snake_case**: `patient_birth_place_extension`

## Testing Requirements

### Unit Tests

**File**: `crates/maki-rules/tests/naming_test.rs`

```rust
#[test]
fn test_kebab_case_validation() {
    assert!(NamingStyle::KebabCase.matches("my-profile"));
    assert!(NamingStyle::KebabCase.matches("patient-profile-2"));
    assert!(!NamingStyle::KebabCase.matches("My-Profile"));  // uppercase
    assert!(!NamingStyle::KebabCase.matches("my_profile"));  // underscore
    assert!(!NamingStyle::KebabCase.matches("-my-profile")); // leading hyphen
}

#[test]
fn test_pascal_case_validation() {
    assert!(NamingStyle::PascalCase.matches("MyProfile"));
    assert!(NamingStyle::PascalCase.matches("PatientProfile2"));
    assert!(!NamingStyle::PascalCase.matches("myProfile"));   // lowercase start
    assert!(!NamingStyle::PascalCase.matches("My-Profile")); // hyphen
    assert!(!NamingStyle::PascalCase.matches("My_Profile")); // underscore
}

#[test]
fn test_case_conversion() {
    let input = "My_Patient-Profile";

    assert_eq!(
        NamingStyle::KebabCase.convert(input),
        "my-patient-profile"
    );

    assert_eq!(
        NamingStyle::PascalCase.convert(input),
        "MyPatientProfile"
    );

    assert_eq!(
        NamingStyle::CamelCase.convert(input),
        "myPatientProfile"
    );

    assert_eq!(
        NamingStyle::SnakeCase.convert(input),
        "my_patient_profile"
    );
}

#[test]
fn test_naming_rule_detects_violations() {
    let source = r#"
        Profile: my_profile
        Id: MyProfile
        Title: "My Profile"
    "#;

    let diagnostics = lint_source(source, &["naming-conventions"]);

    // Should detect both violations
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|d| d.message.contains("Name")));
    assert!(diagnostics.iter().any(|d| d.message.contains("ID")));
}

#[test]
fn test_autofix_converts_correctly() {
    let source = r#"
        Profile: my_patient_profile
        Id: MyPatientProfile
    "#;

    let fixed = apply_autofixes(source, &["naming-conventions"]);

    assert!(fixed.contains("Profile: MyPatientProfile"));
    assert!(fixed.contains("Id: my-patient-profile"));
}
```

### Integration Tests

Test with real FSH files from `examples/`:

```bash
# Test naming violations detection
maki lint examples/naming/ --filter naming-conventions

# Test autofix
maki lint examples/naming/ --fix --filter naming-conventions

# Verify no violations after fix
maki lint examples/naming/ --filter naming-conventions
```

### Golden Files

**File**: `crates/maki-core/tests/golden_files/naming_violations.fsh`

```fsh
// Various naming convention violations for testing

Profile: patient_profile_basic    // Wrong: snake_case name
Id: PatientProfileBasic           // Wrong: PascalCase ID
Title: "Patient Profile"

Extension: Birth-Place            // Wrong: kebab-case name
Id: birthPlace                    // Wrong: camelCase ID

ValueSet: MARITAL_STATUS_VS       // Wrong: UPPER_SNAKE_CASE
Id: MaritalStatus-VS              // Wrong: mixed case

CodeSystem: condition-categories  // Wrong: kebab-case name
Id: ConditionCategories           // Wrong: PascalCase ID
```

## Configuration

### Rule Configuration

Users can customize naming conventions in `.makirc.toml`:

```toml
[rules.naming-conventions]
enabled = true
severity = "warning"

[rules.naming-conventions.config]
id_style = "kebab-case"
name_style = "PascalCase"

# Custom patterns for specific entity types
[rules.naming-conventions.config.patterns]
Profile = { name = ".*Profile$", id = "^[a-z][a-z0-9]*(-[a-z0-9]+)*$" }
Extension = { name = ".*Extension$", id = "^[a-z][a-z0-9]*(-[a-z0-9]+)*$" }
```

### CLI Usage

```bash
# Check naming conventions
maki lint --filter naming-conventions

# Fix naming violations
maki lint --fix --filter naming-conventions

# Show rule details
maki rules --detailed naming-conventions
```

## Dependencies

### Required Components
- **CST/AST** (Task 03-04): For parsing and traversing entity definitions
- **Diagnostic System** (Task 06): For reporting violations
- **Autofix Engine** (Task 37): For automatic case conversion
- **Rule Registry** (Task 15): For rule registration and configuration

### Integration Points
- **required-id rule**: Ensures ID exists before checking format
- **required-title rule**: Can generate title from properly-formatted name
- **Configuration system**: For customizing naming styles

## Acceptance Criteria

- [ ] `NamingStyle` enum supports kebab-case, PascalCase, camelCase, snake_case
- [ ] `matches()` method correctly validates each naming style
- [ ] `convert()` method correctly converts between naming styles
- [ ] `split_into_words()` handles various input formats (PascalCase, snake_case, kebab-case, mixed)
- [ ] `check_naming_conventions()` detects ID format violations
- [ ] `check_naming_conventions()` detects Name format violations
- [ ] Diagnostics include clear messages with expected format
- [ ] Autofixes convert to correct naming style
- [ ] Configuration file support (`.makirc.toml`)
- [ ] CLI filtering works (`--filter naming-conventions`)
- [ ] Unit tests pass for all naming styles
- [ ] Integration tests verify fixes on real FSH files
- [ ] Golden files demonstrate various violations
- [ ] Documentation includes examples and configuration options
- [ ] Performance: &lt;1ms per entity for naming checks
- [ ] Edge cases handled: empty strings, single characters, numeric prefixes

## Performance Considerations

- **Efficient Pattern Matching**: Use simple character iteration instead of regex
- **Lazy Conversion**: Only convert strings when violations are detected
- **Caching**: Cache compiled configuration patterns
- **Batch Processing**: Check all entities in parallel using rayon

## Future Enhancements

1. **Context-Specific Naming**:
   - Different conventions for different entity types
   - Project-wide naming prefixes (e.g., `myorg-`)

2. **Custom Validation Patterns**:
   - Regex-based naming patterns
   - Length constraints (min/max characters)

3. **Naming Consistency Checks**:
   - Ensure related entities use consistent naming (e.g., `PatientProfile` and `PatientExample`)
   - Check that IDs match Names in some normalized form

4. **FHIR-Specific Patterns**:
   - Warn about reserved FHIR names
   - Suggest better names based on FHIR conventions

## Resources

- **FHIR Naming Conventions**: https://www.hl7.org/fhir/general.html#naming
- **FSH Specification**: https://hl7.org/fhir/uv/shorthand/
- **Rust Naming Conventions**: https://rust-lang.github.io/api-guidelines/naming.html

## Related Tasks

- **Task 31: Metadata Requirements** - Generates IDs/Titles using naming conventions
- **Task 35: Duplicate Detection** - Checks for duplicate IDs after normalization
- **Task 37: Autofix Engine Enhancement** - Implements safe/unsafe fix classification
- **Task 70: SUSHI Migrator** - May need to fix naming when migrating projects

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium
**Priority**: High (part of core linting experience)
**Updated**: 2025-11-03
