// Test file for AST rules validation

// Test 1: Missing required fields (Id and Title)
Profile: TestProfileMissingFields
Parent: Patient
// Missing Id and Title - should trigger required_fields rule

// Test 2: Invalid cardinality (min > max)
Profile: TestProfileBadCardinality
Parent: Patient
Id: "test-profile-card"
Title: "Test Profile with Bad Cardinality"
* name 5..2  // Should trigger invalid-cardinality rule

// Test 3: Missing binding strength
Profile: TestProfileMissingBindingStrength
Parent: Patient
Id: "test-profile-binding"
Title: "Test Profile Missing Binding Strength"
* gender from GenderValueSet  // Should trigger binding-strength-present rule

// Test 4: Missing metadata (description)
Profile: TestProfileMissingDescription
Parent: Patient
Id: "test-profile-desc"
Title: "Test Profile Missing Description"
// Missing Description - should trigger missing-metadata rule (warning)

// Test 5: All rules passing
Profile: TestProfileAllGood
Parent: Patient
Id: "test-profile-good"
Title: "Test Profile All Good"
Description: "A properly defined profile with all required fields"
* gender from GenderValueSet (required)
* name 0..1
