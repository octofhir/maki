// Test data for best practice rule validation

// Valid best practice examples
Profile: ValidBestPracticeProfile
Parent: Patient
Id: valid-best-practice-profile
Title: "Valid Best Practice Profile"
Description: "A profile following all best practices"
^publisher = "Example Organization"
^status = #draft
^version = "1.0.0"
* name 1..* MS

Extension: ValidBestPracticeExtension
Id: valid-best-practice-extension
Title: "Valid Best Practice Extension"
Description: "An extension following all best practices"
^publisher = "Example Organization"
^status = #active
^version = "1.0.0"
* value[x] only string

// Profile naming convention violations (should trigger profile-naming-convention rule)
Profile: lowercase_profile  // Should be PascalCase
Parent: Patient

Profile: ALLUPPERCASE  // Should be PascalCase
Parent: Patient

Profile: mixedCase_with_underscores  // Should be PascalCase
Parent: Patient

Profile: ProfileWithNumbers123  // Should not end with numbers
Parent: Patient

Profile: Profile_With_Underscores  // Should not use underscores
Parent: Patient

// Missing description examples (should trigger missing-description rule)
Profile: MissingDescriptionProfile
Parent: Patient
Id: missing-description-profile
Title: "Profile Without Description"
// No Description field
^publisher = "Example Organization"
^status = #draft

Extension: MissingDescriptionExtension
Id: missing-description-extension
Title: "Extension Without Description"
// No Description field
* value[x] only string

ValueSet: MissingDescriptionValueSet
Id: missing-description-valueset
Title: "ValueSet Without Description"
// No Description field

// Missing title examples (should trigger missing-title rule)
Profile: MissingTitleProfile
Parent: Patient
Id: missing-title-profile
Description: "A profile without a title"
// No Title field
^publisher = "Example Organization"

Extension: MissingTitleExtension
Id: missing-title-extension
Description: "An extension without a title"
// No Title field
* value[x] only string

// Inconsistent metadata examples (should trigger inconsistent-metadata rule)
Profile: InconsistentProfile1
Parent: Patient
Id: inconsistent-profile-1
Title: "Inconsistent Profile 1"
Description: "First profile with inconsistent metadata"
^publisher = "Organization A"
^version = "1.0.0"

Profile: InconsistentProfile2
Parent: Observation
Id: inconsistent-profile-2
Title: "Inconsistent Profile 2"
Description: "Second profile with different metadata"
^publisher = "Organization B"  // Different publisher
^version = "2.0.0"  // Different version

// Missing publisher examples (should trigger missing-publisher rule)
Profile: MissingPublisherProfile
Parent: Patient
Id: missing-publisher-profile
Title: "Profile Without Publisher"
Description: "A profile missing publisher information"
// No Publisher field
^status = #draft

Extension: MissingPublisherExtension
Id: missing-publisher-extension
Title: "Extension Without Publisher"
Description: "An extension missing publisher information"
// No Publisher field
* value[x] only string

// Invalid status examples (should trigger invalid-status rule)
Profile: InvalidStatusProfile1
Parent: Patient
Id: invalid-status-profile-1
Title: "Profile With Invalid Status"
Description: "A profile with an invalid status value"
^status = #invalid  // Invalid status value

Profile: InvalidStatusProfile2
Parent: Patient
Id: invalid-status-profile-2
Title: "Another Profile With Invalid Status"
Description: "Another profile with an invalid status value"
^status = #published  // Invalid status value (should be #active)

Extension: InvalidStatusExtension
Id: invalid-status-extension
Title: "Extension With Invalid Status"
Description: "An extension with an invalid status value"
^status = #experimental  // Invalid status value
* value[x] only string