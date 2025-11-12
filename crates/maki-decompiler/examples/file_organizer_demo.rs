//! File Organizer Demo
//!
//! This example demonstrates the different file organization strategies
//! available in the maki decompiler. It shows how to organize FSH files
//! using various patterns matching GoFSH's behavior.
//!
//! Run with: cargo run --example file_organizer_demo

use maki_decompiler::exportable::*;
use maki_decompiler::organizer::{FileOrganizer, OrganizationStrategy};
use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== File Organizer Demo ===\n");

    // Create sample exportables
    let exportables = create_sample_exportables();

    println!("Created {} exportables:", exportables.len());
    for exp in &exportables {
        println!("  - {} ({})", exp.name(), exp.id());
    }
    println!();

    // Demo 1: File Per Definition
    println!("1. FilePerDefinition Strategy:");
    println!("{}", "=".repeat(60));
    println!("Each exportable gets its own file in the root directory");
    println!();

    let organizer = FileOrganizer::new(OrganizationStrategy::FilePerDefinition);
    let output_dir = Path::new("demo-output/file-per-definition");
    organizer.organize(&exportables, output_dir)?;

    println!("✓ Files written to {}/", output_dir.display());
    show_directory_structure(output_dir)?;
    println!();

    // Demo 2: Group By FSH Type
    println!("2. GroupByFshType Strategy:");
    println!("{}", "=".repeat(60));
    println!("Files grouped by FSH type (profiles/, valuesets/, etc.)");
    println!();

    let organizer = FileOrganizer::new(OrganizationStrategy::GroupByFshType);
    let output_dir = Path::new("demo-output/group-by-type");
    organizer.organize(&exportables, output_dir)?;

    println!("✓ Files written to {}/", output_dir.display());
    show_directory_structure(output_dir)?;
    println!();

    // Demo 3: Group By Profile
    println!("3. GroupByProfile Strategy:");
    println!("{}", "=".repeat(60));
    println!("Profiles grouped by parent resource type");
    println!();

    let organizer = FileOrganizer::new(OrganizationStrategy::GroupByProfile);
    let output_dir = Path::new("demo-output/group-by-profile");
    organizer.organize(&exportables, output_dir)?;

    println!("✓ Files written to {}/", output_dir.display());
    show_directory_structure(output_dir)?;
    println!();

    // Demo 4: Single File
    println!("4. SingleFile Strategy:");
    println!("{}", "=".repeat(60));
    println!("All exportables combined into one file");
    println!();

    let organizer = FileOrganizer::new(OrganizationStrategy::SingleFile);
    let output_dir = Path::new("demo-output/single-file");
    organizer.organize(&exportables, output_dir)?;

    println!("✓ File written to {}/definitions.fsh", output_dir.display());

    // Show file size
    let file_path = output_dir.join("definitions.fsh");
    let metadata = fs::metadata(&file_path)?;
    println!("  File size: {} bytes", metadata.len());
    println!("  Lines: {}", count_lines(&file_path)?);
    println!();

    // Demo 5: Custom Configuration
    println!("5. Custom Configuration:");
    println!("{}", "=".repeat(60));
    println!("Using custom FshWriter with different formatting");
    println!();

    use maki_decompiler::writer::FshWriter;
    let custom_writer = FshWriter::new(4, 120); // 4 spaces indent, 120 line width
    let organizer = FileOrganizer::with_writer(
        OrganizationStrategy::FilePerDefinition,
        custom_writer,
    );
    let output_dir = Path::new("demo-output/custom-config");
    organizer.organize(&exportables, output_dir)?;

    println!("✓ Files written with custom formatting");
    println!("  Indent: 4 spaces");
    println!("  Line width: 120 chars");
    println!();

    println!("=== Demo Complete ===");
    println!("All output directories are in demo-output/");
    println!("Compare the different organization strategies:");
    println!("  - demo-output/file-per-definition/");
    println!("  - demo-output/group-by-type/");
    println!("  - demo-output/group-by-profile/");
    println!("  - demo-output/single-file/");
    println!("  - demo-output/custom-config/");

    Ok(())
}

