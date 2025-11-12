//! Demonstration of ContainsExtractor and ObeysExtractor functionality
//!
//! This example shows how the advanced extractors handle slicing and constraints.

use maki_decompiler::{
    extractor::{ContainsExtractor, ObeysExtractor, RuleExtractor},
    exportable::ExportableRule,
    models::{ElementDefinition, common::{Slicing, Constraint, TypeRef}},
    processor::ProcessableElementDefinition,
};

fn main() {
    println!("=== Advanced Extractors Demo ===\n");

    // Example 1: Simple slicing
    demo_simple_slicing();

    // Example 2: Extension slicing with URLs
    demo_extension_slicing();

    // Example 3: Element-level constraints
    demo_element_constraints();

    // Example 4: Resource-level constraints
    demo_resource_constraints();

    println!("\n=== Demo Complete ===");
}

fn demo_simple_slicing() {
    println!("1. Simple Slicing (identifier):");
    println!("   FHIR: Patient.identifier with slicing entry + 2 slices (mrn, ssn)");

    let mut elements = vec![
        // Slicing entry
        {
            let mut elem = create_element("Patient.identifier", None);
            elem.element.slicing = Some(Slicing {
                discriminator: None,
                description: Some("Slice by identifier type".to_string()),
                ordered: None,
                rules: None,
            });
            elem
        },
        // Slice 1: MRN (required)
        {
            let mut elem = create_element("Patient.identifier", Some("mrn".to_string()));
            elem.element.min = Some(1);
            elem.element.max = Some("1".to_string());
            elem
        },
        // Slice 2: SSN (optional)
        {
            let mut elem = create_element("Patient.identifier", Some("ssn".to_string()));
            elem.element.min = Some(0);
            elem.element.max = Some("1".to_string());
            elem
        },
    ];

    let rules = ContainsExtractor::extract_slicing(&mut elements).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!();
}

fn demo_extension_slicing() {
    println!("2. Extension Slicing with URL:");
    println!("   FHIR: Patient.extension with US Core race extension");

    let mut elements = vec![
        // Slicing entry
        {
            let mut elem = create_element("Patient.extension", None);
            elem.element.slicing = Some(Slicing {
                discriminator: None,
                description: Some("Extensions are sliced per their URL".to_string()),
                ordered: None,
                rules: None,
            });
            elem
        },
        // Extension slice with URL
        {
            let mut elem = create_element("Patient.extension", Some("race".to_string()));
            elem.element.min = Some(0);
            elem.element.max = Some("1".to_string());
            elem.element.type_ = Some(vec![TypeRef {
                code: "Extension".to_string(),
                profile: Some(vec![
                    "http://hl7.org/fhir/us/core/StructureDefinition/us-core-race".to_string(),
                ]),
                target_profile: None,
            }]);
            elem
        },
    ];

    let rules = ContainsExtractor::extract_slicing(&mut elements).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!();
}

fn demo_element_constraints() {
    println!("3. Element-level Constraints:");
    println!("   FHIR: Patient.identifier with constraint requiring system and value");

    let mut elem = create_element("Patient.identifier", None);
    elem.element.constraint = Some(vec![
        Constraint {
            key: "us-core-1".to_string(),
            severity: Some("error".to_string()),
            human: "Identifier must have both system and value".to_string(),
            expression: Some("system.exists() and value.exists()".to_string()),
            xpath: None,
        },
    ]);

    let extractor = ObeysExtractor;
    let rules = extractor.extract(&mut elem).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!();
}

fn demo_resource_constraints() {
    println!("4. Resource-level Constraints:");
    println!("   FHIR: Patient resource with root-level constraint");

    let mut elem = create_element("Patient", None);
    elem.element.constraint = Some(vec![
        Constraint {
            key: "pat-1".to_string(),
            severity: Some("error".to_string()),
            human: "SHALL at least have a name or identifier".to_string(),
            expression: Some("name.exists() or identifier.exists()".to_string()),
            xpath: None,
        },
    ]);

    let rules = ObeysExtractor::extract_root_constraints(&mut elem).unwrap();

    println!("   FSH:  {}", rules[0].to_fsh());
    println!("   Note: No path specified for resource-level constraints");
    println!();
}

fn create_element(path: &str, slice_name: Option<String>) -> ProcessableElementDefinition {
    ProcessableElementDefinition::new(ElementDefinition {
        id: if let Some(ref name) = slice_name {
            format!("{}:{}", path, name)
        } else {
            path.to_string()
        },
        path: path.to_string(),
        slice_name,
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
