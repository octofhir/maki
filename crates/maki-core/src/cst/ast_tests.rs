//! Tests for typed AST layer

#[cfg(test)]
mod tests {
    use crate::cst::{
        ast::{AstNode, Document, FlagValue, Rule},
        parse_fsh,
    };

    #[test]
    fn test_profile_basic() {
        let source = r#"Profile: MyPatient
Parent: Patient
Id: my-patient
Title: "My Patient Profile"
Description: "A custom patient profile""#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
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
    #[ignore] // TODO: Rule path() API needs fixing - parser doesn't create Path nodes properly
    fn test_profile_with_rules() {
        let source = r#"Profile: MyPatient
Parent: Patient

* identifier 1..* MS
* name 1..1 MS SU
* gender from GenderValueSet (required)"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");

        let rules: Vec<_> = profile.rules().collect();
        assert_eq!(rules.len(), 3, "Should have 3 rules");

        // First rule: cardinality
        match &rules[0] {
            Rule::Card(card) => {
                assert_eq!(card.path().unwrap().as_string(), "identifier");
                assert!(card.cardinality_string().unwrap().contains("1..*"));
                assert!(card.flags().contains(&FlagValue::MustSupport));
            }
            _ => panic!("Expected CardRule"),
        }

        // Second rule: cardinality with multiple flags
        match &rules[1] {
            Rule::Card(card) => {
                assert_eq!(card.path().unwrap().as_string(), "name");
                assert!(card.cardinality_string().unwrap().contains("1..1"));
                let flags = card.flags();
                assert!(flags.contains(&FlagValue::MustSupport));
                assert!(flags.contains(&FlagValue::Summary));
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

        let (cst, _lexer_errors, errors) = parse_fsh(source);
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

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let valueset = doc.value_sets().next().expect("Should have a valueset");

        assert_eq!(valueset.name().as_deref(), Some("MyValueSet"));

        let id_clause = valueset.id();
        assert!(id_clause.is_some(), "ValueSet should have id clause");
        assert_eq!(id_clause.unwrap().value().as_deref(), Some("my-valueset"));
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

        let (cst, _lexer_errors, errors) = parse_fsh(source);
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
        let source = r#"Alias: SCT = http://snomed.info/sct
Alias: LOINC = http://loinc.org"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let aliases: Vec<_> = doc.aliases().collect();

        assert_eq!(aliases.len(), 2);

        assert_eq!(aliases[0].name().as_deref(), Some("SCT"));
        assert_eq!(
            aliases[0].value().as_deref(),
            Some("http://snomed.info/sct")
        );

        assert_eq!(aliases[1].name().as_deref(), Some("LOINC"));
        assert_eq!(aliases[1].value().as_deref(), Some("http://loinc.org"));
    }

    #[test]
    fn test_multiple_profiles() {
        let source = r#"Profile: FirstProfile
Parent: Patient

Profile: SecondProfile
Parent: Observation"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
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
    #[ignore] // TODO: Path parsing needs implementation
    fn test_path_segments() {
        let source = r#"Profile: MyProfile
Parent: Patient

* name.given 1..1"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");
        let rule = profile.rules().next().expect("Should have a rule");

        match rule {
            Rule::Card(card) => {
                let path = card.path().expect("Should have path");
                assert_eq!(path.as_string(), "name.given");
                let segments = path.segments_as_strings();
                assert_eq!(segments, vec!["name", "given"]);
            }
            _ => panic!("Expected CardRule"),
        }
    }

    #[test]
    #[ignore] // TODO: Fixed value rule parsing needs implementation
    fn test_fixed_value_rule() {
        let source = r#"Profile: MyProfile
Parent: Patient

* active = true
* gender = "male""#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
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
        // Use a simple but realistic profile example
        let source = r#"Profile: USCorePatient
Parent: Patient
Id: us-core-patient
Title: "US Core Patient Profile"
Description: "Patient profile for US Core"

* identifier 1..* MS
* name 1..* MS
* birthDate MS"#;
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        assert!(
            errors.is_empty(),
            "Should parse without errors, got: {:?}",
            errors
        );

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

        let (cst, _lexer_errors, errors) = parse_fsh(source);
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
        let source = r#"Profile: MyPatient
Parent: Patient
Id: my-patient

* name 1..1 MS"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(
            errors.is_empty(),
            "Should parse without errors, got: {:?}",
            errors
        );

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

    // ============================================================================
    // AST API Enhancement Tests (Task 4.6)
    // ============================================================================

    #[test]
    #[ignore] // TODO: Parser needs to create CardinalityNode properly
    fn test_structured_cardinality_access() {
        let source = r#"Profile: MyProfile
Parent: Patient

* identifier 0..1
* name 1..*
* gender 1..1"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");
        let rules: Vec<_> = profile.rules().collect();

        // Test structured cardinality access
        match &rules[0] {
            Rule::Card(card) => {
                let cardinality = card.cardinality().expect("Should have cardinality");
                assert_eq!(cardinality.min(), Some(0));
                assert_eq!(cardinality.max(), Some("1".to_string()));
                assert!(!cardinality.is_unbounded());
                assert_eq!(cardinality.as_string(), "0..1");
            }
            _ => panic!("Expected CardRule"),
        }

        // Test unbounded cardinality
        match &rules[1] {
            Rule::Card(card) => {
                let cardinality = card.cardinality().expect("Should have cardinality");
                assert_eq!(cardinality.min(), Some(1));
                assert_eq!(cardinality.max(), Some("*".to_string()));
                assert!(cardinality.is_unbounded());
                assert_eq!(cardinality.as_string(), "1..*");
            }
            _ => panic!("Expected CardRule"),
        }

        // Test single value cardinality
        match &rules[2] {
            Rule::Card(card) => {
                let cardinality = card.cardinality().expect("Should have cardinality");
                assert_eq!(cardinality.min(), Some(1));
                assert_eq!(cardinality.max(), Some("1".to_string()));
                assert!(!cardinality.is_unbounded());
                assert_eq!(cardinality.as_string(), "1..1");
            }
            _ => panic!("Expected CardRule"),
        }
    }

    #[test]
    #[ignore] // TODO: Parser needs to create PathSegment nodes properly
    fn test_structured_path_segments() {
        let source = r#"Profile: MyProfile
Parent: Patient

* name.given 1..1
* identifier.value 0..1
* contact.123 0..*"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");
        let rules: Vec<_> = profile.rules().collect();

        // Test structured path segments
        match &rules[0] {
            Rule::Card(card) => {
                let path = card.path().expect("Should have path");
                let segments: Vec<_> = path.segments().collect();
                assert_eq!(segments.len(), 2);

                assert_eq!(segments[0].identifier(), Some("name".to_string()));
                assert!(!segments[0].is_numeric());

                assert_eq!(segments[1].identifier(), Some("given".to_string()));
                assert!(!segments[1].is_numeric());
            }
            _ => panic!("Expected CardRule"),
        }

        // Test numeric path segment
        match &rules[2] {
            Rule::Card(card) => {
                let path = card.path().expect("Should have path");
                let segments: Vec<_> = path.segments().collect();
                assert_eq!(segments.len(), 2);

                assert_eq!(segments[0].identifier(), Some("contact".to_string()));
                assert!(!segments[0].is_numeric());

                assert_eq!(segments[1].identifier(), Some("123".to_string()));
                assert!(segments[1].is_numeric());
            }
            _ => panic!("Expected CardRule"),
        }
    }

