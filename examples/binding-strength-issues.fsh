// Demonstrating binding strength issues

// Good example: Proper binding with strength
Profile: ProperBindingProfile
Parent: Observation
Id: proper-binding-profile
Title: "Proper Binding Profile"
Description: "Profile demonstrating correct binding strength usage"

* code from VitalSignsCodes (required)
* category from ObservationCategoryCodes (extensible)
* interpretation from ObservationInterpretationCodes (preferred)
* method from ObservationMethodCodes (example)

// Bad example: Missing binding strength
Profile: MissingStrengthProfile
Parent: Observation
Id: missing-strength-profile
Title: "Missing Strength Profile"

// ERROR: Missing binding strength
* code from VitalSignsCodes
* category from ObservationCategoryCodes
// ERROR: Binding without strength specification
* interpretation from ObservationInterpretationCodes

// Good example: Extension with proper binding
Extension: RaceExtension
Id: race-extension
Title: "Race Extension"
* ^context[+].type = #element
* ^context[=].expression = "Patient"

* extension contains
    category 0..* and
    text 1..1

* extension[category].value[x] only Coding
// Proper binding with strength
* extension[category].valueCoding from RaceCodes (required)
* extension[text].value[x] only string

// Bad example: Extension with binding issues
Extension: ProblematicBindingExtension
Id: problematic-binding-extension
Title: "Problematic Binding Extension"
* ^context[+].type = #element
* ^context[=].expression = "Observation"

* extension contains
    code 0..1

* extension[code].value[x] only Coding
// ERROR: Missing binding strength
* extension[code].valueCoding from SomeValueSet

// ERROR: Invalid binding strength value
* value[x] only CodeableConcept
* valueCodeableConcept from AnotherValueSet (very-strict)

// Good example: Multiple bindings with appropriate strengths
Profile: MultiBindingProfile
Parent: Condition
Id: multi-binding-profile
Title: "Multi Binding Profile"

// Required: Must use codes from this value set
* code from ConditionCodes (required)

// Extensible: Should use codes from this value set, but can use others if needed
* category from ConditionCategoryCodes (extensible)

// Preferred: Codes from this value set are preferred but not required
* severity from ConditionSeverityCodes (preferred)

// Example: These codes are just examples, use any appropriate codes
* bodySite from BodySiteCodes (example)
