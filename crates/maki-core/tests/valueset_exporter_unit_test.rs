//! Unit tests for ValueSet exporter functionality
//!
//! These tests focus specifically on the ValueSet exporter implementation
//! and don't depend on other modules that may have compilation issues.

use maki_core::export::{
    ValueSetCompose, ValueSetConcept, ValueSetConceptDesignation, ValueSetConceptProperty,
    ValueSetConceptPropertyValue, ValueSetFilter, ValueSetInclude, ValueSetResource,
};

#[test]
fn test_valueset_resource_creation() {
    let vs = ValueSetResource::new(
        "http://example.org/fhir/ValueSet/test-vs",
        "TestVS",
        "draft",
    );

    assert_eq!(vs.resource_type, "ValueSet");
    assert_eq!(vs.url, "http://example.org/fhir/ValueSet/test-vs");
    assert_eq!(vs.name, "TestVS");
    assert_eq!(vs.status, "draft");
    assert!(vs.id.is_none());
    assert!(vs.compose.is_none());
}

#[test]
fn test_valueset_compose_operations() {
    let mut compose = ValueSetCompose::new();
    let include = ValueSetInclude::from_system("http://loinc.org");

    compose.add_include(include);

    assert!(compose.include.is_some());
    assert_eq!(compose.include.as_ref().unwrap().len(), 1);
    assert_eq!(
        compose.include.as_ref().unwrap()[0]
            .system
            .as_ref()
            .unwrap(),
        "http://loinc.org"
    );
}

#[test]
fn test_valueset_include_with_concept() {
    let mut include = ValueSetInclude::from_system("http://loinc.org");
    include.add_concept(ValueSetConcept::with_display("12345-6", "Blood pressure"));

    assert!(include.concept.is_some());
    assert_eq!(include.concept.as_ref().unwrap().len(), 1);
    assert_eq!(include.concept.as_ref().unwrap()[0].code, "12345-6");
    assert_eq!(
        include.concept.as_ref().unwrap()[0]
            .display
            .as_ref()
            .unwrap(),
        "Blood pressure"
    );
}

#[test]
fn test_valueset_concept_with_properties() {
    let mut concept = ValueSetConcept::new("12345-6");

    // Add a code property
    concept.add_property(ValueSetConceptProperty::code("status", "active"));

    // Add a boolean property
    concept.add_property(ValueSetConceptProperty::boolean("inactive", false));

    // Add a string property
    concept.add_property(ValueSetConceptProperty::string(
        "description",
        "Test concept",
    ));

    assert!(concept.property.is_some());
    let properties = concept.property.as_ref().unwrap();
    assert_eq!(properties.len(), 3);

    // Check code property
    assert_eq!(properties[0].code, "status");
    match &properties[0].value {
        ValueSetConceptPropertyValue::Code(val) => assert_eq!(val, "active"),
        _ => panic!("Expected code property"),
    }

    // Check boolean property
    assert_eq!(properties[1].code, "inactive");
    match &properties[1].value {
        ValueSetConceptPropertyValue::Boolean(val) => assert_eq!(*val, false),
        _ => panic!("Expected boolean property"),
    }

    // Check string property
    assert_eq!(properties[2].code, "description");
    match &properties[2].value {
        ValueSetConceptPropertyValue::String(val) => assert_eq!(val, "Test concept"),
        _ => panic!("Expected string property"),
    }
}

#[test]
fn test_valueset_concept_with_designations() {
    let mut concept = ValueSetConcept::with_display("12345-6", "Blood pressure");

    // Add English designation
    concept.add_designation(ValueSetConceptDesignation::with_language(
        "Blood pressure measurement",
        "en",
    ));

    // Add Spanish designation
    concept.add_designation(ValueSetConceptDesignation::with_language(
        "Medición de presión arterial",
        "es",
    ));

    assert!(concept.designation.is_some());
    let designations = concept.designation.as_ref().unwrap();
    assert_eq!(designations.len(), 2);

    // Check English designation
    assert_eq!(designations[0].value, "Blood pressure measurement");
    assert_eq!(designations[0].language.as_ref().unwrap(), "en");

    // Check Spanish designation
    assert_eq!(designations[1].value, "Medición de presión arterial");
    assert_eq!(designations[1].language.as_ref().unwrap(), "es");
}

