//! Integration tests for SUSHI configuration parser
//!
//! These tests verify that maki can correctly parse and validate
//! sushi-config.yaml files, maintaining 100% compatibility with SUSHI.

use maki_core::config::SushiConfiguration;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sushi-config")
        .join(name)
}

#[test]
fn test_parse_minimal_config() {
    let path = fixture_path("minimal.yaml");
    let config = SushiConfiguration::from_file(&path)
        .expect("Failed to parse minimal config");

    assert_eq!(config.canonical, "http://example.org/fhir/minimal-ig");
    assert_eq!(config.fhir_version, vec!["4.0.1"]);

    // Validation should pass
    config.validate().expect("Minimal config should be valid");
}

#[test]
fn test_parse_complete_config() {
    let path = fixture_path("complete.yaml");
    let config = SushiConfiguration::from_file(&path)
        .expect("Failed to parse complete config");

    // Test core metadata
    assert_eq!(config.canonical, "http://example.org/fhir/example-ig");
    assert_eq!(config.fhir_version, vec!["4.0.1"]);
    assert_eq!(config.id, Some("example.fhir.ig".to_string()));
    assert_eq!(config.name, Some("ExampleIG".to_string()));
    assert_eq!(config.title, Some("Example Implementation Guide".to_string()));
    assert_eq!(config.version, Some("1.0.0".to_string()));
    assert_eq!(config.status, Some("draft".to_string()));
    assert_eq!(config.experimental, Some(true));
    assert_eq!(config.date, Some("2024-01-01".to_string()));
    assert_eq!(config.publisher, Some("Example Organization".to_string()));
    assert_eq!(config.license, Some("CC0-1.0".to_string()));

    // Test contact details
    let contact = config.contact.as_ref().expect("Should have contact");
    assert_eq!(contact.len(), 1);
    assert_eq!(contact[0].name, Some("Example Contact".to_string()));
    let telecom = contact[0].telecom.as_ref().expect("Should have telecom");
    assert_eq!(telecom.len(), 2);
    assert_eq!(telecom[0].system, "email");
    assert_eq!(telecom[0].value, "contact@example.org");

    // Test dependencies
    let deps = config.dependencies.as_ref().expect("Should have dependencies");
    assert!(deps.contains_key("hl7.fhir.us.core"));
    assert!(deps.contains_key("hl7.fhir.uv.extensions"));

    // Test global profiles
    let global = config.global.as_ref().expect("Should have global profiles");
    assert_eq!(global.len(), 1);
    assert_eq!(global[0].resource_type, "Patient");
    assert_eq!(global[0].profile, "http://example.org/fhir/StructureDefinition/example-patient");

    // Test resource groups
    let groups = config.groups.as_ref().expect("Should have groups");
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].id, "profiles");
    assert_eq!(groups[0].name, "Profiles");
    assert!(groups[0].description.is_some());

    // Test parameters
    let params = config.parameters.as_ref().expect("Should have parameters");
    assert!(params.iter().any(|p| p.code == "copyrightyear"));

    // Test pages
    let pages = config.pages.as_ref().expect("Should have pages");
    assert_eq!(pages.len(), 3);

    // Test menu
    let menu = config.menu.as_ref().expect("Should have menu");
    assert_eq!(menu.len(), 3);
    assert_eq!(menu[0].name, "Home");
    assert_eq!(menu[2].name, "Support");
    let submenu = menu[2].sub_menu.as_ref().expect("Support should have submenu");
    assert_eq!(submenu.len(), 2);

    // Test SUSHI options
    assert_eq!(config.fsh_only, Some(false));
    assert_eq!(config.apply_extension_metadata_to_root, Some(true));

    // Test instance options
    let inst_opts = config.instance_options.as_ref().expect("Should have instance options");
    assert!(inst_opts.set_meta_profile.is_some());
    assert!(inst_opts.set_id.is_some());
    assert_eq!(inst_opts.manual_slice_ordering, Some(false));

    // Validation should pass
    config.validate().expect("Complete config should be valid");
}

