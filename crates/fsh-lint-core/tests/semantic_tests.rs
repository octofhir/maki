//! Tests for semantic analysis functionality

use fsh_lint_core::{
    Cardinality, Constraint, ConstraintType, DefaultSemanticAnalyzer, Element, ElementFlag,
    FhirResource, Location, Reference, ReferenceType, ResourceMetadata, ResourceType,
    SemanticAnalyzer, SemanticAnalyzerConfig, SemanticModel, Severity, Symbol, SymbolType,
    TypeInfo,
};
use std::path::PathBuf;

/// Helper function to create a test file path
fn test_file_path() -> PathBuf {
    PathBuf::from("test.fsh")
}

/// Helper function to create a test location
fn test_location() -> Location {
    Location::new(test_file_path(), 1, 1, 0, 10)
}

/// Helper function to create a test semantic model with sample data
fn create_test_model() -> SemanticModel {
    let mut model = SemanticModel::new(test_file_path());

    // Add a test profile
    let profile = FhirResource {
        resource_type: ResourceType::Profile,
        id: "MyPatient".to_string(),
        name: None,
        title: Some("My Patient Profile".to_string()),
        description: Some("A custom patient profile".to_string()),
        parent: Some("Patient".to_string()),
        elements: vec![Element {
            path: "name".to_string(),
            cardinality: Some(Cardinality {
                min: 1,
                max: Some(1),
            }),
            type_info: Some(TypeInfo {
                type_name: "HumanName".to_string(),
                profile: None,
                target_types: Vec::new(),
            }),
            constraints: Vec::new(),
            location: test_location(),
            flags: vec![ElementFlag::MustSupport],
        }],
        location: test_location(),
        metadata: ResourceMetadata {
            title: Some("My Patient Profile".to_string()),
            description: Some("A custom patient profile".to_string()),
            ..Default::default()
        },
    };

    model.add_resource(profile);

    // Add a test extension
    let extension = FhirResource {
        resource_type: ResourceType::Extension,
        id: "MyExtension".to_string(),
        name: None,
        title: Some("My Extension".to_string()),
        description: Some("A custom extension".to_string()),
        parent: None,
        elements: Vec::new(),
        location: test_location(),
        metadata: ResourceMetadata {
            title: Some("My Extension".to_string()),
            description: Some("A custom extension".to_string()),
            ..Default::default()
        },
    };

    model.add_resource(extension);

    model
}

#[test]
fn test_semantic_analyzer_creation() {
    let analyzer = DefaultSemanticAnalyzer::new();
    assert!(true); // Just test that creation works

    let config = SemanticAnalyzerConfig {
        strict_validation: false,
        resolve_cross_file_references: false,
        max_element_depth: 5,
    };
    let analyzer_with_config = DefaultSemanticAnalyzer::with_config(config);
    assert!(true); // Just test that creation with config works
}

#[test]
fn test_semantic_model_creation() {
    let model = create_test_model();

    assert_eq!(model.resources.len(), 2);

    // Check profile
    let profile = model.get_resource("MyPatient").expect("Profile not found");
    assert_eq!(profile.resource_type, ResourceType::Profile);
    assert_eq!(profile.id, "MyPatient");
    assert_eq!(profile.parent, Some("Patient".to_string()));
    assert_eq!(
        profile.metadata.title,
        Some("My Patient Profile".to_string())
    );
    assert_eq!(
        profile.metadata.description,
        Some("A custom patient profile".to_string())
    );

    // Check elements
    assert_eq!(profile.elements.len(), 1);

    let name_element = &profile.elements[0];
    assert_eq!(name_element.path, "name");
    assert_eq!(
        name_element.cardinality,
        Some(Cardinality {
            min: 1,
            max: Some(1)
        })
    );
    assert!(name_element.flags.contains(&ElementFlag::MustSupport));

    // Check extension
    let extension = model
        .get_resource("MyExtension")
        .expect("Extension not found");
    assert_eq!(extension.resource_type, ResourceType::Extension);
    assert_eq!(extension.id, "MyExtension");
    assert_eq!(extension.parent, None);
}

