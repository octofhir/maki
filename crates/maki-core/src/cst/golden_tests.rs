//! Golden file tests for CST parser
//!
//! These tests use real FSH files from the examples/ directory to verify
//! the lossless property holds for all constructs.

#[cfg(test)]
mod tests {
    use crate::cst::parse_fsh;

    #[test]
    fn test_patient_profile_lossless() {
        let source = include_str!("../../../../examples/patient-profile.fsh");
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        // Should have no lexer errors
        assert!(errors.is_empty(), "Lexer errors: {errors:?}");

        // Perfect lossless roundtrip
        assert_eq!(
            cst.text().to_string(),
            source,
            "Lossless property failed for patient-profile.fsh"
        );
    }

    #[test]
    fn test_comprehensive_test_lossless() {
        let source = include_str!("../../../../examples/comprehensive-test.fsh");
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        assert!(errors.is_empty(), "Lexer errors: {errors:?}");
        assert_eq!(
            cst.text().to_string(),
            source,
            "Lossless property failed for comprehensive-test.fsh"
        );
    }

    #[test]
    fn test_naming_issues_lossless() {
        let source = include_str!("../../../../examples/naming-issues.fsh");
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);
    }

    #[test]
    fn test_missing_metadata_lossless() {
        let source = include_str!("../../../../examples/missing-metadata.fsh");
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);
    }

    #[test]
    fn test_invalid_cardinality_lossless() {
        let source = include_str!("../../../../examples/invalid-cardinality.fsh");
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);
    }

    #[test]
    fn test_extension_issues_lossless() {
        let source = include_str!("../../../../examples/extension-issues.fsh");
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);
    }

    #[test]
    fn test_valueset_examples_lossless() {
        let source = include_str!("../../../../examples/valueset-examples.fsh");
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);
    }

    #[test]
    fn test_lexer_edge_cases_lossless() {
        let source = include_str!("../../../../examples/lexer-edge-cases.fsh");
        let (cst, _lexer_errors, errors) = parse_fsh(source);

        assert!(
            errors.is_empty(),
            "Lexer errors detected in lexer-edge-cases.fsh: {errors:?}"
        );
        assert_eq!(
            cst.text().to_string(),
            source,
            "Lossless property failed for lexer-edge-cases.fsh"
        );
    }
}
