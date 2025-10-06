//! Tests for typed AST layer

#[cfg(test)]
mod tests {
    use crate::cst::{
        ast::{AstNode, Document, Rule},
        parse_fsh,
    };

    #[test]
    fn test_profile_basic() {
        let source = r#"Profile: MyPatient
Parent: Patient
Id: my-patient
Title: "My Patient Profile"
Description: "A custom patient profile""#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");

        assert_eq!(profile.name().as_deref(), Some("MyPatient"));
        assert_eq!(
            profile.parent().unwrap().value().as_deref(),
            Some("Patient")
        );
        assert_eq!(profile.id().unwrap().value().as_deref(), Some("my-patient"));
        assert_eq!(
            profile.title().unwrap().value().as_deref(),
            Some("My Patient Profile")
        );
        assert_eq!(
            profile.description().unwrap().value().as_deref(),
            Some("A custom patient profile")
        );
    }

    #[test]
    fn test_profile_with_rules() {
        let source = r#"Profile: MyPatient
Parent: Patient

* identifier 1..* MS
* name 1..1 MS SU
* gender from GenderValueSet (required)"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");

        let rules: Vec<_> = profile.rules().collect();
        assert_eq!(rules.len(), 3, "Should have 3 rules");

        // First rule: cardinality
        match &rules[0] {
            Rule::Card(card) => {
                assert_eq!(card.path().unwrap().as_string(), "identifier");
                assert!(card.cardinality().unwrap().contains("1..*"));
                assert!(card.flags().contains(&"MS".to_string()));
            }
            _ => panic!("Expected CardRule"),
        }

        // Second rule: cardinality with multiple flags
        match &rules[1] {
            Rule::Card(card) => {
                assert_eq!(card.path().unwrap().as_string(), "name");
                assert!(card.cardinality().unwrap().contains("1..1"));
                let flags = card.flags();
                assert!(flags.contains(&"MS".to_string()));
                assert!(flags.contains(&"SU".to_string()));
            }
            _ => panic!("Expected CardRule"),
        }

        // Third rule: valueset binding
        match &rules[2] {
            Rule::ValueSet(vs) => {
                assert_eq!(vs.path().unwrap().as_string(), "gender");
                assert_eq!(vs.value_set().as_deref(), Some("GenderValueSet"));
                assert_eq!(vs.strength().as_deref(), Some("required"));
            }
            _ => panic!("Expected ValueSetRule"),
        }
    }

    #[test]
    fn test_extension_basic() {
        let source = r#"Extension: MyExtension
Id: my-extension
Title: "My Extension"
Description: "A custom extension""#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let extension = doc.extensions().next().expect("Should have an extension");

        assert_eq!(extension.name().as_deref(), Some("MyExtension"));
        assert_eq!(
            extension.id().unwrap().value().as_deref(),
            Some("my-extension")
        );
        assert_eq!(
            extension.title().unwrap().value().as_deref(),
            Some("My Extension")
        );
        assert_eq!(
            extension.description().unwrap().value().as_deref(),
            Some("A custom extension")
        );
    }

    #[test]
    fn test_valueset_basic() {
        let source = r#"ValueSet: MyValueSet
Id: my-valueset
Title: "My Value Set"
Description: "A custom value set""#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let valueset = doc.value_sets().next().expect("Should have a valueset");

        assert_eq!(valueset.name().as_deref(), Some("MyValueSet"));
        assert_eq!(
            valueset.id().unwrap().value().as_deref(),
            Some("my-valueset")
        );
        assert_eq!(
            valueset.title().unwrap().value().as_deref(),
            Some("My Value Set")
        );
        assert_eq!(
            valueset.description().unwrap().value().as_deref(),
            Some("A custom value set")
        );
    }

    #[test]
    fn test_codesystem_basic() {
        let source = r#"CodeSystem: MyCodeSystem
Id: my-codesystem
Title: "My Code System"
Description: "A custom code system""#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let codesystem = doc.code_systems().next().expect("Should have a codesystem");

        assert_eq!(codesystem.name().as_deref(), Some("MyCodeSystem"));
        assert_eq!(
            codesystem.id().unwrap().value().as_deref(),
            Some("my-codesystem")
        );
        assert_eq!(
            codesystem.title().unwrap().value().as_deref(),
            Some("My Code System")
        );
        assert_eq!(
            codesystem.description().unwrap().value().as_deref(),
            Some("A custom code system")
        );
    }

    #[test]
    fn test_alias_basic() {
        let source = r#"Alias: $SCT = http://snomed.info/sct
Alias: $LOINC = http://loinc.org"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let aliases: Vec<_> = doc.aliases().collect();

        assert_eq!(aliases.len(), 2);

        assert_eq!(aliases[0].name().as_deref(), Some("$SCT"));
        assert_eq!(
            aliases[0].value().as_deref(),
            Some("http://snomed.info/sct")
        );

        assert_eq!(aliases[1].name().as_deref(), Some("$LOINC"));
        assert_eq!(aliases[1].value().as_deref(), Some("http://loinc.org"));
    }

    #[test]
    fn test_multiple_profiles() {
        let source = r#"Profile: FirstProfile
Parent: Patient

Profile: SecondProfile
Parent: Observation"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profiles: Vec<_> = doc.profiles().collect();

        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name().as_deref(), Some("FirstProfile"));
        assert_eq!(
            profiles[0].parent().unwrap().value().as_deref(),
            Some("Patient")
        );

        assert_eq!(profiles[1].name().as_deref(), Some("SecondProfile"));
        assert_eq!(
            profiles[1].parent().unwrap().value().as_deref(),
            Some("Observation")
        );
    }

    #[test]
    fn test_path_segments() {
        let source = r#"Profile: MyProfile
Parent: Patient

* name.given 1..1"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");
        let rule = profile.rules().next().expect("Should have a rule");

        match rule {
            Rule::Card(card) => {
                let path = card.path().expect("Should have path");
                assert_eq!(path.as_string(), "name.given");
                let segments = path.segments();
                assert_eq!(segments, vec!["name", "given"]);
            }
            _ => panic!("Expected CardRule"),
        }
    }

    #[test]
    fn test_fixed_value_rule() {
        let source = r#"Profile: MyProfile
Parent: Patient

* active = true
* gender = "male""#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");
        let rules: Vec<_> = profile.rules().collect();

        assert_eq!(rules.len(), 2);

        // First fixed value: boolean
        match &rules[0] {
            Rule::FixedValue(fv) => {
                assert_eq!(fv.path().unwrap().as_string(), "active");
                assert_eq!(fv.value().as_deref(), Some("true"));
            }
            _ => panic!("Expected FixedValueRule"),
        }

        // Second fixed value: string
        match &rules[1] {
            Rule::FixedValue(fv) => {
                assert_eq!(fv.path().unwrap().as_string(), "gender");
                assert_eq!(fv.value().as_deref(), Some("male"));
            }
            _ => panic!("Expected FixedValueRule"),
        }
    }

    #[test]
    fn test_real_patient_profile() {
        // Use a real example from golden tests
        let source = include_str!("../../../../examples/patient-profile.fsh");
        let (cst, errors) = parse_fsh(source);

        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");

        // Verify we can extract metadata
        assert!(profile.name().is_some());
        assert!(profile.parent().is_some());
        assert!(profile.id().is_some());

        // Verify we can iterate rules
        let rule_count = profile.rules().count();
        assert!(rule_count > 0, "Should have at least one rule");
    }

    #[test]
    fn test_profile_without_optional_metadata() {
        let source = r#"Profile: MinimalProfile
Parent: Patient"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");

        assert_eq!(profile.name().as_deref(), Some("MinimalProfile"));
        assert_eq!(
            profile.parent().unwrap().value().as_deref(),
            Some("Patient")
        );
        assert!(profile.id().is_none(), "Id should be None");
        assert!(profile.title().is_none(), "Title should be None");
        assert!(
            profile.description().is_none(),
            "Description should be None"
        );
    }

    #[test]
    fn test_ast_preserves_cst_lossless() {
        let source = r#"Profile:  MyPatient   // Extra spaces!
Parent: Patient
Id: my-patient

* name 1..1 MS"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        // Create typed AST
        let doc = Document::cast(cst.clone()).expect("Should be a document");
        let _profile = doc.profiles().next().expect("Should have a profile");

        // Verify CST is still lossless
        assert_eq!(
            cst.text().to_string(),
            source,
            "CST should still be lossless"
        );
    }
}
