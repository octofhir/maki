// Test file for Parent keyword validation

// ✅ Valid: FHIR base resource
Profile: ValidPatientProfile
Parent: Patient
Id: valid-patient-profile
Title: "Valid Patient Profile"
Description: "This should pass - Patient is a valid FHIR R4 resource"

// ✅ Valid: Another FHIR base resource
Profile: ValidObservationProfile
Parent: Observation
Id: valid-observation-profile
Title: "Valid Observation Profile"
Description: "This should pass - Observation is a valid FHIR R4 resource"

// ✅ Valid: Extension as parent
Profile: ValidExtensionProfile
Parent: Extension
Id: valid-extension-profile
Title: "Valid Extension Profile"
Description: "This should pass - Extension is a valid FHIR resource"

// ✅ Valid: Profile inheriting from another profile (defined in same file)
Profile: BasePatientProfile
Parent: Patient
Id: base-patient-profile
Title: "Base Patient Profile"
Description: "Base profile"

Profile: DerivedPatientProfile
Parent: BasePatientProfile
Id: derived-patient-profile
Title: "Derived Patient Profile"
Description: "This should pass - BasePatientProfile is defined in this file"

// ⚠️ Warning: External IG profile (US Core)
Profile: ExtendedUSCorePatient
Parent: USCorePatientProfile
Id: extended-uscore-patient
Title: "Extended US Core Patient"
Description: "This should warn - USCorePatientProfile is from external IG"

// ⚠️ Warning: External IG profile (mcode)
Profile: ExtendedMCodeProfile
Parent: mcode-primary-cancer-condition
Id: extended-mcode-profile
Title: "Extended mCODE Profile"
Description: "This should warn - mcode-primary-cancer-condition is from external IG"

// ✅ Valid: Canonical URL
Profile: ProfileWithCanonicalURL
Parent: http://hl7.org/fhir/us/core/StructureDefinition/us-core-patient
Id: profile-with-canonical-url
Title: "Profile With Canonical URL"
Description: "This should pass - valid canonical URL format"

// ✅ Valid: Another canonical URL
Profile: ProfileWithHTTPSURL
Parent: https://example.org/fhir/StructureDefinition/my-custom-profile
Id: profile-with-https-url
Title: "Profile With HTTPS URL"
Description: "This should pass - valid canonical URL format"

// ⚠️ Warning: Unknown parent (might be typo)
Profile: ProfileWithUnknownParent
Parent: UnknownResourceType
Id: profile-with-unknown-parent
Title: "Profile With Unknown Parent"
Description: "This should warn - UnknownResourceType is not a known FHIR resource or defined profile"

// ❌ Error: Missing Parent
Profile: ProfileWithoutParent
Id: profile-without-parent
Title: "Profile Without Parent"
Description: "This should error - Parent is required for profiles"

// ❌ Error: Invalid URL format
Profile: ProfileWithBadURL
Parent: http://x
Id: profile-with-bad-url
Title: "Profile With Bad URL"
Description: "This should error - URL is too short to be valid"

// ✅ Valid: R4/R5 common resources
Profile: ValidConditionProfile
Parent: Condition
Id: valid-condition-profile
Title: "Valid Condition Profile"

Profile: ValidProcedureProfile
Parent: Procedure
Id: valid-procedure-profile
Title: "Valid Procedure Profile"

Profile: ValidEncounterProfile
Parent: Encounter
Id: valid-encounter-profile
Title: "Valid Encounter Profile"

Profile: ValidMedicationRequestProfile
Parent: MedicationRequest
Id: valid-medication-request-profile
Title: "Valid Medication Request Profile"

// ✅ Valid: ValueSet parent
Profile: ValidValueSetProfile
Parent: ValueSet
Id: valid-valueset-profile
Title: "Valid ValueSet Profile"

// ✅ Valid: CodeSystem parent
Profile: ValidCodeSystemProfile
Parent: CodeSystem
Id: valid-codesystem-profile
Title: "Valid CodeSystem Profile"
