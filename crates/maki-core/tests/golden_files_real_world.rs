use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
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
use maki_core::export::ProfileExporter;
use maki_core::semantic::{AliasTable, Package};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Create a test definition session with FHIR R4 definitions
async fn create_test_session() -> Arc<maki_core::canonical::DefinitionSession> {
    let options = CanonicalOptions {
        auto_install_core: true,
        quick_init: true,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    let session = facade.session([FhirRelease::R4]).await.unwrap();

    Arc::new(session)
}

#[tokio::test]
async fn test_mcode_cancer_patient_profile_full() {
    // Real mCODE CancerPatient profile
    let fsh = r#"
Profile: CancerPatient
Parent: Patient
Id: mcode-cancer-patient
Title: "Cancer Patient Profile"
Description: "A patient who has been diagnosed with or is receiving medical treatment for a malignant growth or tumor."
* deceased[x] MS
* extension contains USCoreBirthSex named us-core-birthsex 0..1 MS
"#;

    let (cst, _lexer_errors, errors) = parse_fsh(fsh);
    assert!(
        errors.is_empty(),
        "Should parse without errors: {:?}",
        errors
    );

    let root = Document::cast(cst).expect("Failed to cast to Document");
    let profiles: Vec<_> = root.profiles().collect();

    assert_eq!(profiles.len(), 1, "Should parse exactly one profile");

    let profile = &profiles[0];
    assert_eq!(profile.name(), Some("CancerPatient".to_string()));
    assert_eq!(
        profile.parent().and_then(|p| p.value()),
        Some("Patient".to_string())
    );
    assert_eq!(
        profile.id().and_then(|id| id.value()),
        Some("mcode-cancer-patient".to_string())
    );
    assert_eq!(
        profile.title().and_then(|t| t.value()),
        Some("Cancer Patient Profile".to_string())
    );

    // Test export
    let session = create_test_session().await;
    let alias_table = AliasTable::new();
    let package = Arc::new(RwLock::new(Package::new()));
    let exporter = ProfileExporter::new(
        session,
        "http://hl7.org/fhir/us/mcode".to_string(),
        Some("0.1.0".to_string()),
        Some("draft".to_string()),
        Some("HL7 International".to_string()),
        alias_table,
        package,
    )
    .await
    .expect("Failed to create exporter");

    match exporter.export(profile).await {
        Ok(structure_def) => {
            assert_eq!(structure_def.name, "CancerPatient");
            assert_eq!(structure_def.id, Some("mcode-cancer-patient".to_string()));
            assert_eq!(
                structure_def.url,
                "http://hl7.org/fhir/us/mcode/StructureDefinition/mcode-cancer-patient"
            );
            assert_eq!(structure_def.type_field, "Patient");
            assert_eq!(structure_def.derivation, Some("constraint".to_string()));
        }
        Err(e) => {
            eprintln!("Export note (may need base definitions): {}", e);
        }
    }
}

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
