// Test data for semantic rule validation

// Valid semantic examples
Profile: ValidSemanticProfile
Parent: Patient
Id: valid-semantic-profile
Title: "Valid Semantic Profile"
Description: "A valid profile with proper semantics"
* name 1..* MS
* name.given 0..5
* identifier 0..* MS
* identifier ^slicing.discriminator.type = #value
* identifier ^slicing.discriminator.path = "system"
* identifier ^slicing.rules = #open
* obeys valid-constraint

Invariant: valid-constraint
Description: "A valid constraint"
Expression: "name.exists()"
Severity: #error

// Invalid cardinality examples (should trigger invalid-cardinality rule)
Profile: InvalidCardinalityProfile
Parent: Patient
* name 5..2  // Min greater than max
* identifier -1..5  // Negative minimum
* telecom 1..abc  // Invalid characters
* address 1...2  // Triple dots
* contact 1..*.*  // Invalid format

// Invalid slicing examples (should trigger invalid-slicing rule)
Profile: InvalidSlicingProfile
Parent: Patient
* identifier ^slicing.rules = #open  // Missing discriminator
* telecom ^slicing.discriminator.type = #invalid  // Invalid discriminator type
* address ^slicing.discriminator.type = #value  // Missing discriminator path
* contact ^slicing.ordered = true  // Missing rules

// Duplicate canonical URL examples (should trigger duplicate-canonical-url rule)
Profile: DuplicateUrlProfile1
Parent: Patient
^url = "http://example.com/duplicate"

Profile: DuplicateUrlProfile2
Parent: Observation
^url = "http://example.com/duplicate"  // Same URL as above

// Duplicate identifier examples (should trigger duplicate-identifier rule)
Profile: DuplicateIdProfile
Parent: Patient

Profile: DuplicateIdProfile  // Same ID as above
Parent: Observation

Extension: DuplicateIdExtension
Id: duplicate-extension

Extension: DuplicateIdExtension  // Same ID as above
Id: duplicate-extension-2

// Invalid constraint examples (should trigger invalid-constraint rule)
Profile: InvalidConstraintProfile
Parent: Patient
* obeys missing-expression  // Missing constraint expression
* obeys invalid-fhirpath

Invariant: missing-expression
Description: "Missing expression"
// No Expression field

Invariant: invalid-fhirpath
Description: "Invalid FHIRPath"
Expression: "name.exists( and telecom"  // Unmatched parentheses
Severity: #error

Invariant: empty-expression
Description: "Empty expression"
Expression: ""  // Empty expression
Severity: #error

// Missing parent profile examples (should trigger missing-parent-profile rule)
Profile: MissingParentProfile
// No Parent declaration
Id: missing-parent-profile
Title: "Profile Without Parent"
Description: "This profile is missing a parent declaration"
* name 1..1

Profile: AnotherMissingParentProfile
Id: another-missing-parent
// Also missing Parent declaration
* identifier 0..*