#[test]
fn test_valueset_filters() {
    // Test is-a filter
    let is_a_filter = ValueSetFilter::is_a("123456");
    assert_eq!(is_a_filter.property, "concept");
    assert_eq!(is_a_filter.op, "is-a");
    assert_eq!(is_a_filter.value, "123456");

    // Test regex filter
    let regex_filter = ValueSetFilter::regex("^(A|B).*");
    assert_eq!(regex_filter.property, "concept");
    assert_eq!(regex_filter.op, "regex");
    assert_eq!(regex_filter.value, "^(A|B).*");

    // Test descendent-of filter
    let descendent_filter = ValueSetFilter::descendent_of("12345");
    assert_eq!(descendent_filter.property, "concept");
    assert_eq!(descendent_filter.op, "descendent-of");
    assert_eq!(descendent_filter.value, "12345");

    // Test exists filter
    let exists_filter = ValueSetFilter::exists("inactive");
    assert_eq!(exists_filter.property, "inactive");
    assert_eq!(exists_filter.op, "exists");
    assert_eq!(exists_filter.value, "true");
}

#[test]
fn test_valueset_include_with_filters() {
    let mut include = ValueSetInclude::from_system("http://snomed.info/sct");
    include.add_filter(ValueSetFilter::is_a("123456"));
    include.add_filter(ValueSetFilter::regex("^A.*"));

    assert!(include.filter.is_some());
    let filters = include.filter.as_ref().unwrap();
    assert_eq!(filters.len(), 2);

    assert_eq!(filters[0].op, "is-a");
    assert_eq!(filters[0].value, "123456");

    assert_eq!(filters[1].op, "regex");
    assert_eq!(filters[1].value, "^A.*");
}

#[test]
fn test_valueset_compose_with_excludes() {
    let mut compose = ValueSetCompose::new();

    // Add include
    let include = ValueSetInclude::from_system("http://loinc.org");
    compose.add_include(include);

    // Add exclude
    let exclude = ValueSetInclude::from_system("http://loinc.org");
    compose.add_exclude(exclude);

    assert!(compose.include.is_some());
    assert!(compose.exclude.is_some());
    assert_eq!(compose.include.as_ref().unwrap().len(), 1);
    assert_eq!(compose.exclude.as_ref().unwrap().len(), 1);
}

#[test]
fn test_valueset_include_from_valueset() {
    let include = ValueSetInclude::from_valueset("http://example.org/vs/vital-signs");

    assert!(include.value_set.is_some());
    assert_eq!(
        include.value_set.as_ref().unwrap()[0],
        "http://example.org/vs/vital-signs"
    );
    assert!(include.system.is_none());
}

#[test]
fn test_valueset_include_with_version() {
    let mut include = ValueSetInclude::from_system("http://loinc.org");
    include.version = Some("2.72".to_string());

    assert_eq!(include.version.as_ref().unwrap(), "2.72");
    assert_eq!(include.system.as_ref().unwrap(), "http://loinc.org");
}

#[test]
fn test_complex_valueset_scenario() {
    let mut compose = ValueSetCompose::new();

    // Create include with concepts, filters, and version
    let mut include = ValueSetInclude::from_system("http://snomed.info/sct");
    include.version = Some("20230901".to_string());

    // Add concept with properties and designations
    let mut concept = ValueSetConcept::with_display("123456789", "Test condition");
    concept.add_property(ValueSetConceptProperty::code("status", "active"));
    concept.add_designation(ValueSetConceptDesignation::with_language(
        "Test condition (English)",
        "en",
    ));
    include.add_concept(concept);

    // Add filter
    include.add_filter(ValueSetFilter::is_a("404684003"));

    compose.add_include(include);

    // Create exclude
    let mut exclude = ValueSetInclude::from_system("http://snomed.info/sct");
    exclude.add_concept(ValueSetConcept::new("987654321"));
    compose.add_exclude(exclude);

    // Verify the complex structure
    assert!(compose.include.is_some());
    assert!(compose.exclude.is_some());

    let includes = compose.include.as_ref().unwrap();
    assert_eq!(includes.len(), 1);
    assert_eq!(includes[0].version.as_ref().unwrap(), "20230901");
    assert!(includes[0].concept.is_some());
    assert!(includes[0].filter.is_some());

    let excludes = compose.exclude.as_ref().unwrap();
    assert_eq!(excludes.len(), 1);
    assert!(excludes[0].concept.is_some());
}
