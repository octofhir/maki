//! Assignment rule extractor for fixed[x] and pattern[x] values
//!
//! This is the most complex extractor, handling 20+ fixed[x] types and 10+ pattern[x] types.
//! Converts FHIR constraint values into FSH assignment rules.

use super::RuleExtractor;
use crate::{
    Error, Result,
    exportable::{
        AssignmentRule, ExportableRule, FshCode, FshCodeableConcept, FshCoding, FshQuantity,
        FshReference, FshValue,
    },
    models::common::{CodeableConcept, Coding, Identifier, Quantity, Reference},
    processor::ProcessableElementDefinition,
};

/// Extractor for assignment rules (fixed[x] and pattern[x] values)
pub struct AssignmentExtractor;

impl AssignmentExtractor {
    /// Create an assignment rule for the given element and value
    fn create_assignment_rule(
        elem: &ProcessableElementDefinition,
        value: FshValue,
        exactly: bool,
    ) -> AssignmentRule {
        AssignmentRule {
            path: elem.element.fsh_path(),
            value,
            exactly,
        }
    }

    /// Convert FHIR CodeableConcept to FSH value
    fn convert_codeable_concept(cc: &CodeableConcept) -> Result<FshValue> {
        let mut codings = Vec::new();

        // Convert all codings
        if let Some(coding_list) = &cc.coding {
            for coding in coding_list {
                codings.push(Self::convert_coding(coding)?);
            }
        }

        Ok(FshValue::CodeableConcept(FshCodeableConcept {
            codings,
            text: cc.text.clone(),
        }))
    }

    /// Convert FHIR Coding to FSH Coding
    fn convert_coding(coding: &Coding) -> Result<FshCoding> {
        // Must have a code
        let code = coding
            .code
            .clone()
            .ok_or_else(|| Error::Processing("Coding must have a code".to_string()))?;

        Ok(FshCoding {
            system: coding.system.clone(),
            code,
            display: coding.display.clone(),
        })
    }

    /// Convert FHIR Quantity to FSH Quantity
    fn convert_quantity(qty: &Quantity) -> Result<FshValue> {
        Ok(FshValue::Quantity(FshQuantity {
            value: qty.value,
            unit: qty.unit.clone().or_else(|| qty.code.clone()),
            system: qty.system.clone(),
            code: qty.code.clone(),
        }))
    }

    /// Convert FHIR Identifier to FSH value
    /// Since FSH doesn't have native Identifier syntax, we need to use nested path rules
    /// For now, we'll skip complex Identifier assignments
    fn convert_identifier(_identifier: &Identifier) -> Result<Option<FshValue>> {
        // Identifier is complex and typically needs multiple assignment rules
        // Skip for now - will be handled by nested path assignments
        Ok(None)
    }

    /// Convert FHIR Reference to FSH Reference
    fn convert_reference(reference: &Reference) -> Result<FshValue> {
        let reference_str = reference.reference.clone().ok_or_else(|| {
            Error::Processing("Reference must have a reference value".to_string())
        })?;

        Ok(FshValue::Reference(FshReference {
            reference: reference_str,
            display: reference.display.clone(),
        }))
    }
}