#[test]
fn test_resource_type_filtering() {
    let model = create_test_model();

    let profiles = model.get_resources_by_type(ResourceType::Profile);
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].id, "MyPatient");

    let extensions = model.get_resources_by_type(ResourceType::Extension);
    assert_eq!(extensions.len(), 1);
    assert_eq!(extensions[0].id, "MyExtension");

    let valuesets = model.get_resources_by_type(ResourceType::ValueSet);
    assert_eq!(valuesets.len(), 0);
}

#[test]
fn test_element_validation() {
    let model = create_test_model();
    let profile = model.get_resource("MyPatient").unwrap();
    let element = &profile.elements[0];

    // Test element properties
    assert_eq!(element.path, "name");
    assert_eq!(
        element.cardinality,
        Some(Cardinality {
            min: 1,
            max: Some(1)
        })
    );
    assert!(element.flags.contains(&ElementFlag::MustSupport));

    // Test type info
    assert!(element.type_info.is_some());
    let type_info = element.type_info.as_ref().unwrap();
    assert_eq!(type_info.type_name, "HumanName");
    assert!(type_info.target_types.is_empty());
}

#[test]
fn test_symbol_table_building() {
    let model = create_test_model();

    // Check that all resources are in the symbol table
    assert!(model.symbols().contains_symbol("MyPatient"));
    assert!(model.symbols().contains_symbol("MyExtension"));

    // Check symbol types
    let patient_symbol = model
        .symbols()
        .get_symbol("MyPatient")
        .expect("MyPatient symbol not found");
    assert_eq!(patient_symbol.symbol_type, SymbolType::Profile);

    let extension_symbol = model
        .symbols()
        .get_symbol("MyExtension")
        .expect("MyExtension symbol not found");
    assert_eq!(extension_symbol.symbol_type, SymbolType::Extension);

    // Check symbol names
    let symbol_names = model.symbols().symbol_names();
    assert!(symbol_names.contains(&&"MyPatient".to_string()));
    assert!(symbol_names.contains(&&"MyExtension".to_string()));
}

#[test]
fn test_reference_creation() {
    let mut model = create_test_model();

    // Add a reference manually to test reference functionality
    let reference = Reference {
        from: test_location(),
        target: "Patient".to_string(),
        reference_type: ReferenceType::Parent,
        is_resolved: false,
    };

    model.add_reference(reference);

    // Should have the reference we added
    let parent_refs: Vec<_> = model
        .references
        .iter()
        .filter(|r| r.reference_type == ReferenceType::Parent)
        .collect();
    assert_eq!(parent_refs.len(), 1);
    assert_eq!(parent_refs[0].target, "Patient");
    assert!(!parent_refs[0].is_resolved);
}

#[test]
fn test_cardinality_parsing() {
    let test_cases = vec![
        (
            "1..1",
            Some(Cardinality {
                min: 1,
                max: Some(1),
            }),
        ),
        (
            "0..1",
            Some(Cardinality {
                min: 0,
                max: Some(1),
            }),
        ),
        ("1..*", Some(Cardinality { min: 1, max: None })),
        ("0..*", Some(Cardinality { min: 0, max: None })),
        (
            "5..10",
            Some(Cardinality {
                min: 5,
                max: Some(10),
            }),
        ),
    ];

    let analyzer = DefaultSemanticAnalyzer::new();

    for (cardinality_str, expected) in test_cases {
        let result = analyzer
            .parse_cardinality(cardinality_str)
            .expect("Failed to parse cardinality");
        assert_eq!(
            result, expected,
            "Failed for cardinality: {}",
            cardinality_str
        );
    }
}

