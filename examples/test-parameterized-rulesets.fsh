// Test file for Parameterized RuleSets (Task #001)
// Tests parameter definitions and insert with arguments

// Simple parameterized RuleSet with one parameter
RuleSet: SimpleRule(value)
* ^version = "1.0.0"
* ^status = #active

// Parameterized RuleSet with multiple parameters
RuleSet: MetadataRule(version, status, experimental)
* ^version = {version}
* ^status = {status}
* ^experimental = {experimental}

// Parameterized RuleSet for common element patterns
RuleSet: RequiredElement(min, max, type)
* {element} {min}..{max} {type} MS

// Using parameterized RuleSet with string argument
Profile: TestProfile1
Parent: Patient
Title: "Test Profile 1"
* insert SimpleRule("1.0.0")

// Using parameterized RuleSet with multiple arguments
Profile: TestProfile2
Parent: Patient
Title: "Test Profile 2"
* insert MetadataRule(5.0.0, #active, false)

// Using parameterized RuleSet with different argument types
Profile: TestProfile3
Parent: Observation
Title: "Test Profile 3"
* insert MetadataRule("2.0.0", #draft, true)

// RuleSet without parameters (for comparison)
RuleSet: NoParamsRule
* ^experimental = false
* ^publisher = "Test Publisher"

Profile: TestProfile4
Parent: Patient
Title: "Test Profile 4"
* insert NoParamsRule

// Empty parameter list
RuleSet: EmptyParams()
* ^version = "1.0.0"

Profile: TestProfile5
Parent: Patient
Title: "Test Profile 5"
* insert EmptyParams()
