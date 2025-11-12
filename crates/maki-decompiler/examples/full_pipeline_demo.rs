//! Full pipeline demonstration: FHIR JSON → FSH
//!
//! This example demonstrates the complete decompiler pipeline from loading a FHIR
//! StructureDefinition JSON file to extracting all rules and generating FSH output.

use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
use maki_decompiler::{
    lake::ResourceLake,
    models::{
        Derivation, ElementDefinition, ElementList, StructureDefinition,
        common::{Binding, BindingStrength},
    },
    processor::StructureDefinitionProcessor,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Full Decompiler Pipeline Demo ===\n");

    // Step 1: Create ResourceLake with canonical manager
    println!("Step 1: Initializing ResourceLake...");
    let options = CanonicalOptions {
        quick_init: true,
        auto_install_core: false,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    let session = facade.session(vec![FhirRelease::R4]).await.unwrap();
    let lake = ResourceLake::new(Arc::new(session));
    println!("   ✓ ResourceLake initialized\n");

    // Step 2: Create a sample StructureDefinition (Profile)
    println!("Step 2: Creating sample US Core Patient Profile...");
    let sd = create_us_core_patient_profile();
    println!("   ✓ Profile: {}", sd.name);
    println!("   ✓ Base: {}", sd.base_definition.as_ref().unwrap());
    println!(
        "   ✓ Elements: {} differential elements\n",
        sd.differential
            .as_ref()
            .map(|d| d.element.len())
            .unwrap_or(0)
    );

    // Step 3: Process the StructureDefinition
    println!("Step 3: Processing StructureDefinition...");
    let processor = StructureDefinitionProcessor::new(&lake);
    let exportable = processor.process(&sd).await.unwrap();
    println!("   ✓ Processed as: Profile");
    println!("   ✓ Name: {}", exportable.name());
    println!("   ✓ ID: {}", exportable.id());

    // Step 4: Generate FSH output
    println!("\nStep 4: Generated FSH Output:");
    println!("{}", "=".repeat(60));
    let fsh = exportable.to_fsh();
    println!("{}", fsh);
    println!("{}", "=".repeat(60));

    println!("\n=== Demo Complete ===");
}

/// Create a sample US Core Patient Profile with various rules
fn create_us_core_patient_profile() -> StructureDefinition {
    StructureDefinition {
        resource_type: Some("StructureDefinition".to_string()),
        id: Some("us-core-patient".to_string()),
        url: "http://hl7.org/fhir/us/core/StructureDefinition/us-core-patient".to_string(),
        name: "USCorePatientProfile".to_string(),
        title: Some("US Core Patient Profile".to_string()),
        status: "active".to_string(),
        description: Some("The US Core Patient Profile meets the U.S. Core Data for Interoperability (USCDI) requirements.".to_string()),
        base_definition: Some("http://hl7.org/fhir/StructureDefinition/Patient".to_string()),
        derivation: Some(Derivation::Constraint),
        kind: None,
        abstract_: None,
        context: None,
        version: Some("3.1.1".to_string()),
        publisher: Some("HL7 US Realm Steering Committee".to_string()),
        contact: None,
        copyright: None,
        differential: Some(ElementList {
            element: vec![
                // Root element with metadata
                ElementDefinition {
                    id: "Patient".to_string(),
                    path: "Patient".to_string(),
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
                    short: Some("US Core Patient Profile".to_string()),
                    definition: Some("Defines constraints and extensions on the Patient resource for the minimal set of data to query and retrieve patient demographic information.".to_string()),
                    comment: None,
                    requirements: None,
                    alias: None,
                    example: None,
                },
                // identifier - must support
                ElementDefinition {
                    id: "Patient.identifier".to_string(),
                    path: "Patient.identifier".to_string(),
                    slice_name: None,
                    min: Some(1),
                    max: Some("*".to_string()),
                    type_: None,
                    must_support: Some(true),
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
                    short: Some("An identifier for this patient".to_string()),
                    definition: None,
                    comment: None,
                    requirements: None,
                    alias: None,
                    example: None,
                },
                // name - must support, required
                ElementDefinition {
                    id: "Patient.name".to_string(),
                    path: "Patient.name".to_string(),
                    slice_name: None,
                    min: Some(1),
                    max: Some("*".to_string()),
                    type_: None,
                    must_support: Some(true),
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
                    short: Some("A name associated with the patient".to_string()),
                    definition: None,
                    comment: None,
                    requirements: None,
                    alias: None,
                    example: None,
                },
                // gender - must support, extensible binding
                ElementDefinition {
                    id: "Patient.gender".to_string(),
                    path: "Patient.gender".to_string(),
                    slice_name: None,
                    min: None,
                    max: None,
                    type_: None,
                    must_support: Some(true),
                    is_modifier: None,
                    is_summary: None,
                    binding: Some(Binding {
                        strength: BindingStrength::Required,
                        value_set: Some("http://hl7.org/fhir/ValueSet/administrative-gender".to_string()),
                        description: None,
                    }),
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
            ],
        }),
        snapshot: None,
    }
}