#[test]
fn test_parse_multiple_fhir_versions() {
    let path = fixture_path("multiple-versions.yaml");
    let config = SushiConfiguration::from_file(&path)
        .expect("Failed to parse multi-version config");

    assert_eq!(config.fhir_version, vec!["4.0.1", "4.3.0", "5.0.0"]);
    assert_eq!(config.status, Some("active".to_string()));

    // Validation should pass
    config.validate().expect("Multi-version config should be valid");
}

#[test]
fn test_parse_from_yaml_string() {
    let yaml = r#"
canonical: http://example.org/fhir/string-test
fhirVersion: 4.0.1
name: StringTest
"#;

    let config = SushiConfiguration::from_yaml(yaml)
        .expect("Failed to parse YAML string");

    assert_eq!(config.canonical, "http://example.org/fhir/string-test");
    assert_eq!(config.name, Some("StringTest".to_string()));
}

#[test]
fn test_validation_missing_canonical() {
    let path = fixture_path("invalid-missing-canonical.yaml");

    // Parsing should succeed (canonical is optional in the type)
    // but validation should fail
    let result = SushiConfiguration::from_file(&path);

    // The file doesn't have canonical, so serde will fail to deserialize
    // since canonical is required in the struct
    assert!(result.is_err(), "Should fail to parse without canonical");
}

#[test]
fn test_validation_invalid_status() {
    let path = fixture_path("invalid-bad-status.yaml");
    let config = SushiConfiguration::from_file(&path)
        .expect("Should parse despite invalid status");

    // Validation should fail due to invalid status
    let result = config.validate();
    assert!(result.is_err(), "Validation should fail with invalid status");

    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| e.contains("status")),
        "Should have status error"
    );
}

#[test]
fn test_validation_invalid_canonical_url() {
    let yaml = r#"
canonical: not-a-url
fhirVersion: 4.0.1
"#;

    let config = SushiConfiguration::from_yaml(yaml)
        .expect("Should parse YAML");

    let result = config.validate();
    assert!(result.is_err(), "Validation should fail with invalid canonical URL");

    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| e.contains("canonical")),
        "Should have canonical URL error"
    );
}

#[test]
fn test_validation_invalid_fhir_version() {
    let yaml = r#"
canonical: http://example.org/fhir/test
fhirVersion: not-a-version
"#;

    let config = SushiConfiguration::from_yaml(yaml)
        .expect("Should parse YAML");

    let result = config.validate();
    assert!(result.is_err(), "Validation should fail with invalid FHIR version");

    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| e.contains("invalid FHIR version")),
        "Should have FHIR version error"
    );
}

#[test]
fn test_package_id_fallback() {
    let yaml = r#"
canonical: http://example.org/fhir/test
fhirVersion: 4.0.1
id: test.ig
"#;

    let config = SushiConfiguration::from_yaml(yaml)
        .expect("Should parse YAML");

    // When packageId is not set, it should fall back to id
    assert_eq!(config.package_id(), Some("test.ig"));
}

#[test]
fn test_package_id_explicit() {
    let yaml = r#"
canonical: http://example.org/fhir/test
fhirVersion: 4.0.1
id: test.ig
packageId: custom.package.id
"#;

    let config = SushiConfiguration::from_yaml(yaml)
        .expect("Should parse YAML");

    // When packageId is explicitly set, it should use that
    assert_eq!(config.package_id(), Some("custom.package.id"));
}

#[test]
fn test_dependency_simple_version() {
    let yaml = r#"
canonical: http://example.org/fhir/test
fhirVersion: 4.0.1
dependencies:
  hl7.fhir.us.core: 5.0.1
"#;

    let config = SushiConfiguration::from_yaml(yaml)
        .expect("Should parse YAML");

    let deps = config.dependencies.as_ref().expect("Should have dependencies");
    let core_dep = deps.get("hl7.fhir.us.core").expect("Should have us-core dependency");

    match core_dep {
        maki_core::config::DependencyVersion::Simple(version) => {
            assert_eq!(version, "5.0.1");
        }
        _ => panic!("Expected simple version"),
    }
}

