//! Integration tests for the Fishable trait
//!
//! These tests verify that the Fishable implementation works correctly
//! with real FHIR packages and the canonical manager.

use maki_core::canonical::fishable::{FhirType, Fishable};
use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a minimal FHIR package tarball for testing
fn build_test_package() -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::Builder;

    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut tar = Builder::new(encoder);

    // package/package.json
    let package_json = serde_json::json!({
        "name": "test.fishable",
        "version": "1.0.0",
        "description": "Test package for Fishable",
        "fhirVersions": ["4.0.1"],
        "dependencies": {}
    })
    .to_string();

    let mut header = tar::Header::new_gnu();
    header.set_size(package_json.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "package/package.json", package_json.as_bytes())
        .unwrap();

    // package/.index.json
    let index_json = serde_json::json!({
        "index-version": "1.0",
        "files": {
            "package/StructureDefinition-TestProfile.json": {
                "resourceType": "StructureDefinition",
                "id": "TestProfile",
                "url": "http://test.org/fhir/StructureDefinition/TestProfile",
                "version": "1.0.0",
                "kind": "resource"
            },
            "package/ValueSet-TestValueSet.json": {
                "resourceType": "ValueSet",
                "id": "TestValueSet",
                "url": "http://test.org/fhir/ValueSet/TestValueSet",
                "version": "1.0.0"
            }
        }
    })
    .to_string();

    let mut header = tar::Header::new_gnu();
    header.set_size(index_json.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "package/.index.json", index_json.as_bytes())
        .unwrap();

    // package/StructureDefinition-TestProfile.json
    let profile_json = serde_json::json!({
        "resourceType": "StructureDefinition",
        "id": "TestProfile",
        "url": "http://test.org/fhir/StructureDefinition/TestProfile",
        "version": "1.0.0",
        "name": "TestProfile",
        "status": "draft",
        "kind": "resource",
        "derivation": "constraint",
        "type": "Patient",
        "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Patient"
    })
    .to_string();

    let mut header = tar::Header::new_gnu();
    header.set_size(profile_json.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(
        &mut header,
        "package/StructureDefinition-TestProfile.json",
        profile_json.as_bytes(),
    )
    .unwrap();

    // package/ValueSet-TestValueSet.json
    let valueset_json = serde_json::json!({
        "resourceType": "ValueSet",
        "id": "TestValueSet",
        "url": "http://test.org/fhir/ValueSet/TestValueSet",
        "version": "1.0.0",
        "name": "TestValueSet",
        "status": "draft"
    })
    .to_string();

    let mut header = tar::Header::new_gnu();
    header.set_size(valueset_json.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(
        &mut header,
        "package/ValueSet-TestValueSet.json",
        valueset_json.as_bytes(),
    )
    .unwrap();

    let encoder = tar.into_inner().unwrap();
    encoder.finish().unwrap()
}

/// Helper to set up a mock FHIR package registry server
async fn mock_registry_with_package() -> MockServer {
    let server = MockServer::start().await;
    let tar_bytes = build_test_package();

    let metadata = serde_json::json!({
        "name": "test.fishable",
        "versions": {
            "1.0.0": {
                "name": "test.fishable",
                "version": "1.0.0",
                "dist": { "tarball": format!("{}/test.fishable-1.0.0.tgz", server.uri()) },
                "dependencies": {},
                "fhirVersions": ["4.0.1"]
            }
        },
        "dist-tags": { "latest": "1.0.0" }
    });

    Mock::given(method("GET"))
        .and(path("/test.fishable"))
        .respond_with(ResponseTemplate::new(200).set_body_json(metadata))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/test.fishable-1.0.0.tgz"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(tar_bytes))
        .mount(&server)
        .await;

    server
}

#[tokio::test]
async fn test_fish_by_url() {
    let server = mock_registry_with_package().await;
    let temp_dir = TempDir::new().unwrap();

    let mut config = octofhir_canonical_manager::FcmConfig::test_config(temp_dir.path());
    config.registry.url = format!("{}/", server.uri());

    let options = CanonicalOptions {
        config: Some(config),
        auto_install_core: false,
        quick_init: true,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    let session = facade.session([FhirRelease::R4]).await.unwrap();

    // Install test package
    session
        .ensure_packages(vec![maki_core::PackageCoordinate::new(
            "test.fishable",
            "1.0.0",
        )])
        .await
        .unwrap();

    // Fish by URL
    let result = session
        .fish_by_url("http://test.org/fhir/StructureDefinition/TestProfile")
        .await
        .unwrap();

    assert!(result.is_some(), "Should find resource by URL");
    let resource = result.unwrap();
    assert_eq!(resource.resource_type, "StructureDefinition");
    assert_eq!(
        resource.canonical_url,
        "http://test.org/fhir/StructureDefinition/TestProfile"
    );
}

#[tokio::test]
async fn test_fish_by_id() {
    let server = mock_registry_with_package().await;
    let temp_dir = TempDir::new().unwrap();

    let mut config = octofhir_canonical_manager::FcmConfig::test_config(temp_dir.path());
    config.registry.url = format!("{}/", server.uri());

    let options = CanonicalOptions {
        config: Some(config),
        auto_install_core: false,
        quick_init: true,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    let session = facade.session([FhirRelease::R4]).await.unwrap();

    session
        .ensure_packages(vec![maki_core::PackageCoordinate::new(
            "test.fishable",
            "1.0.0",
        )])
        .await
        .unwrap();

    // Fish by ID
    let result = session.fish_by_id("TestProfile").await.unwrap();

    assert!(result.is_some(), "Should find resource by ID");
    let resource = result.unwrap();
    assert_eq!(resource.resource_type, "StructureDefinition");
}

#[tokio::test]
async fn test_fish_with_type_filter() {
    let server = mock_registry_with_package().await;
    let temp_dir = TempDir::new().unwrap();

    let mut config = octofhir_canonical_manager::FcmConfig::test_config(temp_dir.path());
    config.registry.url = format!("{}/", server.uri());

    let options = CanonicalOptions {
        config: Some(config),
        auto_install_core: false,
        quick_init: true,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    let session = facade.session([FhirRelease::R4]).await.unwrap();

    session
        .ensure_packages(vec![maki_core::PackageCoordinate::new(
            "test.fishable",
            "1.0.0",
        )])
        .await
        .unwrap();

    // Fish with Profile filter - should find TestProfile
    let result = session
        .fish("TestProfile", &[FhirType::Profile])
        .await
        .unwrap();
    assert!(result.is_some(), "Should find Profile by name");

    // Fish with ValueSet filter - should NOT find TestProfile
    let result = session
        .fish("TestProfile", &[FhirType::ValueSet])
        .await
        .unwrap();
    assert!(
        result.is_none(),
        "Should not find Profile with ValueSet filter"
    );

    // Fish for ValueSet - should find TestValueSet
    let result = session
        .fish("TestValueSet", &[FhirType::ValueSet])
        .await
        .unwrap();
    assert!(result.is_some(), "Should find ValueSet");
}

#[tokio::test]
async fn test_fish_for_metadata() {
    let server = mock_registry_with_package().await;
    let temp_dir = TempDir::new().unwrap();

    let mut config = octofhir_canonical_manager::FcmConfig::test_config(temp_dir.path());
    config.registry.url = format!("{}/", server.uri());

    let options = CanonicalOptions {
        config: Some(config),
        auto_install_core: false,
        quick_init: true,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    let session = facade.session([FhirRelease::R4]).await.unwrap();

    session
        .ensure_packages(vec![maki_core::PackageCoordinate::new(
            "test.fishable",
            "1.0.0",
        )])
        .await
        .unwrap();

    // Fish for metadata
    let metadata = session.fish_for_metadata("TestProfile", &[]).await.unwrap();

    assert!(metadata.is_some(), "Should find metadata");
    let metadata = metadata.unwrap();
    assert_eq!(metadata.resource_type, "StructureDefinition");
    assert_eq!(metadata.id, Some("TestProfile".to_string()));
    assert_eq!(metadata.name, Some("TestProfile".to_string()));
    assert_eq!(metadata.kind, Some("resource".to_string()));
    assert_eq!(metadata.derivation, Some("constraint".to_string()));
}

#[tokio::test]
async fn test_fish_multi_strategy() {
    let server = mock_registry_with_package().await;
    let temp_dir = TempDir::new().unwrap();

    let mut config = octofhir_canonical_manager::FcmConfig::test_config(temp_dir.path());
    config.registry.url = format!("{}/", server.uri());

    let options = CanonicalOptions {
        config: Some(config),
        auto_install_core: false,
        quick_init: true,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    let session = facade.session([FhirRelease::R4]).await.unwrap();

    session
        .ensure_packages(vec![maki_core::PackageCoordinate::new(
            "test.fishable",
            "1.0.0",
        )])
        .await
        .unwrap();

    // Fish should work with URL
    let result = session
        .fish("http://test.org/fhir/StructureDefinition/TestProfile", &[])
        .await
        .unwrap();
    assert!(result.is_some(), "Should find by URL");

    // Fish should work with ID
    let result = session.fish("TestProfile", &[]).await.unwrap();
    assert!(result.is_some(), "Should find by ID");
}
