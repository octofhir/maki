//! Snapshot tests for the formatter
//!
//! These tests verify that the formatter produces consistent, expected output
//! for various FSH constructs. They use golden files to ensure formatting
//! doesn't change unexpectedly.

#[cfg(test)]
mod tests {
    use crate::cst::formatter::{FormatOptions, format_document};

    /// Helper to test formatting with snapshots
    fn test_format_snapshot(input: &str, expected: &str, options: &FormatOptions) {
        let formatted = format_document(input, options);
        assert_eq!(
            formatted, expected,
            "\n=== Input ===\n{input}\n\n=== Expected ===\n{expected}\n\n=== Got ===\n{formatted}\n"
        );
    }

    #[test]
    fn snapshot_basic_profile() {
        let input = "Profile:MyPatient\nParent:Patient\nId:my-patient";

        let expected = r#"Profile: MyPatient
Parent: Patient
Id: my-patient
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    fn snapshot_profile_with_metadata() {
        let input = r#"Profile:USCorePatient
Parent:Patient
Id:us-core-patient
Title:"US Core Patient Profile"
Description:"Profile for US Core Patient based on FHIR Patient resource""#;

        let expected = r#"Profile: USCorePatient
Parent: Patient
Id: us-core-patient
Title: "US Core Patient Profile"
Description: "Profile for US Core Patient based on FHIR Patient resource"
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    #[ignore] // TODO: Formatter doesn't output rule paths properly
    fn snapshot_profile_with_rules_aligned() {
        let input = r#"Profile:MyProfile
Parent:Patient

*identifier 1..* MS
*name 1..1 MS
*gender 1..1
*birthDate 0..1"#;

        let expected = r#"Profile: MyProfile
Parent: Patient

* identifier 1..* MS
* name       1..1 MS
* gender     1..1
* birthDate  0..1
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    #[ignore] // TODO: Formatter doesn't output rule paths properly
    fn snapshot_profile_with_rules_not_aligned() {
        let input = r#"Profile:MyProfile
Parent:Patient

*identifier 1..* MS
*name 1..1 MS"#;

        let options = FormatOptions {
            align_carets: false,
            ..Default::default()
        };

        let expected = r#"Profile: MyProfile
Parent: Patient

* identifier 1..* MS
* name 1..1 MS
"#;

        test_format_snapshot(input, expected, &options);
    }

    #[test]
    fn snapshot_extension() {
        let input = r#"Extension:PatientRace
Id:patient-race
Title:"Patient Race Extension"
Description:"Extension for capturing patient race information""#;

        let expected = r#"Extension: PatientRace
Id: patient-race
Title: "Patient Race Extension"
Description: "Extension for capturing patient race information"
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    fn snapshot_valueset() {
        let input = r#"ValueSet:AdministrativeGender
Id:administrative-gender
Title:"Administrative Gender"
Description:"Codes representing administrative gender""#;

        let expected = r#"ValueSet: AdministrativeGender
Id: administrative-gender
Title: "Administrative Gender"
Description: "Codes representing administrative gender"
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    fn snapshot_codesystem() {
        let input = r#"CodeSystem:MyCodeSystem
Id:my-codesystem
Title:"My Code System"
Description:"Custom code system for demonstration""#;

        let expected = r#"CodeSystem: MyCodeSystem
Id: my-codesystem
Title: "My Code System"
Description: "Custom code system for demonstration"
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    fn snapshot_alias() {
        let input = "Alias:SCT=http://snomed.info/sct\nAlias:LOINC=http://loinc.org";

        let expected = "Alias: SCT = http://snomed.info/sct\n\nAlias: LOINC = http://loinc.org\n\n";

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    fn snapshot_multiple_profiles() {
        let input = r#"Profile:FirstProfile
Parent:Patient

Profile:SecondProfile
Parent:Observation"#;

        let expected = r#"Profile: FirstProfile
Parent: Patient

Profile: SecondProfile
Parent: Observation
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    #[ignore] // TODO: Formatter doesn't output rule paths properly
    fn snapshot_mixed_rules() {
        let input = r#"Profile:ComplexProfile
Parent:Patient

*identifier 1..* MS
*name.given 1..1
*gender from GenderValueSet (required)
*active = true"#;

        let expected = r#"Profile: ComplexProfile
Parent: Patient

* identifier 1..* MS
* name.given 1..1
* gender from GenderValueSet (required)
* active = true
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    #[ignore] // TODO: Formatter doesn't output rule paths properly
    fn snapshot_path_segments() {
        let input = r#"Profile:Test
Parent:Patient

*name.given 1..*
*name.family 1..1
*identifier.system 1..1"#;

        let expected = r#"Profile: Test
Parent: Patient

* name.given       1..*
* name.family      1..1
* identifier.system 1..1
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    #[ignore] // TODO: Formatter doesn't output rule paths properly
    fn snapshot_fixed_values() {
        let input = r#"Profile:Test
Parent:Patient

*active = true
*gender = "male"
*multipleBirthInteger = 1"#;

        let expected = r#"Profile: Test
Parent: Patient

* active = true
* gender = "male"
* multipleBirthInteger = 1
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    #[ignore] // TODO: Formatter doesn't output rule paths properly
    fn snapshot_flags() {
        let input = r#"Profile:Test
Parent:Patient

*identifier MS SU
*name MS
*gender SU"#;

        let expected = r#"Profile: Test
Parent: Patient

* identifier MS SU
* name       MS
* gender     SU
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }

    #[test]
    fn snapshot_idempotency() {
        let input = r#"Profile: Test
Parent: Patient

* name MS
* gender"#;

        let formatted1 = format_document(input, &FormatOptions::default());
        let formatted2 = format_document(&formatted1, &FormatOptions::default());
        let formatted3 = format_document(&formatted2, &FormatOptions::default());

        assert_eq!(
            formatted1, formatted2,
            "Formatted once should equal formatted twice"
        );
        assert_eq!(
            formatted2, formatted3,
            "Formatted twice should equal formatted thrice"
        );
    }

    #[test]
    fn snapshot_real_patient_profile() {
        // Test with a real FSH file
        let input = include_str!("../../../../examples/patient-profile.fsh");
        let formatted = format_document(input, &FormatOptions::default());

        // Verify idempotency with real file
        let formatted_twice = format_document(&formatted, &FormatOptions::default());
        assert_eq!(
            formatted, formatted_twice,
            "Real file formatting should be idempotent"
        );
    }

    #[test]
    #[ignore] // TODO: Formatter doesn't output rule paths properly
    fn snapshot_comprehensive_document() {
        let input = r#"Alias:$SCT=http://snomed.info/sct

Profile:USCorePatient
Parent:Patient
Id:us-core-patient
Title:"US Core Patient"
Description:"US Core Patient Profile"

*identifier 1..* MS
*name 1..* MS
*gender 1..1
*birthDate 0..1

Extension:Race
Id:patient-race
Title:"Patient Race"

ValueSet:RaceCodes
Id:race-codes
Title:"Race Value Set""#;

        let expected = r#"Alias: $SCT = http://snomed.info/sct

Profile: USCorePatient
Parent: Patient
Id: us-core-patient
Title: "US Core Patient"
Description: "US Core Patient Profile"

* identifier 1..* MS
* name       1..* MS
* gender     1..1
* birthDate  0..1

Extension: Race
Id: patient-race
Title: "Patient Race"

ValueSet: RaceCodes
Id: race-codes
Title: "Race Value Set"
"#;

        test_format_snapshot(input, expected, &FormatOptions::default());
    }
}
