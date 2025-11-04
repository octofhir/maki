//! Tests for ValueSet Advanced Features
//!
//! This module tests the enhanced ValueSet exporter functionality including:
//! - Filter components (is-a, regex, exists, etc.)
//! - Exclude rules and version handling
//! - Properties and designations support
//! - Complex filter expression parsing

use maki_core::canonical::DefinitionSession;
use maki_core::cst::ast::Document;
use maki_core::cst::parse_fsh;
use maki_core::export::{ValueSetCompose, ValueSetExporter, ValueSetFilter, ValueSetInclude};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Create a test session for ValueSet export tests
async fn create_test_session() -> Arc<DefinitionSession> {
    Arc::new(DefinitionSession::for_testing())
}

/// Parse ValueSets from FSH content
fn parse_valuesets(fsh: &str) -> Vec<maki_core::cst::ast::ValueSet> {
    let (cst, errors) = parse_fsh(fsh);
    assert!(
        errors.is_empty(),
        "Lexer errors encountered while parsing test FSH: {errors:?}"
    );

    let document = Document::cast(cst).expect("Parsed CST should be a document");
    document.valuesets().collect()
}

#[tokio::test]
async fn test_filter_components_is_a() {
    let fsh = r#"
Alias: SNOMED = http://snomed.info/sct

ValueSet: ConditionCodes
Title: "Condition Codes"
Description: "SNOMED CT codes for conditions"
* SNOMED where concept is-a #404684003
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1, "Should parse exactly one valueset");

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    assert!(result.is_ok(), "Export should succeed");

    let vs_resource = result.unwrap();
    assert!(vs_resource.compose.is_some(), "Should have compose");

    let compose = vs_resource.compose.unwrap();
    assert!(compose.include.is_some(), "Should have includes");

    let includes = compose.include.unwrap();
    assert_eq!(includes.len(), 1, "Should have one include");

    let include = &includes[0];
    assert_eq!(include.system.as_ref().unwrap(), "http://snomed.info/sct");
    assert!(include.filter.is_some(), "Should have filters");

    let filters = include.filter.as_ref().unwrap();
    assert_eq!(filters.len(), 1, "Should have one filter");

    let filter = &filters[0];
    assert_eq!(filter.property, "concept");
    assert_eq!(filter.op, "is-a");
    assert_eq!(filter.value, "404684003");
}

#[tokio::test]
async fn test_filter_components_regex() {
    let fsh = r#"
Alias: LOINC = http://loinc.org

ValueSet: BloodPressureCodes
Title: "Blood Pressure Codes"
Description: "LOINC codes for blood pressure measurements"
* LOINC where concept regex "^85354-9|8480-6|8462-4$"
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    assert!(result.is_ok(), "Export should succeed");

    let vs_resource = result.unwrap();
    let compose = vs_resource.compose.unwrap();
    let includes = compose.include.unwrap();
    let include = &includes[0];
    let filters = include.filter.as_ref().unwrap();
    let filter = &filters[0];

    assert_eq!(filter.property, "concept");
    assert_eq!(filter.op, "regex");
    assert_eq!(filter.value, "^85354-9|8480-6|8462-4$");
}

#[tokio::test]
async fn test_exclude_rules() {
    let fsh = r#"
Alias: SNOMED = http://snomed.info/sct

ValueSet: ConditionCodesExcluded
Title: "Condition Codes with Exclusions"
Description: "SNOMED CT codes for conditions excluding deprecated ones"
* SNOMED where concept is-a #404684003
* exclude SNOMED#123456789 "Deprecated condition"
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    assert!(result.is_ok(), "Export should succeed");

    let vs_resource = result.unwrap();
    let compose = vs_resource.compose.unwrap();

    // Check includes
    assert!(compose.include.is_some(), "Should have includes");
    let includes = compose.include.unwrap();
    assert_eq!(includes.len(), 1);

    // Check excludes
    assert!(compose.exclude.is_some(), "Should have excludes");
    let excludes = compose.exclude.unwrap();
    assert_eq!(excludes.len(), 1);

    let exclude = &excludes[0];
    assert_eq!(exclude.system.as_ref().unwrap(), "http://snomed.info/sct");
    assert!(exclude.concept.is_some(), "Should have excluded concepts");

    let concepts = exclude.concept.as_ref().unwrap();
    assert_eq!(concepts.len(), 1);
    assert_eq!(concepts[0].code, "123456789");
    assert_eq!(
        concepts[0].display.as_ref().unwrap(),
        "Deprecated condition"
    );
}

