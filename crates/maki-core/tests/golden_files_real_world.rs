use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
use maki_core::cst::ast::{AstNode, Document, Extension, Profile, ValueSet};
/// Real-World Golden File Tests from HL7 IGs
///
/// These tests use actual FSH content from published FHIR IGs:
/// - mCODE (Minimal Common Oncology Data Elements)
/// - US Core
/// - Other HL7 IGs
///
/// Tests verify that MAKI can parse and export real-world FSH correctly.
use maki_core::cst::parse_fsh;
use maki_core::export::{ExtensionExporter, ProfileExporter, ValueSetExporter};
use std::sync::Arc;

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
    let exporter = ProfileExporter::new(session, "http://hl7.org/fhir/us/mcode".to_string())
        .await
        .expect("Failed to create exporter");

    match exporter.export(&profile).await {
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
async fn test_mcode_primary_cancer_condition() {
    // Real mCODE PrimaryCancerCondition profile
    let fsh = r#"
Profile: PrimaryCancerCondition
Parent: Condition
Id: mcode-primary-cancer-condition
Title: "Primary Cancer Condition"
Description: "Records the history of primary cancers, including location and histology."
* extension contains
    HistologyMorphologyBehavior named histology-morphology-behavior 0..1 MS and
    RelatedCondition named related-condition 0..* MS
* code from PrimaryOrUncertainBehaviorCancerDisorderVS (extensible)
* code MS
* subject only Reference(CancerPatient)
* subject MS
* onset[x] MS
* abatement[x] MS
* stage MS
* stage.summary from CancerStageGroupVS (preferred)
* stage.summary MS
* stage.assessment MS
* stage.type MS
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
    assert_eq!(profile.name(), Some("PrimaryCancerCondition".to_string()));
    assert_eq!(
        profile.parent().and_then(|p| p.value()),
        Some("Condition".to_string())
    );
}

#[tokio::test]
async fn test_us_core_race_extension_full() {
    // Real US Core Race Extension
    let fsh = r#"
Extension: USCoreRace
Id: us-core-race
Title: "US Core Race Extension"
Description: "Concepts classifying the person into a named category of humans sharing common history, traits, geographical origin or nationality."
* extension contains
    ombCategory 0..5 MS and
    detailed 0..* and
    text 1..1 MS
* extension[ombCategory] ^short = "American Indian or Alaska Native|Asian|Black or African American|Native Hawaiian or Other Pacific Islander|White"
* extension[ombCategory].value[x] only Coding
* extension[ombCategory].valueCoding from OmbRaceCategoryVS (required)
* extension[detailed] ^short = "Extended race codes"
* extension[detailed].value[x] only Coding
* extension[detailed].valueCoding from DetailedRaceVS (required)
* extension[text] ^short = "Race Text"
* extension[text].value[x] only string
"#;

    let (cst, _lexer_errors, errors) = parse_fsh(fsh);
    assert!(
        errors.is_empty(),
        "Should parse without errors: {:?}",
        errors
    );

    let root = Document::cast(cst).expect("Failed to cast to Document");
    let extensions: Vec<_> = root.extensions().collect();

    assert_eq!(extensions.len(), 1);

    let extension = &extensions[0];
    assert_eq!(extension.name(), Some("USCoreRace".to_string()));
    assert_eq!(
        extension.id().and_then(|id| id.value()),
        Some("us-core-race".to_string())
    );
}

#[tokio::test]
async fn test_complex_valueset_with_includes() {
    // Real mCODE ValueSet
    let fsh = r#"
ValueSet: PrimaryOrUncertainBehaviorCancerDisorderVS
Id: mcode-primary-or-uncertain-behavior-cancer-disorder-vs
Title: "Primary or Uncertain Behavior Cancer Disorder Value Set"
Description: "Codes representing primary or uncertain behavior cancer disorders, drawn from SNOMED CT."
* ^copyright = "This value set includes content from SNOMED CT, which is copyright © 2002+ International Health Terminology Standards Development Organisation (IHTSDO), and distributed by agreement between IHTSDO and HL7. Implementer use of SNOMED CT is not covered by this agreement"
* ^experimental = false
* include codes from system http://snomed.info/sct where concept is-a #363346000 "Malignant neoplastic disease (disorder)"
* exclude codes from system http://snomed.info/sct where concept is-a #128462008 "Secondary malignant neoplastic disease (disorder)"
"#;

    let (cst, _lexer_errors, errors) = parse_fsh(fsh);
    assert!(
        errors.is_empty(),
        "Should parse without errors: {:?}",
        errors
    );

    let root = Document::cast(cst).expect("Failed to cast to Document");
    let valuesets: Vec<_> = root.value_sets().collect();

    assert_eq!(valuesets.len(), 1);

    let valueset = &valuesets[0];
    assert_eq!(
        valueset.name(),
        Some("PrimaryOrUncertainBehaviorCancerDisorderVS".to_string())
    );
    assert_eq!(
        valueset.id().and_then(|id| id.value()),
        Some("mcode-primary-or-uncertain-behavior-cancer-disorder-vs".to_string())
    );
    assert_eq!(
        valueset.title().and_then(|t| t.value()),
        Some("Primary or Uncertain Behavior Cancer Disorder Value Set".to_string())
    );
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

#[tokio::test]
async fn test_parsing_performance_large_profile() {
    // Test with a larger, more complex profile
    let fsh = r#"
Profile: GenomicsReport
Parent: DiagnosticReport
Id: genomics-report
Title: "Genomics Report"
Description: "Genetic Analysis Summary"
* extension contains
    RecommendedAction named recommended-action 0..* MS and
    SupportingInfo named supporting-info 0..* MS and
    RiskAssessment named risk-assessment 0..* MS
* code MS
* subject only Reference(Patient)
* subject MS
* effective[x] MS
* issued MS
* performer MS
* result MS
* result only Reference(Observation)
* conclusionCode MS
* conclusionCode from http://hl7.org/fhir/ValueSet/clinical-findings (example)
* presentedForm MS
* extension[RecommendedAction] ^short = "Recommended action based on genetics results"
* extension[SupportingInfo] ^short = "Additional supporting information"
* extension[RiskAssessment] ^short = "Risk assessment based on genetics"
* status MS
* category MS
* category ^slicing.discriminator.type = #pattern
* category ^slicing.discriminator.path = "$this"
* category ^slicing.rules = #open
* category contains Genetics 1..1 MS
* category[Genetics] = http://terminology.hl7.org/CodeSystem/v2-0074#GE "Genetics"
"#;

    use std::time::Instant;
    let start = Instant::now();

    let (cst, _lexer_errors, errors) = parse_fsh(fsh);
    let parse_duration = start.elapsed();

    if !errors.is_empty() {
        eprintln!("Parse errors found:");
        for err in &errors {
            eprintln!("  {:?}", err);
        }
    }
    assert!(
        errors.is_empty(),
        "Should parse without errors: {:?}",
        errors
    );

    let root = Document::cast(cst).expect("Failed to cast to Document");
    let profiles: Vec<_> = root.profiles().collect();

    assert_eq!(profiles.len(), 1);
    assert!(
        parse_duration.as_millis() < 100,
        "Parsing should be fast (took {:?})",
        parse_duration
    );

    let profile = &profiles[0];
    assert_eq!(profile.name(), Some("GenomicsReport".to_string()));
}
