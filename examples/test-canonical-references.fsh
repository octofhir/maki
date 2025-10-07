// Test Canonical and Reference - Task #011
Profile: CanonicalTestProfile
Parent: StructureDefinition
Id: canonical-test-profile
Title: "Canonical Test Profile"
Description: "Test profile for Canonical and Reference parsing"

// Reference with single type
* subject only Reference(Patient)

// Reference with multiple types
* performer only Reference(Practitioner) or Reference(Organization)

// Canonical reference
* type only Canonical(StructureDefinition)

// CodeableReference (if supported)
* reason only CodeableReference(Condition)

Extension: TestExtension
Id: test-extension
Title: "Test Extension"
Description: "Test extension with reference"
* value[x] only Reference(Patient) or Reference(Group)
* ^context[0].type = #element
* ^context[0].expression = "Patient"