#[test]
fn test_reference_target_extraction() {
    let analyzer = DefaultSemanticAnalyzer::new();

    let test_cases = vec![
        ("Reference(Patient)", vec!["Patient".to_string()]),
        (
            "Reference(Patient | Practitioner)",
            vec!["Patient".to_string(), "Practitioner".to_string()],
        ),
        (
            "Reference(Patient|Practitioner|Organization)",
            vec![
                "Patient".to_string(),
                "Practitioner".to_string(),
                "Organization".to_string(),
            ],
        ),
        ("string", vec![]),
    ];

    for (type_name, expected) in test_cases {
        let result = analyzer
            .extract_reference_targets(type_name)
            .expect("Failed to extract reference targets");
        assert_eq!(result, expected, "Failed for type: {}", type_name);
    }
}

#[test]
fn test_semantic_validation_unresolved_references() {
    let mut model = create_test_model();

    // Add an unresolved reference
    let unresolved_ref = Reference {
        from: test_location(),
        target: "NonExistentParent".to_string(),
        reference_type: ReferenceType::Parent,
        is_resolved: false,
    };
    model.add_reference(unresolved_ref);

    let analyzer = DefaultSemanticAnalyzer::new();
    let diagnostics = analyzer.validate_semantics(&model);

    // Should have an unresolved reference error
    let unresolved_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule_id == "unresolved-reference")
        .collect();
    assert!(!unresolved_errors.is_empty());
    assert_eq!(unresolved_errors[0].severity, Severity::Error);
}

#[test]
fn test_semantic_validation_duplicate_ids() {
    let mut model = create_test_model();

    // Add another resource with the same ID
    let duplicate_profile = FhirResource {
        resource_type: ResourceType::Profile,
        id: "MyPatient".to_string(), // Same ID as existing resource
        name: None,
        title: Some("Duplicate Patient Profile".to_string()),
        description: Some("A duplicate patient profile".to_string()),
        parent: Some("Patient".to_string()),
        elements: Vec::new(),
        location: test_location(),
        metadata: ResourceMetadata::default(),
    };

    model.add_resource(duplicate_profile);

    let analyzer = DefaultSemanticAnalyzer::new();
    let diagnostics = analyzer.validate_semantics(&model);

    // Should have a duplicate ID error
    let duplicate_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule_id == "duplicate-resource-id")
        .collect();
    assert!(!duplicate_errors.is_empty());
    assert_eq!(duplicate_errors[0].severity, Severity::Error);
}

#[test]
fn test_semantic_validation_missing_metadata() {
    let mut model = SemanticModel::new(test_file_path());

    // Add a profile without title and description
    let profile_without_metadata = FhirResource {
        resource_type: ResourceType::Profile,
        id: "IncompleteProfile".to_string(),
        name: None,
        title: None,
        description: None,
        parent: Some("Patient".to_string()),
        elements: Vec::new(),
        location: test_location(),
        metadata: ResourceMetadata::default(), // No title or description
    };

    model.add_resource(profile_without_metadata);

    let analyzer = DefaultSemanticAnalyzer::new();
    let diagnostics = analyzer.validate_semantics(&model);

    // Should have warnings for missing title and description
    let missing_title: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule_id == "missing-title")
        .collect();
    let missing_description: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule_id == "missing-description")
        .collect();

    assert!(!missing_title.is_empty());
    assert!(!missing_description.is_empty());
    assert_eq!(missing_title[0].severity, Severity::Warning);
    assert_eq!(missing_description[0].severity, Severity::Warning);
}

#[test]
fn test_semantic_validation_invalid_cardinality() {
    let mut model = SemanticModel::new(test_file_path());

    // Add a profile with invalid cardinality
    let profile_with_invalid_cardinality = FhirResource {
        resource_type: ResourceType::Profile,
        id: "InvalidProfile".to_string(),
        name: None,
        title: Some("Invalid Profile".to_string()),
        description: Some("A profile with invalid cardinality".to_string()),
        parent: Some("Patient".to_string()),
        elements: vec![Element {
            path: "name".to_string(),
            cardinality: Some(Cardinality {
                min: 5,
                max: Some(1),
            }), // Invalid: min > max
            type_info: None,
            constraints: Vec::new(),
            location: test_location(),
            flags: Vec::new(),
        }],
        location: test_location(),
        metadata: ResourceMetadata {
            title: Some("Invalid Profile".to_string()),
            description: Some("A profile with invalid cardinality".to_string()),
            ..Default::default()
        },
    };

    model.add_resource(profile_with_invalid_cardinality);

    let analyzer = DefaultSemanticAnalyzer::new();
    let diagnostics = analyzer.validate_semantics(&model);

    // Should have an invalid cardinality error
    let cardinality_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule_id == "invalid-cardinality")
        .collect();
    assert!(!cardinality_errors.is_empty());
    assert_eq!(cardinality_errors[0].severity, Severity::Error);
}