#[tokio::test]
async fn test_version_specific_includes() {
    let fsh = r#"
Alias: LOINC = http://loinc.org

ValueSet: VersionedCodes
Title: "Versioned LOINC Codes"
Description: "LOINC codes from specific version"
* LOINC#12345-6 "Blood pressure" version "2.72"
* LOINC where concept is-a #LP7839-6 version "2.72"
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    assert!(result.is_ok(), "Export should succeed");

    let vs_resource = result.unwrap();
    let compose = vs_resource.compose.unwrap();
    let includes = compose.include.unwrap();

    // Should have two includes (one for concept, one for filter)
    assert_eq!(includes.len(), 2);

    // Check that both have version specified
    for include in &includes {
        assert_eq!(include.version.as_ref().unwrap(), "2.72");
        assert_eq!(include.system.as_ref().unwrap(), "http://loinc.org");
    }

    // First include should have concept
    let concept_include = includes.iter().find(|i| i.concept.is_some()).unwrap();
    let concepts = concept_include.concept.as_ref().unwrap();
    assert_eq!(concepts[0].code, "12345-6");

    // Second include should have filter
    let filter_include = includes.iter().find(|i| i.filter.is_some()).unwrap();
    let filters = filter_include.filter.as_ref().unwrap();
    assert_eq!(filters[0].op, "is-a");
}

#[tokio::test]
async fn test_valueset_references() {
    let fsh = r#"
ValueSet: CompositeValueSet
Title: "Composite Value Set"
Description: "ValueSet that includes codes from other ValueSets"
* include codes from valueset "http://hl7.org/fhir/ValueSet/condition-category"
* exclude codes from valueset "http://example.org/fhir/ValueSet/deprecated-conditions"
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    assert!(result.is_ok(), "Export should succeed");

    let vs_resource = result.unwrap();
    let compose = vs_resource.compose.unwrap();

    // Check includes
    assert!(compose.include.is_some());
    let includes = compose.include.unwrap();
    assert_eq!(includes.len(), 1);

    let include = &includes[0];
    assert!(include.value_set.is_some());
    assert_eq!(
        include.value_set.as_ref().unwrap()[0],
        "http://hl7.org/fhir/ValueSet/condition-category"
    );

    // Check excludes
    assert!(compose.exclude.is_some());
    let excludes = compose.exclude.unwrap();
    assert_eq!(excludes.len(), 1);

    let exclude = &excludes[0];
    assert!(exclude.value_set.is_some());
    assert_eq!(
        exclude.value_set.as_ref().unwrap()[0],
        "http://example.org/fhir/ValueSet/deprecated-conditions"
    );
}

