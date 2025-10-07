// Test file to see error colors
Profile: TestProfileBad
Parent: Patient
// Missing Id - should trigger error

Profile: DuplicateProfile
Parent: Patient
Id: duplicate-id
Title: "Duplicate"

Profile: DuplicateProfile
Parent: Observation
Id: duplicate-id-2
Title: "Duplicate Name"