    #[test]
    #[ignore] // TODO: Parser needs to create proper flag nodes
    fn test_enhanced_flag_access() {
        use crate::cst::ast::FlagValue;

        let source = r#"Profile: MyProfile
Parent: Patient

* identifier MS SU
* name TU N
* gender D ?!"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");
        let rules: Vec<_> = profile.rules().collect();

        // Test structured flag access
        match &rules[0] {
            Rule::Flag(flag) => {
                let flags = flag.flags();
                assert_eq!(flags.len(), 2);
                assert!(flags.contains(&FlagValue::MustSupport));
                assert!(flags.contains(&FlagValue::Summary));
                assert!(!flag.has_flag_conflicts());
            }
            _ => panic!("Expected FlagRule"),
        }

        // Test conflicting flags
        match &rules[1] {
            Rule::Flag(flag) => {
                let flags = flag.flags();
                assert_eq!(flags.len(), 2);
                assert!(flags.contains(&FlagValue::TrialUse));
                assert!(flags.contains(&FlagValue::Normative));
                assert!(flag.has_flag_conflicts());

                let conflicts = flag.flag_conflicts();
                assert_eq!(conflicts.len(), 1);
                assert_eq!(conflicts[0], (FlagValue::TrialUse, FlagValue::Normative));
            }
            _ => panic!("Expected FlagRule"),
        }
    }

    #[test]
    #[ignore] // TODO: Parser needs to create proper ValueSet component nodes
    fn test_enhanced_valueset_structure() {
        let source = r#"ValueSet: MyValueSet
* include codes from system http://snomed.info/sct where concept is-a #123456
* exclude codes from valueset OtherValueSet"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let valueset = doc.value_sets().next().expect("Should have a valueset");
        let rules: Vec<_> = valueset.rules().collect();

        // Test structured ValueSet component access
        match &rules[0] {
            Rule::ValueSet(vs_rule) => {
                let components: Vec<_> = vs_rule.value_set_components().collect();
                assert_eq!(components.len(), 1);

                let component = &components[0];
                assert!(component.is_include());
                assert!(!component.is_exclude());

                if let Some(filter_comp) = component.filter() {
                    let filters = filter_comp.filters();
                    assert_eq!(filters.len(), 1);

                    let filter = &filters[0];
                    assert_eq!(filter.property(), Some("concept".to_string()));
                    assert_eq!(filter.operator_string(), Some("is-a".to_string()));
                    assert_eq!(filter.value_string(), Some("#123456".to_string()));
                }
            }
            _ => panic!("Expected ValueSetRule"),
        }
    }

    #[test]
    #[ignore] // TODO: Parser needs to create proper CodeCaretValueRule nodes
    fn test_code_caret_value_rule_api() {
        let source = r#"Profile: MyProfile
Parent: Patient

* #active ^short = "Patient is active"
* #gender ^definition = "Patient gender""#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");
        let rules: Vec<_> = profile.rules().collect();

        // Test CodeCaretValueRule API
        match &rules[0] {
            Rule::CodeCaretValue(code_caret) => {
                assert_eq!(code_caret.code_value(), Some("active".to_string()));

                let caret_path = code_caret.caret_path().expect("Should have caret path");
                assert_eq!(caret_path.as_string(), "short");

                assert_eq!(
                    code_caret.assigned_value(),
                    Some("Patient is active".to_string())
                );
            }
            _ => panic!("Expected CodeCaretValueRule"),
        }

        match &rules[1] {
            Rule::CodeCaretValue(code_caret) => {
                assert_eq!(code_caret.code_value(), Some("gender".to_string()));

                let caret_path = code_caret.caret_path().expect("Should have caret path");
                assert_eq!(caret_path.as_string(), "definition");

                assert_eq!(
                    code_caret.assigned_value(),
                    Some("Patient gender".to_string())
                );
            }
            _ => panic!("Expected CodeCaretValueRule"),
        }
    }

    #[test]
    #[ignore] // TODO: Parser needs to create proper CodeInsertRule nodes
    fn test_code_insert_rule_api() {
        let source = r#"Profile: MyProfile
Parent: Patient

* #active insert ActiveElementRules
* #gender insert GenderElementRules(male, female)"#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let profile = doc.profiles().next().expect("Should have a profile");
        let rules: Vec<_> = profile.rules().collect();

        // Test CodeInsertRule API
        match &rules[0] {
            Rule::CodeInsert(code_insert) => {
                assert_eq!(code_insert.code_value(), Some("active".to_string()));
                assert_eq!(
                    code_insert.ruleset_reference(),
                    Some("ActiveElementRules".to_string())
                );

                let args = code_insert.arguments();
                assert_eq!(args.len(), 0);
            }
            _ => panic!("Expected CodeInsertRule"),
        }

        match &rules[1] {
            Rule::CodeInsert(code_insert) => {
                assert_eq!(code_insert.code_value(), Some("gender".to_string()));
                assert_eq!(
                    code_insert.ruleset_reference(),
                    Some("GenderElementRules".to_string())
                );

                let args = code_insert.arguments();
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], "male");
                assert_eq!(args[1], "female");
            }
            _ => panic!("Expected CodeInsertRule"),
        }
    }

    #[test]
    #[ignore] // TODO: Parser needs to create proper VsFilterDefinition nodes
    fn test_vs_filter_definition_api() {
        let source = r#"ValueSet: MyValueSet
* include codes from system http://snomed.info/sct where concept is-a #123456 and display regex ".*test.*""#;

        let (cst, _lexer_errors, errors) = parse_fsh(source);
        assert!(errors.is_empty());

        let doc = Document::cast(cst).expect("Should be a document");
        let valueset = doc.value_sets().next().expect("Should have a valueset");
        let rules: Vec<_> = valueset.rules().collect();

        match &rules[0] {
            Rule::ValueSet(vs_rule) => {
                let filter_defs: Vec<_> = vs_rule.filter_definitions().collect();
                assert_eq!(filter_defs.len(), 1);

                let filter = &filter_defs[0];
                assert_eq!(filter.property(), Some("concept".to_string()));
                assert_eq!(filter.operator_string(), Some("is-a".to_string()));
                assert_eq!(filter.value_string(), Some("#123456".to_string()));

                // Test chained filters
                let chained = filter.chained_filters();
                assert_eq!(chained.len(), 1);

                let chained_filter = &chained[0];
                assert_eq!(chained_filter.property(), Some("display".to_string()));
                assert_eq!(chained_filter.operator_string(), Some("regex".to_string()));
                assert_eq!(chained_filter.value_string(), Some(".*test.*".to_string()));
            }
            _ => panic!("Expected ValueSetRule"),
        }
    }
}
