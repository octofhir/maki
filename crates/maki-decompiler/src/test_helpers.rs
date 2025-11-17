//! Test helpers and utilities for unit and integration testing
//!
//! This module provides:
//! - Test data builders for FHIR resources
//! - FSH validation helpers
//! - Common test fixtures
//!
//! Note: For async tests requiring Fishable, use ResourceLake with a real DefinitionSession.
//! For simple unit tests, use the test data builders to create test data directly.

use crate::models::*;

/// Builder for creating test ElementDefinition instances
pub struct TestElementBuilder {
    element: ElementDefinition,
}

impl TestElementBuilder {
    /// Create a new builder with the given path
    pub fn new(path: &str) -> Self {
        Self {
            element: ElementDefinition {
                id: path.to_string(),
                path: path.to_string(),
                slice_name: None,
                min: None,
                max: None,
                type_: None,
                must_support: None,
                is_modifier: None,
                is_summary: None,
                binding: None,
                constraint: None,
                slicing: None,
                fixed_boolean: None,
                fixed_integer: None,
                fixed_decimal: None,
                fixed_string: None,
                fixed_uri: None,
                fixed_url: None,
                fixed_canonical: None,
                fixed_code: None,
                fixed_date: None,
                fixed_date_time: None,
                fixed_instant: None,
                fixed_time: None,
                fixed_id: None,
                fixed_oid: None,
                fixed_uuid: None,
                fixed_codeable_concept: None,
                fixed_coding: None,
                fixed_quantity: None,
                fixed_identifier: None,
                fixed_reference: None,
                pattern_boolean: None,
                pattern_integer: None,
                pattern_decimal: None,
                pattern_string: None,
                pattern_code: None,
                pattern_codeable_concept: None,
                pattern_coding: None,
                pattern_quantity: None,
                pattern_identifier: None,
                pattern_reference: None,
                short: None,
                definition: None,
                comment: None,
                requirements: None,
                alias: None,
                example: None,
            },
        }
    }

    /// Set cardinality (min and max)
    pub fn with_cardinality(mut self, min: u32, max: &str) -> Self {
        self.element.min = Some(min);
        self.element.max = Some(max.to_string());
        self
    }

    /// Set only min cardinality
    pub fn with_min(mut self, min: u32) -> Self {
        self.element.min = Some(min);
        self
    }

    /// Set only max cardinality
    pub fn with_max(mut self, max: &str) -> Self {
        self.element.max = Some(max.to_string());
        self
    }

    /// Add a type constraint
    pub fn with_type(mut self, type_code: &str) -> Self {
        let type_def = TypeRef {
            code: type_code.to_string(),
            profile: None,
            target_profile: None,
        };
        let mut types = self.element.type_.unwrap_or_default();
        types.push(type_def);
        self.element.type_ = Some(types);
        self
    }

    /// Add a type with profile constraint
    pub fn with_type_profile(mut self, type_code: &str, profile: &str) -> Self {
        let type_def = TypeRef {
            code: type_code.to_string(),
            profile: Some(vec![profile.to_string()]),
            target_profile: None,
        };
        let mut types = self.element.type_.unwrap_or_default();
        types.push(type_def);
        self.element.type_ = Some(types);
        self
    }

    /// Set binding
    pub fn with_binding(mut self, value_set: &str, strength: BindingStrength) -> Self {
        self.element.binding = Some(Binding {
            strength,
            value_set: Some(value_set.to_string()),
            description: None,
        });
        self
    }

    /// Set short description
    pub fn with_short(mut self, short: &str) -> Self {
        self.element.short = Some(short.to_string());
        self
    }

    /// Set definition
    pub fn with_definition(mut self, definition: &str) -> Self {
        self.element.definition = Some(definition.to_string());
        self
    }

    /// Set comment
    pub fn with_comment(mut self, comment: &str) -> Self {
        self.element.comment = Some(comment.to_string());
        self
    }

    /// Set must support flag
    pub fn with_must_support(mut self, value: bool) -> Self {
        self.element.must_support = Some(value);
        self
    }

    /// Set is modifier flag
    pub fn with_is_modifier(mut self, value: bool) -> Self {
        self.element.is_modifier = Some(value);
        self
    }

    /// Set is summary flag
    pub fn with_is_summary(mut self, value: bool) -> Self {
        self.element.is_summary = Some(value);
        self
    }

    /// Set fixed value (boolean)
    pub fn with_fixed_boolean(mut self, value: bool) -> Self {
        self.element.fixed_boolean = Some(value);
        self
    }

    /// Set fixed value (string)
    pub fn with_fixed_string(mut self, value: &str) -> Self {
        self.element.fixed_string = Some(value.to_string());
        self
    }

    /// Set fixed value (integer)
    pub fn with_fixed_integer(mut self, value: i32) -> Self {
        self.element.fixed_integer = Some(value);
        self
    }

    /// Set fixed value (code)
    pub fn with_fixed_code(mut self, value: &str) -> Self {
        self.element.fixed_code = Some(value.to_string());
        self
    }

