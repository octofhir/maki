// Test file for Mapping rule parsing

// Simple mapping with target only
Mapping: PatientToV2
Id: patient-to-v2
Source: Patient
Target: "HL7 V2 PID segment"
Title: "FHIR Patient to V2 PID Mapping"
* name -> "PID-5"
* birthDate -> "PID-7"
* gender -> "PID-8"

// Mapping with comments
Mapping: ObservationToV2
Id: observation-to-v2
Source: Observation
Target: "HL7 V2 OBX segment"
Title: "FHIR Observation to V2 OBX Mapping"
Description: "Maps FHIR Observation to HL7 V2 OBX segment"
* status -> "OBX-11" "Observation result status"
* code -> "OBX-3" "Observation identifier"
* value[x] -> "OBX-5" "Observation value"
* effectiveDateTime -> "OBX-14" "Date/time of the observation"

// Mapping with language code
Mapping: PatientToFHIR
Id: patient-to-fhir
Source: Patient
Target: "http://hl7.org/fhir/StructureDefinition/Patient"
* identifier -> "Patient.identifier" "Business identifier" #en
* name -> "Patient.name" "Patient name" #en-US

// Mapping without path (whole resource mapping)
Mapping: BundleToMessage
Id: bundle-to-message
Source: Bundle
Target: "HL7 V2 Message"
* -> "MSH" "Message header"

// Real-world example
Mapping: USCorePatientToArgonaut
Id: us-core-patient-to-argonaut
Source: USCorePatient
Target: "http://fhir.org/guides/argonaut/StructureDefinition/argo-patient"
Title: "US Core Patient to Argonaut Patient"
Description: "Mapping from US Core Patient to Argonaut Patient profile"
* identifier -> "Patient.identifier"
* name -> "Patient.name"
* telecom -> "Patient.telecom"
* gender -> "Patient.gender" "Administrative gender"
* birthDate -> "Patient.birthDate"
* address -> "Patient.address"
* communication.language -> "Patient.communication.language"
