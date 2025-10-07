// Test file for AddElementRule parsing in Logical models

// Simple Logical model with elements
Logical: PatientModel
Id: patient-model
Title: "Patient Logical Model"
Description: "A simple patient data model"
* identifier 0..* Identifier "Business identifier"
* status 1..1 code "Status"
* active 1..1 boolean "Active flag"
* name 1..* HumanName "Name"

// Logical model with flags
Logical: ObservationModel
Id: observation-model
Title: "Observation Logical Model"
* identifier 0..* Identifier MS "Business ID" "Unique business identifier"
* status 1..1 code MS "Status" "Current status"
* code 1..1 CodeableConcept MS SU "Code" "What was observed"
* value 0..1 Quantity TU "Value" "Actual result"

// Logical model with multiple types (or)
Logical: MedicationModel
Id: medication-model
Title: "Medication Logical Model"
* medication 1..1 CodeableConcept or Reference "Medication" "What medication"
* dosage 0..* Dosage "Dosage instructions"
* effective 0..1 dateTime or Period "Effective time"

// Logical model with full definition strings
Logical: PatientDetailedModel
Id: patient-detailed-model
Title: "Patient Detailed Model"
Description: "Detailed patient model with full descriptions"
* identifier 0..* Identifier "Business ID" "Unique business identifier for the patient across systems"
* name 1..* HumanName "Name" "A name associated with the patient, including given names, family name, and prefixes"
* telecom 0..* ContactPoint "Contact details" "Contact details for the patient including phone, email, and other communication methods"

// Custom Resource with elements
Resource: CustomPatient
Id: custom-patient
Title: "Custom Patient Resource"
Description: "Custom patient resource definition"
* id 0..1 id "Logical id"
* meta 0..1 Meta "Metadata"
* identifier 0..* Identifier MS "Business identifier"
* active 0..1 boolean "Whether record is active"
* name 0..* HumanName MS "Patient name"

// Mixed with standard rules
Logical: MixedModel
Id: mixed-model
Title: "Mixed Rules Model"
Description: "Model with both add elements and constraints"
* identifier 0..* Identifier "Business ID"
* status 1..1 code "Status"
* ^status = #draft
* category 0..* CodeableConcept "Category"

// Nested/complex structure
Logical: AddressModel
Id: address-model
Title: "Address Model"
* use 0..1 code "home | work | temp | old"
* type 0..1 code "postal | physical | both"
* text 0..1 string "Text representation"
* line 0..* string MS "Street address lines"
* city 0..1 string MS "City"
* state 0..1 string "State or province"
* postalCode 0..1 string MS "Postal code"
* country 0..1 string "Country"
