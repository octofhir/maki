// Test data for syntax rule validation

// Valid FSH examples
Profile: ValidPatientProfile
Parent: Patient
Id: valid-patient-profile
Title: "Valid Patient Profile"
Description: "A valid patient profile example"
* name 1..* MS

Extension: ValidExtension
Id: valid-extension
Title: "Valid Extension"
Description: "A valid extension example"
* value[x] only string

ValueSet: ValidValueSet
Id: valid-valueset
Title: "Valid Value Set"
Description: "A valid value set example"

// Invalid keyword examples (should trigger invalid-keyword rule)
Profil: InvalidKeywordProfile  // Misspelled "Profile"
Parent: Patient

Extensio: InvalidKeywordExtension  // Misspelled "Extension"
Id: invalid-extension

ValueSe: InvalidKeywordValueSet  // Misspelled "ValueSet"

// Malformed alias examples (should trigger malformed-alias rule)
Alias: BadAlias1  // Missing equals sign
Alias: BadAlias2 == "http://example.com"  // Double equals
Alias:  = "http://example.com"  // Missing alias name
Alias: Bad-Alias! = "http://example.com"  // Invalid characters

// Valid alias for comparison
Alias: GoodAlias = "http://example.com/good"

// Invalid caret path examples (should trigger invalid-caret-path rule)
Profile: CaretPathProfile
Parent: Patient
* name..given 1..1  // Double dots
* .name 1..1  // Starting with dot
* name. 1..1  // Ending with dot
* name[] 1..1  // Empty brackets
* name[test 1..1  // Unmatched brackets
* name@invalid 1..1  // Invalid characters

// Trailing text examples (should trigger trailing-text rule)
Profile: TrailingTextProfile extra text here
Parent: Patient more text
* name 1..1 some trailing text
^ title = "Test" with extra content

// Missing profile ID examples (should trigger missing-profile-id rule)
Profile:  // Empty ID
Parent: Patient

Profile:  // Whitespace only ID
Parent: Patient

Profile: 123InvalidId  // Starting with number
Parent: Patient

// Invalid identifier examples (should trigger invalid-identifier rule)
Profile: 1StartsWithNumber
Parent: Patient

Profile: Contains@InvalidChars
Parent: Patient

Profile: true  // Reserved keyword
Parent: Patient

Profile: !  // Single invalid character
Parent: Patient