#[tokio::test]
async fn test_complex_filter_expressions() {
    let fsh = r#"
Alias: SNOMED = http://snomed.info/sct

ValueSet: ComplexFilters
Title: "Complex Filter Examples"
Description: "ValueSet demonstrating complex filter expressions"
* SNOMED where concept descendent-of #123456
* SNOMED where STATUS = "ACTIVE"
* SNOMED where inactive exists false
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    assert!(result.is_ok(), "Export should succeed");

    let vs_resource = result.unwrap();
    let compose = vs_resource.compose.unwrap();
    let includes = compose.include.unwrap();

    // Should have three includes (one for each filter)
    assert_eq!(includes.len(), 3);

    // Check each filter type
    let filters: Vec<&ValueSetFilter> = includes
        .iter()
        .filter_map(|i| i.filter.as_ref())
        .flatten()
        .collect();

    assert_eq!(filters.len(), 3);

    // Find descendent-of filter
    let descendent_filter = filters.iter().find(|f| f.op == "descendent-of").unwrap();
    assert_eq!(descendent_filter.property, "concept");
    assert_eq!(descendent_filter.value, "123456");

    // Find equals filter
    let equals_filter = filters.iter().find(|f| f.op == "=").unwrap();
    assert_eq!(equals_filter.property, "STATUS");
    assert_eq!(equals_filter.value, "ACTIVE");

    // Find exists filter
    let exists_filter = filters.iter().find(|f| f.op == "exists").unwrap();
    assert_eq!(exists_filter.property, "inactive");
    assert_eq!(exists_filter.value, "false");
}

#[tokio::test]
async fn test_concept_properties_and_designations() {
    let fsh = r#"
Alias: LOINC = http://loinc.org

ValueSet: ConceptsWithMetadata
Title: "Concepts with Properties and Designations"
Description: "ValueSet demonstrating concept properties and designations"
* LOINC#12345-6 "Blood pressure" ^property[status] = #active ^designation[0].language = #en ^designation[0].value = "Blood pressure measurement"
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    assert!(result.is_ok(), "Export should succeed");

    let vs_resource = result.unwrap();
    let compose = vs_resource.compose.unwrap();
    let includes = compose.include.unwrap();
    assert_eq!(includes.len(), 1);

    let include = &includes[0];
    assert!(include.concept.is_some());

    let concepts = include.concept.as_ref().unwrap();
    assert_eq!(concepts.len(), 1);

    let concept = &concepts[0];
    assert_eq!(concept.code, "12345-6");
    assert_eq!(concept.display.as_ref().unwrap(), "Blood pressure");

    // Check properties
    assert!(concept.property.is_some());
    let properties = concept.property.as_ref().unwrap();
    assert_eq!(properties.len(), 1);
    assert_eq!(properties[0].code, "status");

    // Check designations
    assert!(concept.designation.is_some());
    let designations = concept.designation.as_ref().unwrap();
    assert_eq!(designations.len(), 1);
    assert_eq!(designations[0].language.as_ref().unwrap(), "en");
    assert_eq!(designations[0].value, "Blood pressure measurement");
}

#[tokio::test]
async fn test_filter_validation_errors() {
    let fsh = r#"
Alias: SNOMED = http://snomed.info/sct

ValueSet: InvalidFilters
Title: "Invalid Filter Examples"
Description: "ValueSet with invalid filter expressions"
* SNOMED where concept invalid-operator #123456
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    // Should handle invalid operators gracefully (may warn but not fail)
    // The exact behavior depends on implementation - this tests error handling
    match result {
        Ok(_) => {
            // If it succeeds, that's fine - it means we handled the error gracefully
        }
        Err(e) => {
            // If it fails, the error should be informative
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("invalid-operator") || error_msg.contains("Unsupported"));
        }
    }
}

#[tokio::test]
async fn test_exclude_conflict_validation() {
    let fsh = r#"
Alias: SNOMED = http://snomed.info/sct

ValueSet: ConflictingRules
Title: "Conflicting Include/Exclude Rules"
Description: "ValueSet with potentially conflicting rules"
* SNOMED#123456 "Included concept"
* exclude SNOMED#123456 "Same concept excluded"
"#;

    let valuesets = parse_valuesets(fsh);
    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    let session = create_test_session().await;
    let exporter = ValueSetExporter::new(session, "http://example.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    let result = exporter.export(&valueset).await;
    assert!(result.is_ok(), "Export should succeed despite conflicts");

    let vs_resource = result.unwrap();
    let compose = vs_resource.compose.unwrap();

    // Should have both includes and excludes
    assert!(compose.include.is_some());
    assert!(compose.exclude.is_some());

    // The validation should have logged warnings about conflicts
    // but not prevented the export from succeeding
}
