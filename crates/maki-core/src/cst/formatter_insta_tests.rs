//! Snapshot tests for formatter using insta

#[cfg(test)]
mod tests {
    use crate::cst::formatter::{FormatOptions, format_document};
    use insta::assert_snapshot;

    #[test]
    fn test_format_patient_profile() {
        let source = include_str!("../../../../examples/patient-profile.fsh");
        let formatted = format_document(source, &FormatOptions::default());

        assert_snapshot!(formatted);
    }

    #[test]
    fn test_format_simple_profile() {
        let source = r#"Profile:MyPatient
Parent:Patient
Id:my-patient
Title:"My Patient Profile"

*identifier 1..* MS
*name 1..1 MS
*gender 1..1"#;

        let formatted = format_document(source, &FormatOptions::default());
        assert_snapshot!(formatted);
    }

    #[test]
    fn test_format_with_caret_rules() {
        let source = r#"Profile:Test
Parent:Patient

*^version = "1.0.0"
*^status = #active
*name 1..1"#;

        let formatted = format_document(source, &FormatOptions::default());
        assert_snapshot!(formatted);
    }

    #[test]
    fn test_format_extension_contains() {
        let source = r#"Profile:Test
Parent:Patient

*extension contains
    race 0..1 MS and
    ethnicity 0..1 MS"#;

        let formatted = format_document(source, &FormatOptions::default());
        assert_snapshot!(formatted);
    }

    #[test]
    fn test_format_valueset_binding() {
        let source = r#"Profile:Test
Parent:Patient

*gender from GenderValueSet (required)
*communication.language from AllLanguages (extensible)"#;

        let formatted = format_document(source, &FormatOptions::default());
        assert_snapshot!(formatted);
    }

    #[test]
    fn test_format_idempotency_patient() {
        let source = include_str!("../../../../examples/patient-profile.fsh");
        let formatted1 = format_document(source, &FormatOptions::default());
        let formatted2 = format_document(&formatted1, &FormatOptions::default());

        assert_eq!(formatted1, formatted2, "Formatting should be idempotent");
    }

    #[test]
    fn test_format_no_alignment() {
        let source = r#"Profile:Test
Parent:Patient

*identifier 1..* MS
*name 1..1 MS
*gender 1..1"#;

        let options = FormatOptions {
            align_carets: false,
            ..Default::default()
        };

        let formatted = format_document(source, &options);
        assert_snapshot!(formatted);
    }
}
