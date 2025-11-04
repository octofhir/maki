Profile: EdgeCaseProfile
Parent: Observation
* value = Canonical(MyProfile|1.0.0 or OtherProfile|2.0)
* extension[edge].valueCodeableReference = CodeableReference(Observation or Procedure)
* note.valueString = /^(foo|bar|\s+)$/
* insert ParamTokens([[Bracketed Value]], second value)

CodeSystem: EdgeCaseCS
Id: edge-case-cs
* #collection "Display With Spaces"

RuleSet: ParamTokens([[First Parameter]], plain value, escaped\,comma)
* ^description = "Parameter edge cases"
