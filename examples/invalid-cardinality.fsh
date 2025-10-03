// Bad example: Invalid cardinality constraints
Profile: ProblematicPatient
Parent: Patient
Id: problematic-patient
Title: "Problematic Patient Profile"
Description: "This profile has several cardinality issues"

// ERROR: Upper bound cannot be less than lower bound
* identifier 1..0

// WARNING: Redundant cardinality (same as parent)
* gender 0..1

// ERROR: Invalid cardinality syntax
* name 1..*..2

// WARNING: Narrowing cardinality might break conformance
* telecom 0..0

// ERROR: Missing cardinality for must-support element
* address MS

// ERROR: Cardinality conflicts with parent constraint
* birthDate 2..*

// WARNING: Upper bound exceeds parent's cardinality
* contact 0..100

// ERROR: Non-numeric cardinality
* maritalStatus one..many

// ERROR: Cardinality on extension without proper definition
* extension[unknownExt] 1..1
