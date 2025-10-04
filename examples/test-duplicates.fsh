// Test file for duplicate detection rules

Profile: DuplicateProfile
Parent: Patient
Id: duplicate-id
Title: "First Profile"

Profile: DuplicateProfile
Parent: Observation
Id: unique-id
Title: "Second Profile with Same Name"

ValueSet: UniqueValueSet
Id: duplicate-id
Title: "ValueSet with Duplicate ID"

Extension: ExtensionWithoutContext
Title: "This extension has no context"
Description: "Should trigger extension-context-missing error"

Extension: ExtensionWithContext
Title: "This extension has context"
Description: "Should not trigger error"
Context: Patient

Profile: GoodProfile
Parent: Patient
Id: good-profile
Title: "A Good Profile"
Description: "No issues here"
