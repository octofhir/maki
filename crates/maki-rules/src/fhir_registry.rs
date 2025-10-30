//! FHIR resource registry
//!
//! Contains comprehensive lists of FHIR base resources for validation.

use once_cell::sync::Lazy;
use std::collections::HashSet;

/// FHIR version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FhirVersion {
    R4,
    R5,
}

/// Complete list of FHIR R4 base resources
static FHIR_R4_RESOURCES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        // Foundation
        "CapabilityStatement",
        "StructureDefinition",
        "ImplementationGuide",
        "SearchParameter",
        "MessageDefinition",
        "OperationDefinition",
        "CompartmentDefinition",
        "StructureMap",
        "GraphDefinition",
        "ExampleScenario",
        // Terminology
        "CodeSystem",
        "ValueSet",
        "ConceptMap",
        "NamingSystem",
        "TerminologyCapabilities",
        // Security & Privacy
        "Provenance",
        "AuditEvent",
        "Consent",
        // Documents
        "Composition",
        "DocumentManifest",
        "DocumentReference",
        // Workflow
        "Task",
        "Appointment",
        "AppointmentResponse",
        "Schedule",
        "Slot",
        "VerificationResult",
        // Financial
        "Coverage",
        "CoverageEligibilityRequest",
        "CoverageEligibilityResponse",
        "EnrollmentRequest",
        "EnrollmentResponse",
        "Claim",
        "ClaimResponse",
        "Invoice",
        "PaymentNotice",
        "PaymentReconciliation",
        "Account",
        "ChargeItem",
        "ChargeItemDefinition",
        "Contract",
        "ExplanationOfBenefit",
        "InsurancePlan",
        // Clinical
        "AllergyIntolerance",
        "AdverseEvent",
        "Condition",
        "Procedure",
        "FamilyMemberHistory",
        "ClinicalImpression",
        "DetectedIssue",
        "Observation",
        "Media",
        "DiagnosticReport",
        "Specimen",
        "BodyStructure",
        "ImagingStudy",
        "QuestionnaireResponse",
        "MolecularSequence",
        // Care Provision
        "CarePlan",
        "CareTeam",
        "Goal",
        "ServiceRequest",
        "NutritionOrder",
        "VisionPrescription",
        "RiskAssessment",
        "RequestGroup",
        // Medication
        "MedicationRequest",
        "MedicationAdministration",
        "MedicationDispense",
        "MedicationStatement",
        "Medication",
        "MedicationKnowledge",
        "Immunization",
        "ImmunizationEvaluation",
        "ImmunizationRecommendation",
        // Diagnostics
        "DeviceRequest",
        "DeviceUseStatement",
        "DeviceMetric",
        "Device",
        "DeviceDefinition",
        // Individuals
        "Patient",
        "Practitioner",
        "PractitionerRole",
        "RelatedPerson",
        "Person",
        "Group",
        // Entities
        "Organization",
        "OrganizationAffiliation",
        "HealthcareService",
        "Endpoint",
        "Location",
        "Substance",
        "BiologicallyDerivedProduct",
        // Workflow
        "Encounter",
        "EpisodeOfCare",
        "Flag",
        "List",
        "Library",
        "Measure",
        "MeasureReport",
        "ResearchStudy",
        "ResearchSubject",
        // Public Health & Research
        "EffectEvidenceSynthesis",
        "Evidence",
        "EvidenceVariable",
        "RiskEvidenceSynthesis",
        // Specialized
        "Questionnaire",
        "SupplyRequest",
        "SupplyDelivery",
        "CatalogEntry",
        "EventDefinition",
        "ObservationDefinition",
        "SpecimenDefinition",
        "ActivityDefinition",
        "PlanDefinition",
        // Definitional Artifacts
        "Linkage",
        "MessageHeader",
        "Bundle",
        "Binary",
        "Basic",
        "Parameters",
        "Subscription",
        "OperationOutcome",
        // Special
        "Extension",
        "DomainResource",
        "Resource",
    ])
});

