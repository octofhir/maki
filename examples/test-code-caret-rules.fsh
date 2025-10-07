// Test Code Caret Rules - Task #014
CodeSystem: TestCodeSystem
Id: test-code-system
Title: "Test Code System"
Description: "Test code system for code caret rules"

* #code1 "First code"
* #code1 ^property[0].code = #status
* #code1 ^property[0].valueCode = #active
* #code1 ^designation[0].use = http://snomed.info/sct#900000000000013009
* #code1 ^designation[0].value = "Code 1"

* #code2 "Second code"
* #code2 ^property[0].code = #deprecated
* #code2 ^property[0].valueDateTime = "2023-01-01"
