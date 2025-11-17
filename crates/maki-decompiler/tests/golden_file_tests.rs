//! Golden File Tests for FHIR to FSH Decompiler
//!
//! These tests verify that the decompiler produces expected FSH output
//! for given FHIR inputs. The tests use programmatically generated FHIR
//! resources and verify the FSH output matches expected patterns.

use maki_decompiler::models::*;
use maki_decompiler::test_helpers::*;
use maki_decompiler::*;
use serial_test::serial;

/// Helper to decompile a StructureDefinition to FSH
async fn decompile_profile(sd: StructureDefinition) -> Result<String> {
    // Set up canonical environment
    let session = setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![]).await?;
    let lake = ResourceLake::new(session);

    // Process the profile
    let processor = StructureDefinitionProcessor::new(&lake);
    let exportable = processor.process(&sd).await?;

    // Generate FSH
    Ok(exportable.to_fsh())
}

// ============================================================================
// PROFILE TESTS (8 tests)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_simple_patient_profile() {
    let profile = fixtures::simple_patient_profile();
    let fsh = decompile_profile(profile).await.unwrap();

    fsh_validation::assert_fsh_contains(
        &fsh,
        &[
            "Profile: SimplePatientProfile",
            "Parent: Patient",
            "* identifier 1..*",
            "* identifier MS",
            "* name 1..*",
            "* name MS",
        ],
    );
}

#[tokio::test]
#[serial]
async fn test_complex_patient_profile() {
    let profile = fixtures::complex_patient_profile();
    let fsh = decompile_profile(profile).await.unwrap();

    fsh_validation::assert_fsh_contains(
        &fsh,
        &[
            "Profile: ComplexPatientProfile",
            "Parent: Patient",
            "* identifier 1..*",
            "* identifier MS",
            "* active 1..1",
            "* active MS",
            "* birthDate 1..1",
            "* birthDate MS",
        ],
    );
}

#[tokio::test]
#[serial]
async fn test_profile_with_slicing() {
    let profile = fixtures::observation_with_slicing();
    let fsh = decompile_profile(profile).await.unwrap();

    fsh_validation::assert_fsh_contains(
        &fsh,
        &[
            "Profile: ObservationWithSlicing",
            "Parent: Observation",
            "* category 1..*",
            "* category:laboratory 1..1",
        ],
    );
}

#[tokio::test]
#[serial]
async fn test_profile_with_extensions() {
    let profile = TestProfileBuilder::new(
        "PatientWithExtensions",
        "http://example.org/StructureDefinition/PatientWithExtensions",
    )
    .with_title("Patient with Extensions")
    .with_differential(vec![
        TestElementBuilder::new("Patient").build(),
        TestElementBuilder::new("Patient.extension")
            .with_cardinality(0, "*")
            .build(),
        TestElementBuilder::new("Patient.extension:race")
            .with_slice_name("race")
            .with_cardinality(0, "1")
            .with_type_profile(
                "Extension",
                "http://hl7.org/fhir/us/core/StructureDefinition/us-core-race",
            )
            .build(),
    ])
    .build();

    let fsh = decompile_profile(profile).await.unwrap();
    fsh_validation::assert_fsh_contains(
        &fsh,
        &["Profile: PatientWithExtensions", "Parent: Patient"],
    );
}

#[tokio::test]
#[serial]
async fn test_profile_with_invariants() {
    let profile = TestProfileBuilder::new(
        "PatientWithInvariants",
        "http://example.org/StructureDefinition/PatientWithInvariants",
    )
    .with_title("Patient with Invariants")
    .with_differential(vec![
        TestElementBuilder::new("Patient").build(),
        TestElementBuilder::new("Patient.identifier")
            .with_cardinality(1, "*")
            .with_constraint(
                "pat-1",
                "error",
                "Either system or value must be present",
                "system.exists() or value.exists()",
            )
            .build(),
    ])
    .build();

    let fsh = decompile_profile(profile).await.unwrap();
    fsh_validation::assert_fsh_contains(
        &fsh,
        &["Profile: PatientWithInvariants", "* identifier 1..*"],
    );
}

#[tokio::test]
#[serial]
async fn test_profile_with_bindings() {
    let profile = TestProfileBuilder::new(
        "ObservationWithBinding",
        "http://example.org/StructureDefinition/ObservationWithBinding",
    )
    .with_base("http://hl7.org/fhir/StructureDefinition/Observation")
    .with_title("Observation with Binding")
    .with_differential(vec![
        TestElementBuilder::new("Observation").build(),
        TestElementBuilder::new("Observation.status")
            .with_binding(
                "http://hl7.org/fhir/ValueSet/observation-status",
                BindingStrength::Required,
            )
            .build(),
        TestElementBuilder::new("Observation.category")
            .with_binding(
                "http://hl7.org/fhir/ValueSet/observation-category",
                BindingStrength::Extensible,
            )
            .build(),
    ])
    .build();

    let fsh = decompile_profile(profile).await.unwrap();
    fsh_validation::assert_fsh_contains(
        &fsh,
        &["Profile: ObservationWithBinding", "Parent: Observation"],
    );
}

