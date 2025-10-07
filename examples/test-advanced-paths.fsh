// Test Advanced Path Features - Task #012
Profile: AdvancedPathProfile
Parent: Observation
Id: advanced-path-profile
Title: "Advanced Path Profile"
Description: "Test profile for advanced path features"

// Choice type with [x]
* value[x] only Quantity or string

// Soft indexing with [=]
* component[=].code = http://loinc.org#1234-5

// Keywords in paths (if supported)
* extension[type].value[x] only string

// Complex path with multiple brackets
* component[0].value[x] only Quantity
* component[1].value[x] only string

Instance: TestObservation
InstanceOf: Observation
Usage: #example
* status = #final
* code = http://loinc.org#1234-5
* value[x] = 5.4 'mg'
* component[0].code = http://loinc.org#5678-9
* component[0].valueQuantity = 120 'mm[Hg]'
