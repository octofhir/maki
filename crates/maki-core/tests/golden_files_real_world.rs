use maki_core::cst::ast::{AstNode, Document};
/// Real-World Golden File Tests from HL7 IGs
///
/// These tests use actual FSH content from published FHIR IGs:
/// - mCODE (Minimal Common Oncology Data Elements)
/// - US Core
/// - Other HL7 IGs
///
/// Tests verify that MAKI can parse and export real-world FSH correctly.
use maki_core::cst::parse_fsh;

#[tokio::test]
async fn test_profile_with_multiple_slices() {
    // Profile with complex slicing
    let fsh = r#"
Profile: TumorMarkerTest
Parent: Observation
Id: mcode-tumor-marker-test
Title: "Tumor Marker Test"
Description: "The result of a tumor marker test."
* code from TumorMarkerTestVS (extensible)
* value[x] only Quantity or Ratio or string or CodeableConcept
* value[x] MS
* interpretation MS
* method MS
* bodySite MS
* specimen MS
* hasMember only Reference(TumorMarkerTest)
* component MS
* component ^slicing.discriminator.type = #pattern
* component ^slicing.discriminator.path = "code"
* component ^slicing.rules = #open
* component ^slicing.description = "Slice based on the component.code pattern"
* component contains tumorMarkerTest 0..* MS
* component[tumorMarkerTest].code from TumorMarkerTestVS (extensible)
* component[tumorMarkerTest].value[x] only Quantity or Ratio or string or CodeableConcept
"#;

    let (cst, _lexer_errors, errors) = parse_fsh(fsh);
    assert!(
        errors.is_empty(),
        "Should parse without errors: {:?}",
        errors
    );

    let root = Document::cast(cst).expect("Failed to cast to Document");
    let profiles: Vec<_> = root.profiles().collect();

    assert_eq!(profiles.len(), 1);

    let profile = &profiles[0];
    assert_eq!(profile.name(), Some("TumorMarkerTest".to_string()));
}

#[tokio::test]
async fn test_profile_with_cardinality_and_flags() {
    // Test MS (Must Support) and other flags
    let fsh = r#"
Profile: CancerDiseaseStatus
Parent: Observation
Id: mcode-cancer-disease-status
Title: "Cancer Disease Status"
Description: "A clinician's qualitative judgment on the current trend of the cancer."
* status MS
* code = LNC#88040-1 "Response to cancer treatment"
* code MS
* subject 1..1 MS
* subject only Reference(CancerPatient)
* effective[x] only dateTime or Period
* effective[x] 1..1 MS
* value[x] only CodeableConcept
* value[x] 1..1 MS
* valueCodeableConcept from ConditionStatusTrendVS (required)
* interpretation 0..0
* bodySite 0..0
* specimen 0..0
* device 0..0
* referenceRange 0..0
* hasMember 0..0
* component 0..0
* method MS
* evidence only Reference(CancerCondition)
* evidence MS
"#;

    let (cst, _lexer_errors, errors) = parse_fsh(fsh);
    assert!(
        errors.is_empty(),
        "Should parse without errors: {:?}",
        errors
    );

    let root = Document::cast(cst).expect("Failed to cast to Document");
    let profiles: Vec<_> = root.profiles().collect();

    assert_eq!(profiles.len(), 1);

    let profile = &profiles[0];
    assert_eq!(profile.name(), Some("CancerDiseaseStatus".to_string()));
    assert_eq!(
        profile.id().and_then(|id| id.value()),
        Some("mcode-cancer-disease-status".to_string())
    );
}

#[tokio::test]
async fn test_instance_with_complex_assignments() {
    // Test instance with nested assignments
    let fsh = r#"
Instance: cancer-disease-status-improved
InstanceOf: CancerDiseaseStatus
Usage: #example
Description: "Example showing disease improved"
* status = #final
* code = LNC#88040-1 "Response to cancer treatment"
* subject = Reference(cancer-patient-john-anyperson)
* effectiveDateTime = "2019-04-01"
* valueCodeableConcept = SCT#268910001 "Patient condition improved (finding)"
* method = SCT##4284000 "Radiography finding (observable entity)"
* performer = Reference(us-core-practitioner-owen-oncologist)
"#;

    let (cst, _lexer_errors, errors) = parse_fsh(fsh);
    assert!(
        errors.is_empty(),
        "Should parse without errors: {:?}",
        errors
    );

    let root = Document::cast(cst).expect("Failed to cast to Document");
    let instances: Vec<_> = root.instances().collect();

    assert_eq!(instances.len(), 1);

    let instance = &instances[0];
    assert_eq!(
        instance.name(),
        Some("cancer-disease-status-improved".to_string())
    );
    assert_eq!(
        instance.instance_of().and_then(|i| i.value()),
        Some("CancerDiseaseStatus".to_string())
    );
}
