// Test file for Logical Characteristics (Task #008)
// Tests comma-separated characteristic code values

// Simple characteristics
Logical: SimpleModel
Id: simple-model
Title: "Simple Model"
Characteristics: #can-bind
* field1 0..1 string "Field 1"

// Multiple characteristics
Logical: ComplexModel
Id: complex-model
Title: "Complex Model"
Description: "A complex logical model with multiple characteristics"
Characteristics: #can-bind, #has-range, #is-logical
* identifier 0..* Identifier "Business identifier"
* status 1..1 code "Status"

// Real-world FHIR characteristics example
Logical: ObservationDataModel
Id: observation-data-model
Title: "Observation Data Model"
Description: "Logical model for observation data"
Characteristics: #can-bind, #has-units, #has-range
* code 1..1 CodeableConcept "What was observed"
* value[x] 0..1 Quantity or CodeableConcept or string "Actual result"
* effectiveDateTime 0..1 dateTime "Clinically relevant time"
* status 1..1 code "Status of result"