#[tokio::test]
#[serial]
async fn test_profile_with_nested_slicing() {
    let profile = TestProfileBuilder::new(
        "PatientWithNestedSlicing",
        "http://example.org/StructureDefinition/PatientWithNestedSlicing",
    )
    .with_title("Patient with Nested Slicing")
    .with_differential(vec![
        TestElementBuilder::new("Patient").build(),
        TestElementBuilder::new("Patient.name")
            .with_cardinality(1, "*")
            .with_slicing(DiscriminatorType::Value, "use")
            .build(),
        TestElementBuilder::new("Patient.name:official")
            .with_slice_name("official")
            .with_cardinality(1, "1")
            .build(),
        TestElementBuilder::new("Patient.name:official.use")
            .with_fixed_code("official")
            .build(),
    ])
    .build();

    let fsh = decompile_profile(profile).await.unwrap();
    fsh_validation::assert_fsh_contains(
        &fsh,
        &["Profile: PatientWithNestedSlicing", "* name 1..*"],
    );
}

#[tokio::test]
#[serial]
async fn test_profile_with_cardinality_constraints() {
    let profile = TestProfileBuilder::new(
        "PatientWithCardinalityConstraints",
        "http://example.org/StructureDefinition/PatientWithCardinalityConstraints",
    )
    .with_title("Patient with Cardinality Constraints")
    .with_differential(vec![
        TestElementBuilder::new("Patient").build(),
        TestElementBuilder::new("Patient.identifier")
            .with_cardinality(1, "*")
            .build(),
        TestElementBuilder::new("Patient.name")
            .with_cardinality(1, "3")
            .build(),
        TestElementBuilder::new("Patient.telecom")
            .with_cardinality(0, "0")
            .build(),
    ])
    .build();

    let fsh = decompile_profile(profile).await.unwrap();
    fsh_validation::assert_fsh_contains(
        &fsh,
        &[
            "Profile: PatientWithCardinalityConstraints",
            "* identifier 1..*",
            "* name 1..3",
            "* telecom 0..0",
        ],
    );
}

// ============================================================================
// VALUESET TESTS (4 tests)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_simple_valueset() {
    // ValueSet tests require ValueSetProcessor
    // For now, we'll create a placeholder test structure
    // This will be implemented when ValueSet golden files are added

    // TODO: Implement ValueSet golden file test
    // let vs = create_simple_valueset();
    // let fsh = decompile_valueset(vs).await.unwrap();
    // fsh_validation::assert_fsh_contains(&fsh, &["ValueSet: SimpleVS"]);
}

#[tokio::test]
#[serial]
async fn test_valueset_with_filters() {
    // TODO: Implement ValueSet with filters test
}

#[tokio::test]
#[serial]
async fn test_composed_valueset() {
    // TODO: Implement composed ValueSet test
}

#[tokio::test]
#[serial]
async fn test_expansion_only_valueset() {
    // TODO: Implement expansion-only ValueSet test
}

// ============================================================================
// CODESYSTEM TESTS (3 tests)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_flat_codesystem() {
    // TODO: Implement flat CodeSystem test
}

#[tokio::test]
#[serial]
async fn test_hierarchical_codesystem() {
    // TODO: Implement hierarchical CodeSystem test
}

#[tokio::test]
#[serial]
async fn test_codesystem_with_properties() {
    // TODO: Implement CodeSystem with properties test
}

// ============================================================================
// INSTANCE TESTS (3 tests)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_simple_instance() {
    // TODO: Implement simple Instance test
}

#[tokio::test]
#[serial]
async fn test_instance_with_inline_resources() {
    // TODO: Implement Instance with inline resources test
}

#[tokio::test]
#[serial]
async fn test_complex_instance() {
    // TODO: Implement complex Instance test
}

// ============================================================================
// EXTENSION TESTS (2 tests)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_simple_extension() {
    let extension = TestProfileBuilder::new(
        "SimpleExtension",
        "http://example.org/StructureDefinition/SimpleExtension",
    )
    .with_base("http://hl7.org/fhir/StructureDefinition/Extension")
    .with_title("Simple Extension")
    .with_kind(StructureDefinitionKind::ComplexType)
    .with_differential(vec![
        TestElementBuilder::new("Extension").build(),
        TestElementBuilder::new("Extension.value[x]")
            .with_cardinality(1, "1")
            .with_type("string")
            .build(),
    ])
    .build();

    let fsh = decompile_profile(extension).await.unwrap();
    fsh_validation::assert_fsh_contains(&fsh, &["Extension: SimpleExtension"]);
}

#[tokio::test]
#[serial]
async fn test_complex_extension() {
    let extension = TestProfileBuilder::new(
        "ComplexExtension",
        "http://example.org/StructureDefinition/ComplexExtension",
    )
    .with_base("http://hl7.org/fhir/StructureDefinition/Extension")
    .with_title("Complex Extension")
    .with_kind(StructureDefinitionKind::ComplexType)
    .with_differential(vec![
        TestElementBuilder::new("Extension").build(),
        TestElementBuilder::new("Extension.extension")
            .with_cardinality(2, "3")
            .with_slicing(DiscriminatorType::Value, "url")
            .build(),
        TestElementBuilder::new("Extension.extension:subext1")
            .with_slice_name("subext1")
            .with_cardinality(1, "1")
            .build(),
        TestElementBuilder::new("Extension.extension:subext2")
            .with_slice_name("subext2")
            .with_cardinality(1, "2")
            .build(),
    ])
    .build();

    let fsh = decompile_profile(extension).await.unwrap();
    fsh_validation::assert_fsh_contains(&fsh, &["Extension: ComplexExtension"]);
}
