//! Demonstration of AssignmentExtractor functionality
//!
//! This example shows how the AssignmentExtractor converts FHIR fixed[x] and pattern[x]
//! values into FSH assignment rules.

use maki_decompiler::{
    extractor::{AssignmentExtractor, RuleExtractor},
    models::{ElementDefinition, common::{CodeableConcept, Coding, Quantity}},
    processor::ProcessableElementDefinition,
};

fn main() {
    println!("=== Assignment Extractor Demo ===\n");

    // Example 1: Fixed boolean
    demo_fixed_boolean();

    // Example 2: Fixed code
    demo_fixed_code();

    // Example 3: Fixed Coding with system and display
    demo_fixed_coding();

    // Example 4: Fixed CodeableConcept with multiple codings
    demo_fixed_codeable_concept();

    // Example 5: Fixed Quantity
    demo_fixed_quantity();

    // Example 6: Pattern values
    demo_pattern_values();

    println!("\n=== Demo Complete ===");
}

fn demo_fixed_boolean() {
    println!("1. Fixed Boolean Value:");
    println!("   FHIR: fixedBoolean = true");

    let mut elem = create_element("Patient.active");
    elem.element.fixed_boolean = Some(true);

    let extractor = AssignmentExtractor;
    let rules = extractor.extract(&mut elem).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!();
}

fn demo_fixed_code() {
    println!("2. Fixed Code Value:");
    println!("   FHIR: fixedCode = 'active'");

    let mut elem = create_element("Patient.status");
    elem.element.fixed_code = Some("active".to_string());

    let extractor = AssignmentExtractor;
    let rules = extractor.extract(&mut elem).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!();
}

fn demo_fixed_coding() {
    println!("3. Fixed Coding Value:");
    println!("   FHIR: fixedCoding = {{ system: 'http://hl7.org/fhir/status', code: 'active', display: 'Active' }}");

    let mut elem = create_element("Observation.status");
    elem.element.fixed_coding = Some(Coding {
        system: Some("http://hl7.org/fhir/observation-status".to_string()),
        version: None,
        code: Some("final".to_string()),
        display: Some("Final".to_string()),
    });

    let extractor = AssignmentExtractor;
    let rules = extractor.extract(&mut elem).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!();
}

fn demo_fixed_codeable_concept() {
    println!("4. Fixed CodeableConcept (multiple codings):");
    println!("   FHIR: fixedCodeableConcept with multiple codings");

    let mut elem = create_element("Observation.code");
    elem.element.fixed_codeable_concept = Some(CodeableConcept {
        coding: Some(vec![
            Coding {
                system: Some("http://loinc.org".to_string()),
                version: None,
                code: Some("8867-4".to_string()),
                display: Some("Heart rate".to_string()),
            },
            Coding {
                system: Some("http://snomed.info/sct".to_string()),
                version: None,
                code: Some("364075005".to_string()),
                display: Some("Heart rate (observable entity)".to_string()),
            },
        ]),
        text: Some("Heart Rate".to_string()),
    });

    let extractor = AssignmentExtractor;
    let rules = extractor.extract(&mut elem).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!();
}

fn demo_fixed_quantity() {
    println!("5. Fixed Quantity Value:");
    println!("   FHIR: fixedQuantity = {{ value: 5.0, unit: 'mg' }}");

    let mut elem = create_element("Medication.amount");
    elem.element.fixed_quantity = Some(Quantity {
        value: Some(5.0),
        unit: Some("mg".to_string()),
        system: None,
        code: None,
    });

    let extractor = AssignmentExtractor;
    let rules = extractor.extract(&mut elem).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!();
}

fn demo_pattern_values() {
    println!("6. Pattern Values (vs Fixed):");
    println!("   FHIR: patternCode = 'draft' (allows additional properties)");

    let mut elem = create_element("Composition.status");
    elem.element.pattern_code = Some("draft".to_string());

    let extractor = AssignmentExtractor;
    let rules = extractor.extract(&mut elem).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!("   Note: Pattern values use same syntax but allow flexibility");
    println!();
}

fn create_element(path: &str) -> ProcessableElementDefinition {
    ProcessableElementDefinition::new(ElementDefinition {
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
    })
}