    /// Set fixed value (CodeableConcept)
    pub fn with_fixed_codeable_concept(mut self, system: &str, code: &str, display: &str) -> Self {
        self.element.fixed_codeable_concept = Some(CodeableConcept {
            coding: Some(vec![Coding {
                system: Some(system.to_string()),
                version: None,
                code: Some(code.to_string()),
                display: Some(display.to_string()),
            }]),
            text: None,
        });
        self
    }

    /// Set pattern value (CodeableConcept)
    pub fn with_pattern_codeable_concept(mut self, system: &str, code: &str) -> Self {
        self.element.pattern_codeable_concept = Some(CodeableConcept {
            coding: Some(vec![Coding {
                system: Some(system.to_string()),
                version: None,
                code: Some(code.to_string()),
                display: None,
            }]),
            text: None,
        });
        self
    }

    /// Set slicing information
    pub fn with_slicing(
        mut self,
        discriminator_type: DiscriminatorType,
        discriminator_path: &str,
    ) -> Self {
        self.element.slicing = Some(Slicing {
            discriminator: Some(vec![Discriminator {
                type_: discriminator_type,
                path: discriminator_path.to_string(),
            }]),
            description: None,
            ordered: None,
            rules: Some(SlicingRules::Open),
        });
        self
    }

    /// Set slice name
    pub fn with_slice_name(mut self, name: &str) -> Self {
        self.element.slice_name = Some(name.to_string());
        self
    }

    /// Add a constraint
    pub fn with_constraint(
        mut self,
        key: &str,
        severity: &str,
        human: &str,
        expression: &str,
    ) -> Self {
        let constraint = Constraint {
            key: key.to_string(),
            severity: Some(severity.to_string()),
            human: human.to_string(),
            expression: Some(expression.to_string()),
            xpath: None,
        };
        let mut constraints = self.element.constraint.unwrap_or_default();
        constraints.push(constraint);
        self.element.constraint = Some(constraints);
        self
    }

    /// Build the ElementDefinition
    pub fn build(self) -> ElementDefinition {
        self.element
    }
}

/// Builder for creating test StructureDefinition instances
pub struct TestProfileBuilder {
    profile: StructureDefinition,
}

impl TestProfileBuilder {
    /// Create a new profile builder
    pub fn new(name: &str, url: &str) -> Self {
        Self {
            profile: StructureDefinition {
                resource_type: Some("StructureDefinition".to_string()),
                id: Some(name.to_string()),
                url: url.to_string(),
                name: name.to_string(),
                status: "draft".to_string(),
                kind: Some(StructureDefinitionKind::Resource),
                abstract_: Some(false),
                base_definition: Some(
                    "http://hl7.org/fhir/StructureDefinition/Patient".to_string(),
                ),
                derivation: Some(Derivation::Constraint),
                differential: None,
                snapshot: None,
                version: None,
                title: None,
                description: None,
                publisher: None,
                contact: None,
                context: None,
                copyright: None,
            },
        }
    }

    /// Set the base definition
    pub fn with_base(mut self, base: &str) -> Self {
        self.profile.base_definition = Some(base.to_string());
        self
    }

    /// Set the kind
    pub fn with_kind(mut self, kind: StructureDefinitionKind) -> Self {
        self.profile.kind = Some(kind);
        self
    }

    /// Set title
    pub fn with_title(mut self, title: &str) -> Self {
        self.profile.title = Some(title.to_string());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.profile.description = Some(description.to_string());
        self
    }

    /// Add differential elements
    pub fn with_differential(mut self, elements: Vec<ElementDefinition>) -> Self {
        self.profile.differential = Some(ElementList { element: elements });
        self
    }

    /// Add snapshot elements
    pub fn with_snapshot(mut self, elements: Vec<ElementDefinition>) -> Self {
        self.profile.snapshot = Some(ElementList { element: elements });
        self
    }

    /// Build the StructureDefinition
    pub fn build(self) -> StructureDefinition {
        self.profile
    }
}

/// FSH validation helpers
pub mod fsh_validation {
    /// Assert that FSH output contains expected rules
    pub fn assert_fsh_contains(fsh: &str, expected_rules: &[&str]) {
        for rule in expected_rules {
            assert!(
                fsh.contains(rule),
                "Expected rule not found in FSH output:\n  Expected: {}\n  Actual FSH:\n{}",
                rule,
                fsh
            );
        }
    }

    /// Assert that FSH output does NOT contain specific rules
    pub fn assert_fsh_not_contains(fsh: &str, unexpected_rules: &[&str]) {
        for rule in unexpected_rules {
            assert!(
                !fsh.contains(rule),
                "Unexpected rule found in FSH output:\n  Unexpected: {}\n  Actual FSH:\n{}",
                rule,
                fsh
            );
        }
    }

    /// Assert that FSH contains all rules in specific order
    pub fn assert_fsh_order(fsh: &str, ordered_rules: &[&str]) {
        let mut last_pos = 0;
        for (i, rule) in ordered_rules.iter().enumerate() {
            if let Some(pos) = fsh[last_pos..].find(rule) {
                last_pos += pos + rule.len();
            } else {
                panic!(
                    "Rule {} (index {}) not found in expected order.\n  Expected: {}\n  FSH:\n{}",
                    i, i, rule, fsh
                );
            }
        }
    }

