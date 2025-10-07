// Test Code Insert Rules - Task #015
RuleSet: CommonCodeProperties
* ^property[0].code = #status
* ^property[0].valueCode = #active

CodeSystem: TestCodeSystem2
Id: test-code-system-2
Title: "Test Code System 2"
Description: "Test code system for code insert rules"

* #code1 "First code"
* #code1 insert CommonCodeProperties

* #code2 "Second code"
* #code2 insert CommonCodeProperties
