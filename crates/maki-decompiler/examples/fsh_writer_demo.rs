//! FSH Writer Demo
//!
//! This example demonstrates how to use the FshWriter to create FSH files
//! from Exportable types. It shows various features including:
//!
//! - Creating different types of exportables (Profile, ValueSet, CodeSystem)
//! - Adding rules to exportables
//! - Writing to strings
//! - Writing to files
//! - Batch writing multiple exportables
//!
//! Run with: cargo run --example fsh_writer_demo

use maki_decompiler::exportable::*;
use maki_decompiler::writer::FshWriter;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== FSH Writer Demo ===\n");

    // Create a writer with default settings
    let writer = FshWriter::default();

    // Demo 1: Simple Profile
    println!("1. Simple Profile:");
    println!("{}", "=".repeat(60));
    let simple_profile = create_simple_profile();
    let fsh = writer.write(&simple_profile);
    println!("{}", fsh);

    // Demo 2: Complex Profile with Rules
    println!("\n2. Complex Profile with Rules:");
    println!("{}", "=".repeat(60));
    let complex_profile = create_complex_profile();
    let fsh = writer.write(&complex_profile);
    println!("{}", fsh);

    // Demo 3: ValueSet
    println!("\n3. ValueSet:");
    println!("{}", "=".repeat(60));
    let value_set = create_value_set();
    let fsh = writer.write(&value_set);
    println!("{}", fsh);

    // Demo 4: CodeSystem
    println!("\n4. CodeSystem:");
    println!("{}", "=".repeat(60));
    let code_system = create_code_system();
    let fsh = writer.write(&code_system);
    println!("{}", fsh);

    // Demo 5: Write to files
    println!("\n5. Writing to Files:");
    println!("{}", "=".repeat(60));
    let output_dir = Path::new("fsh-output");

    println!("Writing files to {}/", output_dir.display());
    writer.write_to_file(&simple_profile, &output_dir.join("SimpleProfile.fsh"))?;
    writer.write_to_file(&complex_profile, &output_dir.join("ComplexProfile.fsh"))?;
    writer.write_to_file(&value_set, &output_dir.join("MyValueSet.fsh"))?;
    writer.write_to_file(&code_system, &output_dir.join("MyCodeSystem.fsh"))?;
    println!("✓ Files written successfully");

    // Demo 6: Batch writing
    println!("\n6. Batch Writing:");
    println!("{}", "=".repeat(60));
    let batch_dir = Path::new("fsh-batch");
    let exportables: Vec<&dyn Exportable> =
        vec![&simple_profile, &complex_profile, &value_set, &code_system];
    writer.write_batch(&exportables, batch_dir)?;
    println!("✓ Batch written to {}/", batch_dir.display());

    // Demo 7: Custom writer configuration
    println!("\n7. Custom Writer Configuration:");
    println!("{}", "=".repeat(60));
    let custom_writer = FshWriter::new(4, 120); // 4 spaces indent, 120 line width
    println!("Indent size: {}", custom_writer.indent_size());
    println!("Line width: {}", custom_writer.line_width());
    let fsh = custom_writer.write(&simple_profile);
    println!("\nOutput with custom settings:");
    println!("{}", fsh);

    println!("\n=== Demo Complete ===");
    println!("Check the following directories for output files:");
    println!("  - fsh-output/");
    println!("  - fsh-batch/");

    Ok(())
}

/// Create a simple profile example
fn create_simple_profile() -> ExportableProfile {
    ExportableProfile::new("SimplePatient".to_string(), "Patient".to_string())
        .with_id("simple-patient".to_string())
        .with_title("Simple Patient Profile".to_string())
        .with_description("A minimal patient profile for demonstration".to_string())
}