    /// Normalize FSH for comparison (removes extra whitespace, normalizes line endings)
    pub fn normalize_fsh(fsh: &str) -> String {
        fsh.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Common test fixtures
pub mod fixtures {
    use super::*;

    /// Create a simple Patient profile for testing
    pub fn simple_patient_profile() -> StructureDefinition {
        TestProfileBuilder::new(
            "SimplePatientProfile",
            "http://example.org/StructureDefinition/SimplePatientProfile",
        )
        .with_title("Simple Patient Profile")
        .with_description("A simple patient profile for testing")
        .with_differential(vec![
            TestElementBuilder::new("Patient").build(),
            TestElementBuilder::new("Patient.identifier")
                .with_cardinality(1, "*")
                .with_must_support(true)
                .build(),
            TestElementBuilder::new("Patient.name")
                .with_cardinality(1, "*")
                .with_must_support(true)
                .build(),
        ])
        .build()
    }

    /// Create a Patient profile with various constraint types
    pub fn complex_patient_profile() -> StructureDefinition {
        TestProfileBuilder::new(
            "ComplexPatientProfile",
            "http://example.org/StructureDefinition/ComplexPatientProfile",
        )
        .with_title("Complex Patient Profile")
        .with_description("A complex patient profile with various constraints")
        .with_differential(vec![
            TestElementBuilder::new("Patient").build(),
            TestElementBuilder::new("Patient.identifier")
                .with_cardinality(1, "*")
                .with_must_support(true)
                .with_short("Patient identifier")
                .build(),
            TestElementBuilder::new("Patient.active")
                .with_cardinality(1, "1")
                .with_must_support(true)
                .with_fixed_boolean(true)
                .build(),
            TestElementBuilder::new("Patient.gender")
                .with_binding(
                    "http://hl7.org/fhir/ValueSet/administrative-gender",
                    BindingStrength::Required,
                )
                .with_must_support(true)
                .build(),
            TestElementBuilder::new("Patient.birthDate")
                .with_cardinality(1, "1")
                .with_must_support(true)
                .build(),
        ])
        .build()
    }

    /// Create an Observation profile with slicing
    pub fn observation_with_slicing() -> StructureDefinition {
        TestProfileBuilder::new(
            "ObservationWithSlicing",
            "http://example.org/StructureDefinition/ObservationWithSlicing",
        )
        .with_base("http://hl7.org/fhir/StructureDefinition/Observation")
        .with_title("Observation with Slicing")
        .with_differential(vec![
            TestElementBuilder::new("Observation").build(),
            TestElementBuilder::new("Observation.category")
                .with_cardinality(1, "*")
                .with_slicing(DiscriminatorType::Pattern, "coding.code")
                .build(),
            TestElementBuilder::new("Observation.category:laboratory")
                .with_slice_name("laboratory")
                .with_cardinality(1, "1")
                .with_pattern_codeable_concept(
                    "http://terminology.hl7.org/CodeSystem/observation-category",
                    "laboratory",
                )
                .build(),
        ])
        .build()
    }
}

#[cfg(test)]
mod test_helpers_tests {
    use super::*;

    #[test]
    fn test_element_builder_basic() {
        let elem = TestElementBuilder::new("Patient.identifier")
            .with_cardinality(0, "1")
            .build();

        assert_eq!(elem.path, "Patient.identifier");
        assert_eq!(elem.min, Some(0));
        assert_eq!(elem.max, Some("1".to_string()));
    }

    #[test]
    fn test_element_builder_with_binding() {
        let elem = TestElementBuilder::new("Patient.gender")
            .with_binding(
                "http://hl7.org/fhir/ValueSet/administrative-gender",
                BindingStrength::Required,
            )
            .build();

        assert!(elem.binding.is_some());
        let binding = elem.binding.unwrap();
        assert_eq!(binding.strength, BindingStrength::Required);
        assert_eq!(
            binding.value_set,
            Some("http://hl7.org/fhir/ValueSet/administrative-gender".to_string())
        );
    }

    #[test]
    fn test_profile_builder() {
        let profile = TestProfileBuilder::new(
            "TestProfile",
            "http://example.org/StructureDefinition/TestProfile",
        )
        .with_title("Test Profile")
        .with_description("A test profile")
        .build();

        assert_eq!(profile.name, "TestProfile");
        assert_eq!(
            profile.url,
            "http://example.org/StructureDefinition/TestProfile"
        );
        assert_eq!(profile.title, Some("Test Profile".to_string()));
    }

    #[test]
    fn test_fsh_validation_contains() {
        let fsh = "Profile: TestProfile\nParent: Patient\n* identifier 1..*";
        fsh_validation::assert_fsh_contains(fsh, &["Profile: TestProfile", "* identifier 1..*"]);
    }

    #[test]
    #[should_panic(expected = "Expected rule not found")]
    fn test_fsh_validation_contains_fails() {
        let fsh = "Profile: TestProfile\nParent: Patient";
        fsh_validation::assert_fsh_contains(fsh, &["* identifier 1..*"]);
    }
}