#[test]
fn test_all_valid_statuses() {
    for status in &["draft", "active", "retired", "unknown"] {
        let yaml = format!(
            r#"
canonical: http://example.org/fhir/test
fhirVersion: 4.0.1
status: {}
"#,
            status
        );

        let config = SushiConfiguration::from_yaml(&yaml)
            .expect("Should parse YAML");

        assert!(
            config.validate().is_ok(),
            "Status '{}' should be valid",
            status
        );
    }
}

#[test]
fn test_copyright_and_description_multiline() {
    let yaml = r#"
canonical: http://example.org/fhir/test
fhirVersion: 4.0.1
description: |
  This is a multiline
  description that spans
  multiple lines.
copyright: |
  Copyright 2024 Example Org
  All rights reserved
"#;

    let config = SushiConfiguration::from_yaml(yaml)
        .expect("Should parse YAML");

    assert!(config.description.is_some());
    assert!(config.copyright.is_some());

    let desc = config.description.unwrap();
    assert!(desc.contains("multiline"));
    assert!(desc.contains("multiple lines"));
}

#[test]
fn test_instance_options_enums() {
    let yaml = r#"
canonical: http://example.org/fhir/test
fhirVersion: 4.0.1
instanceOptions:
  setMetaProfile: inline-only
  setId: standalone-only
  manualSliceOrdering: true
"#;

    let config = SushiConfiguration::from_yaml(yaml)
        .expect("Should parse YAML");

    let opts = config.instance_options.as_ref().expect("Should have instance options");
    assert!(matches!(
        opts.set_meta_profile,
        Some(maki_core::config::MetaProfileSetting::InlineOnly)
    ));
    assert!(matches!(
        opts.set_id,
        Some(maki_core::config::IdSetting::StandaloneOnly)
    ));
    assert_eq!(opts.manual_slice_ordering, Some(true));
}

#[test]
fn test_real_world_us_core_example() {
    let path = fixture_path("us-core-example.yaml");
    let config = SushiConfiguration::from_file(&path)
        .expect("Failed to parse US Core example config");

    // Verify core metadata
    assert_eq!(config.canonical, "http://hl7.org/fhir/us/core");
    assert_eq!(config.id, Some("hl7.fhir.us.core".to_string()));
    assert_eq!(config.name, Some("USCoreImplementationGuide".to_string()));
    assert_eq!(config.title, Some("US Core Implementation Guide".to_string()));
    assert_eq!(config.version, Some("5.0.1".to_string()));
    assert_eq!(config.status, Some("active".to_string()));
    assert_eq!(config.license, Some("CC0-1.0".to_string()));

    // Verify publisher and contact
    assert_eq!(
        config.publisher,
        Some("HL7 International - US Realm Steering Committee".to_string())
    );
    let contact = config.contact.as_ref().expect("Should have contact");
    assert_eq!(contact.len(), 1);

    // Verify jurisdiction
    let jurisdiction = config.jurisdiction.as_ref().expect("Should have jurisdiction");
    assert_eq!(jurisdiction.len(), 1);
    let coding = jurisdiction[0]
        .coding
        .as_ref()
        .expect("Should have coding");
    assert_eq!(coding[0].system, Some("urn:iso:std:iso:3166".to_string()));
    assert_eq!(coding[0].code, Some("US".to_string()));

    // Verify dependencies
    let deps = config.dependencies.as_ref().expect("Should have dependencies");
    assert!(deps.contains_key("hl7.fhir.uv.extensions"));

    // Verify global profiles
    let global = config.global.as_ref().expect("Should have global profiles");
    assert_eq!(global.len(), 3);
    assert!(global.iter().any(|g| g.resource_type == "Patient"));
    assert!(global.iter().any(|g| g.resource_type == "AllergyIntolerance"));
    assert!(global.iter().any(|g| g.resource_type == "Observation"));

    // Verify parameters
    let params = config.parameters.as_ref().expect("Should have parameters");
    assert!(params.iter().any(|p| p.code == "copyrightyear"));
    assert!(params.iter().any(|p| p.code == "releaselabel"));
    assert!(params.iter().any(|p| p.code == "active-tables" && p.value == "true"));

    // Verify SUSHI options
    assert_eq!(config.fsh_only, Some(false));
    assert_eq!(config.apply_extension_metadata_to_root, Some(true));

    // Validation should pass
    config.validate().expect("US Core example should be valid");
}