/// Create a complex profile with various rules
fn create_complex_profile() -> ExportableProfile {
    let mut profile = ExportableProfile::new(
        "USCorePatient".to_string(),
        "Patient".to_string(),
    )
    .with_id("us-core-patient".to_string())
    .with_title("US Core Patient Profile".to_string())
    .with_description("This profile sets minimum expectations for the Patient resource to record, search, and fetch basic demographics and other administrative information about an individual patient.".to_string());

    // Cardinality rules
    profile.add_rule(Box::new(CardinalityRule {
        path: "identifier".to_string(),
        min: 1,
        max: "*".to_string(),
    }));

    profile.add_rule(Box::new(CardinalityRule {
        path: "name".to_string(),
        min: 1,
        max: "*".to_string(),
    }));

    profile.add_rule(Box::new(CardinalityRule {
        path: "telecom".to_string(),
        min: 0,
        max: "*".to_string(),
    }));

    profile.add_rule(Box::new(CardinalityRule {
        path: "gender".to_string(),
        min: 1,
        max: "1".to_string(),
    }));

    // Flag rules
    profile.add_rule(Box::new(FlagRule {
        path: "identifier".to_string(),
        flags: vec![Flag::MustSupport],
    }));

    profile.add_rule(Box::new(FlagRule {
        path: "name".to_string(),
        flags: vec![Flag::MustSupport],
    }));

    profile.add_rule(Box::new(FlagRule {
        path: "gender".to_string(),
        flags: vec![Flag::MustSupport, Flag::Summary],
    }));

    // Assignment rule
    profile.add_rule(Box::new(AssignmentRule {
        path: "active".to_string(),
        value: FshValue::Boolean(true),
        exactly: false,
    }));

    // Binding rule
    profile.add_rule(Box::new(BindingRule {
        path: "gender".to_string(),
        value_set: "http://hl7.org/fhir/ValueSet/administrative-gender".to_string(),
        strength: BindingStrength::Required,
    }));

    // Type rule
    profile.add_rule(Box::new(TypeRule {
        path: "contact.relationship".to_string(),
        types: vec![TypeReference {
            type_name: "CodeableConcept".to_string(),
            profiles: vec![],
            target_profiles: vec![],
        }],
    }));

    // Caret value rule for metadata
    profile.add_rule(Box::new(CaretValueRule {
        path: None,
        caret_path: "status".to_string(),
        value: FshValue::Code(FshCode {
            system: None,
            code: "active".to_string(),
        }),
    }));

    profile.add_rule(Box::new(CaretValueRule {
        path: None,
        caret_path: "experimental".to_string(),
        value: FshValue::Boolean(false),
    }));

    // Contains rule for extensions
    profile.add_rule(Box::new(ContainsRule {
        path: "extension".to_string(),
        items: vec![
            ContainsItem {
                name: "race".to_string(),
                type_name: Some(
                    "http://hl7.org/fhir/us/core/StructureDefinition/us-core-race".to_string(),
                ),
                min: 0,
                max: "1".to_string(),
            },
            ContainsItem {
                name: "ethnicity".to_string(),
                type_name: Some(
                    "http://hl7.org/fhir/us/core/StructureDefinition/us-core-ethnicity".to_string(),
                ),
                min: 0,
                max: "1".to_string(),
            },
        ],
    }));

    profile
}

/// Create a value set example
fn create_value_set() -> ExportableValueSet {
    let mut value_set = ExportableValueSet::new("PatientStatusValueSet".to_string())
        .with_id("patient-status-vs".to_string())
        .with_title("Patient Status Value Set".to_string())
        .with_description("A value set of possible patient statuses".to_string());

    // Include codes from a system
    value_set.add_rule(Box::new(IncludeRule {
        system: "http://terminology.hl7.org/CodeSystem/v3-ActStatus".to_string(),
        version: None,
        concepts: vec![
            IncludeConcept {
                code: "active".to_string(),
                display: Some("Active".to_string()),
            },
            IncludeConcept {
                code: "inactive".to_string(),
                display: Some("Inactive".to_string()),
            },
            IncludeConcept {
                code: "completed".to_string(),
                display: Some("Completed".to_string()),
            },
        ],
        filters: vec![],
    }));

    // Exclude some codes
    value_set.add_rule(Box::new(ExcludeRule {
        system: "http://terminology.hl7.org/CodeSystem/v3-ActStatus".to_string(),
        version: None,
        concepts: vec![IncludeConcept {
            code: "nullified".to_string(),
            display: None,
        }],
        filters: vec![],
    }));

    value_set
}

/// Create a code system example
fn create_code_system() -> ExportableCodeSystem {
    let mut code_system = ExportableCodeSystem::new("ExampleCodeSystem".to_string())
        .with_id("example-codes".to_string())
        .with_title("Example Code System".to_string())
        .with_description("An example code system for demonstration purposes".to_string());

    // Add local codes
    code_system.add_rule(Box::new(LocalCodeRule {
        code: "code1".to_string(),
        display: Some("First Code".to_string()),
        definition: Some("This is the first example code".to_string()),
    }));

    code_system.add_rule(Box::new(LocalCodeRule {
        code: "code2".to_string(),
        display: Some("Second Code".to_string()),
        definition: Some("This is the second example code".to_string()),
    }));

    code_system.add_rule(Box::new(LocalCodeRule {
        code: "code3".to_string(),
        display: Some("Third Code".to_string()),
        definition: Some("This is the third example code".to_string()),
    }));

    // Metadata
    code_system.add_rule(Box::new(CaretValueRule {
        path: None,
        caret_path: "caseSensitive".to_string(),
        value: FshValue::Boolean(true),
    }));

    code_system.add_rule(Box::new(CaretValueRule {
        path: None,
        caret_path: "content".to_string(),
        value: FshValue::Code(FshCode {
            system: None,
            code: "complete".to_string(),
        }),
    }));

    code_system
}