#[test]
fn test_semantic_validation_profile_missing_parent() {
    let mut model = SemanticModel::new(test_file_path());

    // Add a profile without parent
    let profile_without_parent = FhirResource {
        resource_type: ResourceType::Profile,
        id: "OrphanProfile".to_string(),
        name: None,
        title: Some("Orphan Profile".to_string()),
        description: Some("A profile without parent".to_string()),
        parent: None, // Missing parent
        elements: Vec::new(),
        location: test_location(),
        metadata: ResourceMetadata {
            title: Some("Orphan Profile".to_string()),
            description: Some("A profile without parent".to_string()),
            ..Default::default()
        },
    };

    model.add_resource(profile_without_parent);

    let analyzer = DefaultSemanticAnalyzer::new();
    let diagnostics = analyzer.validate_semantics(&model);

    // Should have a missing parent error
    let parent_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule_id == "profile-missing-parent")
        .collect();
    assert!(!parent_errors.is_empty());
    assert_eq!(parent_errors[0].severity, Severity::Error);
}

#[test]
fn test_resource_id_validation() {
    let analyzer = DefaultSemanticAnalyzer::new();

    let valid_ids = vec!["MyPatient", "Patient-Profile", "a1b2c3", "ValidID"];
    let invalid_ids = vec![
        "",
        "123Invalid",
        "-InvalidStart",
        "Invalid.ID",
        "Invalid ID",
    ];

    for id in valid_ids {
        assert!(analyzer.is_valid_resource_id(id), "Should be valid: {}", id);
    }

    for id in invalid_ids {
        assert!(
            !analyzer.is_valid_resource_id(id),
            "Should be invalid: {}",
            id
        );
    }
}

#[test]
fn test_element_path_validation() {
    let analyzer = DefaultSemanticAnalyzer::new();

    let valid_paths = vec![
        "name",
        "name.given",
        "contact.telecom.value",
        "extension.value",
    ];
    let invalid_paths = vec!["", ".name", "name.", "name..given", "name."];

    for path in valid_paths {
        assert!(
            analyzer.is_valid_element_path(path),
            "Should be valid: {}",
            path
        );
    }

    for path in invalid_paths {
        assert!(
            !analyzer.is_valid_element_path(path),
            "Should be invalid: {}",
            path
        );
    }
}

#[test]
fn test_fhir_type_validation() {
    let analyzer = DefaultSemanticAnalyzer::new();

    let valid_types = vec![
        "string",
        "boolean",
        "integer",
        "Patient",
        "Practitioner",
        "Reference(Patient)",
        "Reference(Patient | Practitioner)",
    ];

    for type_name in valid_types {
        assert!(
            analyzer.is_valid_fhir_type(type_name),
            "Should be valid: {}",
            type_name
        );
    }
}

#[test]
fn test_semantic_model_resource_queries() {
    let model = create_test_model();

    // Test get_resource
    let patient_resource = model.get_resource("MyPatient");
    assert!(patient_resource.is_some());
    assert_eq!(
        patient_resource.unwrap().resource_type,
        ResourceType::Profile
    );

    let nonexistent_resource = model.get_resource("NonExistent");
    assert!(nonexistent_resource.is_none());

    // Test get_resources_by_type
    let profiles = model.get_resources_by_type(ResourceType::Profile);
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].id, "MyPatient");

    let extensions = model.get_resources_by_type(ResourceType::Extension);
    assert_eq!(extensions.len(), 1);
    assert_eq!(extensions[0].id, "MyExtension");

    let valuesets = model.get_resources_by_type(ResourceType::ValueSet);
    assert_eq!(valuesets.len(), 0);

    let instances = model.get_resources_by_type(ResourceType::Instance);
    assert_eq!(instances.len(), 0);
}