/// Create sample exportables for the demo
fn create_sample_exportables() -> Vec<Box<dyn Exportable>> {
    let mut exportables: Vec<Box<dyn Exportable>> = Vec::new();

    // Patient profiles
    let mut us_core_patient = ExportableProfile::new(
        "USCorePatientProfile".to_string(),
        "Patient".to_string(),
    )
    .with_id("us-core-patient".to_string())
    .with_title("US Core Patient Profile".to_string())
    .with_description("US Core Patient Profile for demographics".to_string());

    us_core_patient.add_rule(Box::new(CardinalityRule {
        path: "identifier".to_string(),
        min: 1,
        max: "*".to_string(),
    }));

    us_core_patient.add_rule(Box::new(FlagRule {
        path: "identifier".to_string(),
        flags: vec![Flag::MustSupport],
    }));

    exportables.push(Box::new(us_core_patient));

    // Observation profile
    let mut vital_signs = ExportableProfile::new(
        "VitalSignsObservationProfile".to_string(),
        "Observation".to_string(),
    )
    .with_id("vital-signs".to_string())
    .with_title("Vital Signs Observation".to_string());

    vital_signs.add_rule(Box::new(CardinalityRule {
        path: "code".to_string(),
        min: 1,
        max: "1".to_string(),
    }));

    exportables.push(Box::new(vital_signs));

    // Condition profile
    let condition_profile = ExportableProfile::new(
        "USCoreConditionProfile".to_string(),
        "Condition".to_string(),
    )
    .with_id("us-core-condition".to_string())
    .with_title("US Core Condition Profile".to_string());

    exportables.push(Box::new(condition_profile));

    // ValueSets
    let mut gender_vs = ExportableValueSet::new(
        "AdministrativeGenderValueSet".to_string(),
    )
    .with_id("administrative-gender-vs".to_string())
    .with_title("Administrative Gender".to_string());

    gender_vs.add_rule(Box::new(IncludeRule {
        system: "http://hl7.org/fhir/administrative-gender".to_string(),
        version: None,
        concepts: vec![
            IncludeConcept {
                code: "male".to_string(),
                display: Some("Male".to_string()),
            },
            IncludeConcept {
                code: "female".to_string(),
                display: Some("Female".to_string()),
            },
        ],
        filters: vec![],
    }));

    exportables.push(Box::new(gender_vs));

    let observation_status_vs = ExportableValueSet::new(
        "ObservationStatusValueSet".to_string(),
    )
    .with_id("observation-status-vs".to_string())
    .with_title("Observation Status Codes".to_string());

    exportables.push(Box::new(observation_status_vs));

    // CodeSystems
    let mut example_cs = ExportableCodeSystem::new(
        "ExampleCodeSystem".to_string(),
    )
    .with_id("example-codes".to_string())
    .with_title("Example Codes".to_string());

    example_cs.add_rule(Box::new(LocalCodeRule {
        code: "active".to_string(),
        display: Some("Active".to_string()),
        definition: Some("The entity is active".to_string()),
    }));

    example_cs.add_rule(Box::new(LocalCodeRule {
        code: "inactive".to_string(),
        display: Some("Inactive".to_string()),
        definition: Some("The entity is inactive".to_string()),
    }));

    exportables.push(Box::new(example_cs));

    // Extension
    let extension = ExportableExtension::new(
        "BirthSexExtension".to_string(),
    )
    .with_id("birth-sex".to_string())
    .with_title("Birth Sex".to_string());

    exportables.push(Box::new(extension));

    // Logical model
    let logical = ExportableLogical::new(
        "PatientContactLogical".to_string(),
    )
    .with_id("patient-contact-logical".to_string())
    .with_title("Patient Contact Logical Model".to_string());

    exportables.push(Box::new(logical));

    exportables
}

/// Show the directory structure
fn show_directory_structure(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        println!("  (directory not created)");
        return Ok(());
    }

    show_directory_recursive(path, "", true)?;
    Ok(())
}

/// Recursively show directory structure
fn show_directory_recursive(
    path: &Path,
    prefix: &str,
    is_last: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if !name.is_empty() {
        let branch = if is_last { "└── " } else { "├── " };
        println!("{}{}{}", prefix, branch, name);
    }

    if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .collect();

        entries.sort_by_key(|e| e.path());

        let new_prefix = if name.is_empty() {
            prefix.to_string()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        for (i, entry) in entries.iter().enumerate() {
            let is_last_entry = i == entries.len() - 1;
            show_directory_recursive(&entry.path(), &new_prefix, is_last_entry)?;
        }
    }

    Ok(())
}

/// Count lines in a file
fn count_lines(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().count())
}
