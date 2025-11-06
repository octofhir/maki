//! Integration tests for PathResolver
//!
//! These tests verify the path resolution algorithm matches SUSHI behavior
//! using real FHIR StructureDefinitions.

use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
use maki_core::semantic::PathResolver;
use std::sync::Arc;
use tempfile::TempDir;

/// Create a test canonical facade with R4 core package
async fn create_test_facade() -> (CanonicalFacade, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = octofhir_canonical_manager::config::FcmConfig::test_config(temp_dir.path());

    // Use public FHIR registry
    config.registry.url = "https://packages.fhir.org/".to_string();

    let options = CanonicalOptions {
        config: Some(config),
        default_release: FhirRelease::R4,
        auto_install_core: true,
        quick_init: false,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    (facade, temp_dir)
}

#[tokio::test]
#[ignore] // Requires network access to download FHIR packages
async fn test_resolve_simple_path() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test simple path in Patient resource
    let element = resolver
        .resolve_path("Patient", "name")
        .await
        .expect("Should resolve Patient.name");

    assert_eq!(element.path(), Some("Patient.name"));
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_resolve_nested_path() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test nested path requiring unfolding
    let element = resolver
        .resolve_path("Patient", "name.given")
        .await
        .expect("Should resolve Patient.name.given");

    assert_eq!(element.path(), Some("HumanName.given"));
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_resolve_deeply_nested_path() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test deeply nested path
    let element = resolver
        .resolve_path("Patient", "contact.telecom.system")
        .await
        .expect("Should resolve Patient.contact.telecom.system");

    // The path should be contextualized
    assert!(element.path().unwrap().contains("system"));
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_resolve_choice_type() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test choice type resolution
    let element = resolver
        .resolve_path("Patient", "deceased[x]")
        .await
        .expect("Should resolve Patient.deceased[x]");

    // Should match the choice type element
    assert!(element.is_choice_type() || element.path().unwrap().starts_with("Patient.deceased"));
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_resolve_invalid_path() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test invalid path
    let result = resolver
        .resolve_path("Patient", "invalid.path.that.does.not.exist")
        .await;

    assert!(result.is_err());
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_cache_effectiveness() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // First resolution
    let _element1 = resolver.resolve_path("Patient", "name").await.unwrap();

    let (cache_size_before, _) = resolver.cache_stats();
    assert_eq!(cache_size_before, 1);

    // Second resolution (should hit cache)
    let _element2 = resolver.resolve_path("Patient", "name").await.unwrap();

    let (cache_size_after, _) = resolver.cache_stats();
    assert_eq!(cache_size_after, 1); // Cache size shouldn't increase

    // Different path
    let _element3 = resolver.resolve_path("Patient", "gender").await.unwrap();

    let (cache_size_final, _) = resolver.cache_stats();
    assert_eq!(cache_size_final, 2); // Cache should now have 2 entries
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_resolve_with_array_index() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test array index notation
    let element = resolver
        .resolve_path("Patient", "name[0]")
        .await
        .expect("Should resolve Patient.name[0]");

    // Array index should resolve to the base element
    assert_eq!(element.path(), Some("Patient.name"));
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_resolve_with_soft_index() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test soft indexing
    let element = resolver
        .resolve_path("Patient", "telecom[+]")
        .await
        .expect("Should resolve Patient.telecom[+]");

    assert_eq!(element.path(), Some("Patient.telecom"));
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_resolve_observation_paths() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test various Observation paths
    let value_x = resolver
        .resolve_path("Observation", "value[x]")
        .await
        .expect("Should resolve Observation.value[x]");
    assert!(value_x.is_choice_type() || value_x.path().unwrap().starts_with("Observation.value"));

    let code = resolver
        .resolve_path("Observation", "code")
        .await
        .expect("Should resolve Observation.code");
    assert_eq!(code.path(), Some("Observation.code"));

    let status = resolver
        .resolve_path("Observation", "status")
        .await
        .expect("Should resolve Observation.status");
    assert_eq!(status.path(), Some("Observation.status"));
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_resolve_multiple_resources() {
    let (facade, _temp_dir) = create_test_facade().await;
    let session = facade.session([FhirRelease::R4]).await.unwrap();
    session.ensure_core_packages().await.unwrap();

    let resolver = PathResolver::new(Arc::new(session));

    // Test resolving paths in different resource types
    let patient_name = resolver.resolve_path("Patient", "name").await.unwrap();
    assert_eq!(patient_name.path(), Some("Patient.name"));

    let observation_code = resolver.resolve_path("Observation", "code").await.unwrap();
    assert_eq!(observation_code.path(), Some("Observation.code"));

    let practitioner_name = resolver.resolve_path("Practitioner", "name").await.unwrap();
    assert_eq!(practitioner_name.path(), Some("Practitioner.name"));
}

#[tokio::test]
async fn test_path_segment_creation() {
    use maki_core::semantic::{Bracket, PathSegment, SoftIndexOp};

    let simple = PathSegment::new("name".to_string());
    assert_eq!(simple.base, "name");
    assert_eq!(simple.bracket, None);

    let with_index = PathSegment::with_bracket("name".to_string(), Bracket::Index(0));
    assert_eq!(with_index.base, "name");
    assert_eq!(with_index.bracket, Some(Bracket::Index(0)));

    let with_slice = PathSegment::with_bracket(
        "component".to_string(),
        Bracket::Slice("systolic".to_string()),
    );
    assert_eq!(with_slice.base, "component");
    assert_eq!(
        with_slice.bracket,
        Some(Bracket::Slice("systolic".to_string()))
    );

    let with_soft =
        PathSegment::with_bracket("telecom".to_string(), Bracket::Soft(SoftIndexOp::Increment));
    assert_eq!(with_soft.base, "telecom");
    assert_eq!(
        with_soft.bracket,
        Some(Bracket::Soft(SoftIndexOp::Increment))
    );
}

#[tokio::test]
async fn test_element_definition_json_parsing() {
    use maki_core::semantic::ElementDefinition;
    use serde_json::json;

    let element_json = json!({
        "id": "Patient.name",
        "path": "Patient.name",
        "sliceName": "officialName",
        "type": [
            {
                "code": "HumanName",
                "profile": ["http://hl7.org/fhir/StructureDefinition/HumanName"]
            }
        ]
    });

    let element = ElementDefinition::new(element_json);

    assert_eq!(element.id(), Some("Patient.name"));
    assert_eq!(element.path(), Some("Patient.name"));
    assert_eq!(element.slice_name(), Some("officialName"));

    let types = element.types();
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].code, "HumanName");
    assert!(types[0].profile.is_some());
}

#[tokio::test]
async fn test_structure_definition_json_parsing() {
    use maki_core::semantic::StructureDefinition;
    use serde_json::json;

    let sd_json = json!({
        "resourceType": "StructureDefinition",
        "url": "http://hl7.org/fhir/StructureDefinition/Patient",
        "type": "Patient",
        "snapshot": {
            "element": [
                {
                    "id": "Patient",
                    "path": "Patient"
                },
                {
                    "id": "Patient.name",
                    "path": "Patient.name",
                    "type": [{"code": "HumanName"}]
                }
            ]
        }
    });

    let sd = StructureDefinition {
        content: Arc::new(sd_json),
    };

    assert_eq!(
        sd.url(),
        Some("http://hl7.org/fhir/StructureDefinition/Patient")
    );
    assert_eq!(sd.type_name(), "Patient");

    let elements = sd.elements();
    assert_eq!(elements.len(), 2);

    let found = sd.find_element_by_path("Patient.name");
    assert!(found.is_some());
    assert_eq!(found.unwrap().path(), Some("Patient.name"));
}