#[test]
fn test_symbol_table_file_tracking() {
    let model = create_test_model();

    let file_symbols = model.symbols().get_symbols_in_file(&test_file_path());
    assert_eq!(file_symbols.len(), 2); // MyPatient and MyExtension

    let symbol_names: Vec<&str> = file_symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(symbol_names.contains(&"MyPatient"));
    assert!(symbol_names.contains(&"MyExtension"));

    let other_file = PathBuf::from("other.fsh");
    let other_file_symbols = model.symbols().get_symbols_in_file(&other_file);
    assert_eq!(other_file_symbols.len(), 0);
}

#[test]
fn test_reference_resolution() {
    let mut model = create_test_model();

    // Add references - one resolvable, one not
    let resolvable_ref = Reference {
        from: test_location(),
        target: "MyPatient".to_string(), // This exists in the model
        reference_type: ReferenceType::Type,
        is_resolved: false,
    };

    let unresolvable_ref = Reference {
        from: test_location(),
        target: "NonExistentResource".to_string(), // This doesn't exist
        reference_type: ReferenceType::Type,
        is_resolved: false,
    };

    model.add_reference(resolvable_ref);
    model.add_reference(unresolvable_ref);

    let analyzer = DefaultSemanticAnalyzer::new();

    // Before resolution, both references should be unresolved
    let unresolved_before = model.unresolved_references().len();
    assert_eq!(unresolved_before, 2);

    // Resolve references
    analyzer
        .resolve_references(&mut model)
        .expect("Failed to resolve references");

    // After resolution, one reference should be resolved
    let unresolved_after = model.unresolved_references().len();
    assert_eq!(unresolved_after, 1); // Only the unresolvable one remains
}

#[test]
fn test_constraint_handling() {
    let mut model = SemanticModel::new(test_file_path());

    // Create an element with constraints
    let element_with_constraints = Element {
        path: "name".to_string(),
        cardinality: Some(Cardinality {
            min: 1,
            max: Some(1),
        }),
        type_info: Some(TypeInfo {
            type_name: "HumanName".to_string(),
            profile: None,
            target_types: Vec::new(),
        }),
        constraints: vec![
            Constraint {
                constraint_type: ConstraintType::FixedValue,
                value: "Fixed Name".to_string(),
                location: test_location(),
            },
            Constraint {
                constraint_type: ConstraintType::Binding,
                value: "MyValueSet".to_string(),
                location: test_location(),
            },
        ],
        location: test_location(),
        flags: vec![ElementFlag::MustSupport],
    };

    let profile = FhirResource {
        resource_type: ResourceType::Profile,
        id: "TestProfile".to_string(),
        name: None,
        title: Some("Test Profile".to_string()),
        description: Some("A test profile with constraints".to_string()),
        parent: Some("Patient".to_string()),
        elements: vec![element_with_constraints],
        location: test_location(),
        metadata: ResourceMetadata {
            title: Some("Test Profile".to_string()),
            description: Some("A test profile with constraints".to_string()),
            ..Default::default()
        },
    };

    model.add_resource(profile);

    let resource = &model.resources[0];
    let element = &resource.elements[0];

    // Check constraints
    assert_eq!(element.constraints.len(), 2);
    assert!(
        element
            .constraints
            .iter()
            .any(|c| c.constraint_type == ConstraintType::FixedValue)
    );
    assert!(
        element
            .constraints
            .iter()
            .any(|c| c.constraint_type == ConstraintType::Binding)
    );

    // Check flags
    assert!(element.flags.contains(&ElementFlag::MustSupport));
}