/// Complete list of FHIR R5 base resources
static FHIR_R5_RESOURCES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut resources = FHIR_R4_RESOURCES.clone();

    // Add R5-specific resources
    resources.extend([
        "ArtifactAssessment",
        "Citation",
        "ConditionDefinition",
        "InventoryReport",
        "InventoryItem",
        "DeviceDispense",
        "DeviceUsage",
        "FormularyItem",
        "GenomicStudy",
        "NutritionIntake",
        "NutritionProduct",
        "Permission",
        "RegulatedAuthorization",
        "Ingredient",
        "ManufacturedItemDefinition",
        "AdministrableProductDefinition",
        "PackagedProductDefinition",
        "ClinicalUseDefinition",
        "MedicinalProductDefinition",
        "SubstanceDefinition",
        "SubstanceNucleicAcid",
        "SubstancePolymer",
        "SubstanceProtein",
        "SubstanceReferenceInformation",
        "SubstanceSourceMaterial",
        "Transport",
        "SubscriptionStatus",
        "SubscriptionTopic",
        "ActorDefinition",
        "Requirements",
        "TestPlan",
        "TestReport",
        "TestScript",
    ]);

    // Remove R4 resources that were removed/renamed in R5
    resources.remove("DocumentManifest");

    resources
});

/// Common external profile prefixes that should not trigger warnings
static KNOWN_EXTERNAL_PREFIXES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "USCore", "mcode-", "QICore", "CARIN", "DaVinci", "PACIO", "IPA", "IHE",
        "AU", // Australian profiles
        "UK", // UK profiles
        "CA", // Canadian profiles
        "CH", // Swiss profiles
    ])
});

/// Check if a name is a known FHIR base resource
pub fn is_fhir_resource(name: &str, version: FhirVersion) -> bool {
    match version {
        FhirVersion::R4 => FHIR_R4_RESOURCES.contains(name),
        FhirVersion::R5 => FHIR_R5_RESOURCES.contains(name),
    }
}

/// Check if a name looks like a canonical URL
pub fn is_canonical_url(name: &str) -> bool {
    name.starts_with("http://") || name.starts_with("https://")
}

/// Check if a name looks like it might be from a known external IG
pub fn is_likely_external_profile(name: &str) -> bool {
    KNOWN_EXTERNAL_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Validate a canonical URL structure
pub fn validate_canonical_url(url: &str) -> Result<(), String> {
    if !is_canonical_url(url) {
        return Err("Not a valid URL (must start with http:// or https://)".to_string());
    }

    // Basic URL structure validation
    if url.len() < 10 {
        return Err("URL is too short to be valid".to_string());
    }

    // Check for common canonical URL patterns for FHIR profiles
    if url.contains("/StructureDefinition/")
        || url.contains("/ValueSet/")
        || url.contains("/CodeSystem/")
        || url.contains("/Extension/")
    {
        Ok(())
    } else {
        // It's a URL but might not be a FHIR canonical - we'll allow it with a note
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fhir_r4_resources() {
        assert!(is_fhir_resource("Patient", FhirVersion::R4));
        assert!(is_fhir_resource("Observation", FhirVersion::R4));
        assert!(is_fhir_resource("Extension", FhirVersion::R4));
        assert!(!is_fhir_resource("NotAResource", FhirVersion::R4));
    }

    #[test]
    fn test_fhir_r5_resources() {
        assert!(is_fhir_resource("Patient", FhirVersion::R5));
        assert!(is_fhir_resource("Permission", FhirVersion::R5));
        assert!(is_fhir_resource("Transport", FhirVersion::R5));
        assert!(!is_fhir_resource("DocumentManifest", FhirVersion::R5)); // Removed in R5
    }

    #[test]
    fn test_canonical_url_detection() {
        assert!(is_canonical_url(
            "http://example.com/StructureDefinition/MyProfile"
        ));
        assert!(is_canonical_url(
            "https://hl7.org/fhir/us/core/StructureDefinition/us-core-patient"
        ));
        assert!(!is_canonical_url("Patient"));
        assert!(!is_canonical_url("USCorePatient"));
    }

    #[test]
    fn test_external_profile_detection() {
        assert!(is_likely_external_profile("USCorePatient"));
        assert!(is_likely_external_profile("mcode-primary-cancer-condition"));
        assert!(is_likely_external_profile("QICoreCondition"));
        assert!(!is_likely_external_profile("MyCustomProfile"));
    }

    #[test]
    fn test_canonical_url_validation() {
        assert!(
            validate_canonical_url(
                "http://hl7.org/fhir/us/core/StructureDefinition/us-core-patient"
            )
            .is_ok()
        );
        assert!(
            validate_canonical_url("https://example.com/fhir/StructureDefinition/my-profile")
                .is_ok()
        );
        assert!(validate_canonical_url("Patient").is_err());
        assert!(validate_canonical_url("http://x").is_err());
    }
}
