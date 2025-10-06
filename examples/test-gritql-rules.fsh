// Test file for GritQL rules

// Test 1: Invalid keyword - should match "correctness/invalid-keyword"
Profil: TestInvalidKeyword
Parent: Patient

// Test 2: Another invalid keyword
Extenstion: TestInvalidExtension
Parent: Extension

// Test 3: Valid profile (should not match any GritQL rules)
Profile: ValidProfile
Parent: Patient
Id: "valid-profile"
Description: "A valid profile for testing"
