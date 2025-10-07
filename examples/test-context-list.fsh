// Test file for Extension Context List (Task #007)
// Tests comma-separated context values

// Simple context list
Extension: PatientExtension
Id: patient-extension
Context: Patient

// Multiple contexts
Extension: ObservationExtension
Id: observation-extension
Context: Observation, DiagnosticReport, ServiceRequest
* value[x] only string

// More complex example with multiple contexts
Extension: ClinicalImpressionExtension
Id: clinical-impression-extension
Title: "Clinical Impression Extension"
Description: "Extension for clinical impressions"
Context: ClinicalImpression, Observation, Condition, Procedure
* value[x] only CodeableConcept

// Mix of contexts (resources and elements)
Extension: MetadataExtension
Id: metadata-extension
Context: Patient, Observation.status, Encounter.serviceType
* value[x] only string or date