impl RuleExtractor for AssignmentExtractor {
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule>>> {
        let mut rules: Vec<Box<dyn ExportableRule>> = Vec::new();

        // Process fixed[x] values (use exactly = true)
        // These are constraints that fix the value exactly

        // Primitive fixed values
        if let Some(value) = elem.element.fixed_boolean
            && !elem.is_processed("fixedBoolean")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Boolean(value),
                true,
            )));
            elem.mark_processed("fixedBoolean");
        }

        if let Some(value) = elem.element.fixed_integer
            && !elem.is_processed("fixedInteger")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Integer(value),
                true,
            )));
            elem.mark_processed("fixedInteger");
        }

        if let Some(value) = elem.element.fixed_decimal
            && !elem.is_processed("fixedDecimal")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Decimal(value),
                true,
            )));
            elem.mark_processed("fixedDecimal");
        }

        if let Some(ref value) = elem.element.fixed_string
            && !elem.is_processed("fixedString")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::String(value.clone()),
                true,
            )));
            elem.mark_processed("fixedString");
        }

        if let Some(ref value) = elem.element.fixed_code
            && !elem.is_processed("fixedCode")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Code(FshCode {
                    system: None,
                    code: value.clone(),
                }),
                true,
            )));
            elem.mark_processed("fixedCode");
        }

        // URI/URL types
        if let Some(ref value) = elem.element.fixed_uri
            && !elem.is_processed("fixedUri")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Url(value.clone()),
                true,
            )));
            elem.mark_processed("fixedUri");
        }

        if let Some(ref value) = elem.element.fixed_url
            && !elem.is_processed("fixedUrl")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Url(value.clone()),
                true,
            )));
            elem.mark_processed("fixedUrl");
        }

        if let Some(ref value) = elem.element.fixed_canonical
            && !elem.is_processed("fixedCanonical")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Canonical(value.clone()),
                true,
            )));
            elem.mark_processed("fixedCanonical");
        }

        // Date/time types
        if let Some(ref value) = elem.element.fixed_date
            && !elem.is_processed("fixedDate")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::String(value.clone()),
                true,
            )));
            elem.mark_processed("fixedDate");
        }

        if let Some(ref value) = elem.element.fixed_date_time
            && !elem.is_processed("fixedDateTime")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::String(value.clone()),
                true,
            )));
            elem.mark_processed("fixedDateTime");
        }

        if let Some(ref value) = elem.element.fixed_instant
            && !elem.is_processed("fixedInstant")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::String(value.clone()),
                true,
            )));
            elem.mark_processed("fixedInstant");
        }

        if let Some(ref value) = elem.element.fixed_time
            && !elem.is_processed("fixedTime")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::String(value.clone()),
                true,
            )));
            elem.mark_processed("fixedTime");
        }

        // ID types
        if let Some(ref value) = elem.element.fixed_id
            && !elem.is_processed("fixedId")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Id(value.clone()),
                true,
            )));
            elem.mark_processed("fixedId");
        }

        if let Some(ref value) = elem.element.fixed_oid
            && !elem.is_processed("fixedOid")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Oid(value.clone()),
                true,
            )));
            elem.mark_processed("fixedOid");
        }

        if let Some(ref value) = elem.element.fixed_uuid
            && !elem.is_processed("fixedUuid")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Uuid(value.clone()),
                true,
            )));
            elem.mark_processed("fixedUuid");
        }

        // Complex types - fixed values
        if let Some(ref cc) = elem.element.fixed_codeable_concept
            && !elem.is_processed("fixedCodeableConcept")
        {
            let fsh_value = Self::convert_codeable_concept(cc)?;
            rules.push(Box::new(Self::create_assignment_rule(
                elem, fsh_value, true,
            )));
            elem.mark_processed("fixedCodeableConcept");
        }

        if let Some(ref coding) = elem.element.fixed_coding
            && !elem.is_processed("fixedCoding")
        {
            let fsh_coding = Self::convert_coding(coding)?;
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Coding(fsh_coding),
                true,
            )));
            elem.mark_processed("fixedCoding");
        }

        if let Some(ref qty) = elem.element.fixed_quantity
            && !elem.is_processed("fixedQuantity")
        {
            let fsh_value = Self::convert_quantity(qty)?;
            rules.push(Box::new(Self::create_assignment_rule(
                elem, fsh_value, true,
            )));
            elem.mark_processed("fixedQuantity");
        }

        if let Some(ref identifier) = elem.element.fixed_identifier
            && !elem.is_processed("fixedIdentifier")
        {
            // Identifier is complex - skip for now
            if let Some(fsh_value) = Self::convert_identifier(identifier)? {
                rules.push(Box::new(Self::create_assignment_rule(
                    elem, fsh_value, true,
                )));
            }
            elem.mark_processed("fixedIdentifier");
        }

        if let Some(ref reference) = elem.element.fixed_reference
            && !elem.is_processed("fixedReference")
        {
            let fsh_value = Self::convert_reference(reference)?;
            rules.push(Box::new(Self::create_assignment_rule(
                elem, fsh_value, true,
            )));
            elem.mark_processed("fixedReference");
        }

        // Process pattern[x] values (use exactly = false)
        // These are constraints that match patterns

        if let Some(value) = elem.element.pattern_boolean
            && !elem.is_processed("patternBoolean")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Boolean(value),
                false,
            )));
            elem.mark_processed("patternBoolean");
        }

        if let Some(value) = elem.element.pattern_integer
            && !elem.is_processed("patternInteger")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Integer(value),
                false,
            )));
            elem.mark_processed("patternInteger");
        }

        if let Some(value) = elem.element.pattern_decimal
            && !elem.is_processed("patternDecimal")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Decimal(value),
                false,
            )));
            elem.mark_processed("patternDecimal");
        }

        if let Some(ref value) = elem.element.pattern_string
            && !elem.is_processed("patternString")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::String(value.clone()),
                false,
            )));
            elem.mark_processed("patternString");
        }

        if let Some(ref value) = elem.element.pattern_code
            && !elem.is_processed("patternCode")
        {
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Code(FshCode {
                    system: None,
                    code: value.clone(),
                }),
                false,
            )));
            elem.mark_processed("patternCode");
        }

        // Complex types - pattern values
        if let Some(ref cc) = elem.element.pattern_codeable_concept
            && !elem.is_processed("patternCodeableConcept")
        {
            let fsh_value = Self::convert_codeable_concept(cc)?;
            rules.push(Box::new(Self::create_assignment_rule(
                elem, fsh_value, false,
            )));
            elem.mark_processed("patternCodeableConcept");
        }

        if let Some(ref coding) = elem.element.pattern_coding
            && !elem.is_processed("patternCoding")
        {
            let fsh_coding = Self::convert_coding(coding)?;
            rules.push(Box::new(Self::create_assignment_rule(
                elem,
                FshValue::Coding(fsh_coding),
                false,
            )));
            elem.mark_processed("patternCoding");
        }

        if let Some(ref qty) = elem.element.pattern_quantity
            && !elem.is_processed("patternQuantity")
        {
            let fsh_value = Self::convert_quantity(qty)?;
            rules.push(Box::new(Self::create_assignment_rule(
                elem, fsh_value, false,
            )));
            elem.mark_processed("patternQuantity");
        }

        if let Some(ref identifier) = elem.element.pattern_identifier
            && !elem.is_processed("patternIdentifier")
        {
            // Identifier is complex - skip for now
            if let Some(fsh_value) = Self::convert_identifier(identifier)? {
                rules.push(Box::new(Self::create_assignment_rule(
                    elem, fsh_value, false,
                )));
            }
            elem.mark_processed("patternIdentifier");
        }

        if let Some(ref reference) = elem.element.pattern_reference
            && !elem.is_processed("patternReference")
        {
            let fsh_value = Self::convert_reference(reference)?;
            rules.push(Box::new(Self::create_assignment_rule(
                elem, fsh_value, false,
            )));
            elem.mark_processed("patternReference");
        }

        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ElementDefinition;

    fn create_test_element() -> ProcessableElementDefinition {
        ProcessableElementDefinition::new(ElementDefinition {
            id: "Patient.status".to_string(),
            path: "Patient.status".to_string(),
            slice_name: None,
            min: None,
            max: None,
            type_: None,
            must_support: None,
            is_modifier: None,
            is_summary: None,
            binding: None,
            constraint: None,
            slicing: None,
            fixed_boolean: None,
            fixed_integer: None,
            fixed_decimal: None,
            fixed_string: None,
            fixed_uri: None,
            fixed_url: None,
            fixed_canonical: None,
            fixed_code: None,
            fixed_date: None,
            fixed_date_time: None,
            fixed_instant: None,
            fixed_time: None,
            fixed_id: None,
            fixed_oid: None,
            fixed_uuid: None,
            fixed_codeable_concept: None,
            fixed_coding: None,
            fixed_quantity: None,
            fixed_identifier: None,
            fixed_reference: None,
            pattern_boolean: None,
            pattern_integer: None,
            pattern_decimal: None,
            pattern_string: None,
            pattern_code: None,
            pattern_codeable_concept: None,
            pattern_coding: None,
            pattern_quantity: None,
            pattern_identifier: None,
            pattern_reference: None,
            short: None,
            definition: None,
            comment: None,
            requirements: None,
            alias: None,
            example: None,
        })
    }

    #[test]
    fn test_extract_fixed_boolean() {
        let mut elem = create_test_element();
        elem.element.fixed_boolean = Some(true);

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "status = true");
        assert!(elem.is_processed("fixedBoolean"));
    }

    #[test]
    fn test_extract_fixed_integer() {
        let mut elem = create_test_element();
        elem.element.fixed_integer = Some(42);

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "status = 42");
        assert!(elem.is_processed("fixedInteger"));
    }

    #[test]
    fn test_extract_fixed_string() {
        let mut elem = create_test_element();
        elem.element.fixed_string = Some("hello world".to_string());

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "status = \"hello world\"");
        assert!(elem.is_processed("fixedString"));
    }

    #[test]
    fn test_extract_fixed_code() {
        let mut elem = create_test_element();
        elem.element.fixed_code = Some("active".to_string());

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "status = #active");
        assert!(elem.is_processed("fixedCode"));
    }

    #[test]
    fn test_extract_fixed_coding() {
        let mut elem = create_test_element();
        elem.element.fixed_coding = Some(Coding {
            system: Some("http://hl7.org/fhir/status".to_string()),
            version: None,
            code: Some("active".to_string()),
            display: Some("Active".to_string()),
        });

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(
            rules[0].to_fsh(),
            "status = http://hl7.org/fhir/status#active \"Active\""
        );
        assert!(elem.is_processed("fixedCoding"));
    }

    #[test]
    fn test_extract_fixed_codeable_concept() {
        let mut elem = create_test_element();
        elem.element.fixed_codeable_concept = Some(CodeableConcept {
            coding: Some(vec![Coding {
                system: Some("http://snomed.info/sct".to_string()),
                version: None,
                code: Some("123456".to_string()),
                display: Some("Example".to_string()),
            }]),
            text: Some("Example text".to_string()),
        });

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        let fsh = rules[0].to_fsh();
        assert!(fsh.contains("http://snomed.info/sct#123456"));
        assert!(fsh.contains("Example"));
        assert!(elem.is_processed("fixedCodeableConcept"));
    }

    #[test]
    fn test_extract_fixed_quantity() {
        let mut elem = create_test_element();
        elem.element.fixed_quantity = Some(Quantity {
            value: Some(5.0),
            unit: Some("mg".to_string()),
            system: None,
            code: None,
        });

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "status = 5 'mg'");
        assert!(elem.is_processed("fixedQuantity"));
    }

    #[test]
    fn test_extract_pattern_boolean() {
        let mut elem = create_test_element();
        elem.element.pattern_boolean = Some(false);

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "status = false");
        assert!(elem.is_processed("patternBoolean"));
    }

    #[test]
    fn test_extract_pattern_code() {
        let mut elem = create_test_element();
        elem.element.pattern_code = Some("draft".to_string());

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "status = #draft");
        assert!(elem.is_processed("patternCode"));
    }

    #[test]
    fn test_extract_multiple_values() {
        let mut elem = create_test_element();
        elem.element.fixed_code = Some("active".to_string());
        elem.element.pattern_boolean = Some(true);

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        // Both should be extracted
        assert_eq!(rules.len(), 2);
        assert!(elem.is_processed("fixedCode"));
        assert!(elem.is_processed("patternBoolean"));
    }

    #[test]
    fn test_extract_no_duplicate_processing() {
        let mut elem = create_test_element();
        elem.element.fixed_code = Some("active".to_string());

        let extractor = AssignmentExtractor;

        // First extraction
        let rules1 = extractor.extract(&mut elem).unwrap();
        assert_eq!(rules1.len(), 1);

        // Second extraction should return empty (already processed)
        let rules2 = extractor.extract(&mut elem).unwrap();
        assert_eq!(rules2.len(), 0);
    }

    #[test]
    fn test_convert_codeable_concept_with_multiple_codings() {
        let cc = CodeableConcept {
            coding: Some(vec![
                Coding {
                    system: Some("http://snomed.info/sct".to_string()),
                    version: None,
                    code: Some("123".to_string()),
                    display: Some("First".to_string()),
                },
                Coding {
                    system: Some("http://loinc.org".to_string()),
                    version: None,
                    code: Some("456".to_string()),
                    display: None,
                },
            ]),
            text: None,
        };

        let result = AssignmentExtractor::convert_codeable_concept(&cc).unwrap();
        let fsh = result.to_fsh();

        assert!(fsh.contains("http://snomed.info/sct#123"));
        assert!(fsh.contains("http://loinc.org#456"));
    }

    #[test]
    fn test_convert_coding_without_code_fails() {
        let coding = Coding {
            system: Some("http://example.org".to_string()),
            version: None,
            code: None,
            display: Some("Display".to_string()),
        };

        let result = AssignmentExtractor::convert_coding(&coding);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_quantity_without_value() {
        let qty = Quantity {
            value: None,
            unit: Some("mg".to_string()),
            system: None,
            code: None,
        };

        let result = AssignmentExtractor::convert_quantity(&qty);
        // Implementation allows None value - creates quantity without value
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_reference() {
        let reference = Reference {
            reference: Some("Patient/123".to_string()),
            display: Some("John Doe".to_string()),
            type_: None,
        };

        let result = AssignmentExtractor::convert_reference(&reference).unwrap();
        // Reference includes display when present
        assert_eq!(result.to_fsh(), "Reference(Patient/123) \"John Doe\"");
    }

    #[test]
    fn test_fixed_url() {
        let mut elem = create_test_element();
        elem.element.fixed_url = Some("http://example.org".to_string());

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "status = \"http://example.org\"");
        assert!(elem.is_processed("fixedUrl"));
    }

    #[test]
    fn test_fixed_canonical() {
        let mut elem = create_test_element();
        elem.element.fixed_canonical =
            Some("http://example.org/StructureDefinition/MyProfile".to_string());

        let extractor = AssignmentExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(rules[0].to_fsh().contains("Canonical"));
        assert!(elem.is_processed("fixedCanonical"));
    }
}
