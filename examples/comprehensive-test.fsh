// Comprehensive test file for all FSH lint rules

// ❌ Profile without Parent - should warn
Profile: ProfileWithoutParent
Id: profile-no-parent
Title: "Profile Without Parent"
Description: "This profile has no parent declaration"

// ❌ Profile without Id and Title - should error
Profile: ProfileMissingFields
Parent: Patient

// ❌ Profile with invalid cardinality - should error
Profile: ProfileBadCardinality
Parent: Patient
Id: bad-cardinality
Title: "Bad Cardinality"
* name 5..2

// ❌ Profile without binding strength - should error
Profile: ProfileMissingBinding
Parent: Patient
Id: missing-binding
Title: "Missing Binding Strength"
* gender from GenderValueSet

// ❌ Profile without ^status - should info
Profile: ProfileNoStatus
Parent: Patient
Id: no-status
Title: "No Status"
Description: "Missing status assignment"

// ✅ Good profile with everything
Profile: GoodProfile
Parent: Patient
Id: good-profile
Title: "Good Profile"
Description: "Has all required fields"
* ^status = #draft
* name 0..1 MS

// ❌ Extension without context - should error
Extension: ExtensionNoContext
Id: ext-no-context
Title: "Extension Without Context"
Description: "Missing context specification"

// ✅ Good extension with context
Extension: GoodExtension
Id: good-extension
Title: "Good Extension"
Description: "Has context"
Context: Patient

// ❌ Duplicate profiles - should error twice
Profile: DuplicateTest
Parent: Patient
Id: dup-1
Title: "First"

Profile: DuplicateTest
Parent: Observation
Id: dup-2
Title: "Second Duplicate"

// ❌ Duplicate IDs - should error
ValueSet: ValueSet1
Id: duplicate-id
Title: "First ValueSet"

CodeSystem: CodeSystem1
Id: duplicate-id
Title: "CodeSystem with Duplicate ID"
