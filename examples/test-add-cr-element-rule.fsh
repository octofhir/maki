// Test AddCRElementRule - Task #003
Logical: TestModel
Id: test-model
Title: "Test Logical Model"
Description: "Test model for contentreference"

// AddCRElementRule with contentreference
* element1 0..1 MS contentreference http://example.org/StructureDefinition/MyType "Short description"
* element2 1..1 contentreference http://example.org/StructureDefinition/OtherType "Short" "Full definition"
* element3 0..* contentreference #MyLocalType "Reference to local type"
