// Example demonstrating naming convention issues

// ERROR: Profile name doesn't match Id
Profile: MyPatientProfile
Parent: Patient
Id: some-random-id  // Should be: my-patient-profile
Title: "Different Title Here"  // Should be: "My Patient Profile"

// ERROR: CodeSystem name doesn't match filename/id/title
CodeSystem: ExampleCodeSystem_CS
Id: wrong_id_format  // Should be: example-code-system
Title: "Wrong Title Format"  // Should be: "Example Code System"

// ERROR: ValueSet name doesn't match conventions
ValueSet: MyValueSet_VS
Id: my-valueset-vs  // Should be: my-valueset (no -vs suffix)
Title: "MyValueSet"  // Should be: "My Value Set"

// GOOD: Proper naming with acronyms
Profile: HIVDiagnosisProfile
Parent: Observation
Id: hiv-diagnosis-profile
Title: "HIV Diagnosis Profile"

// GOOD: Proper naming with numbers
Profile: COVID19TestProfile
Parent: Observation
Id: covid-19-test-profile  // or covid19-test-profile (flexible)
Title: "COVID 19 Test Profile"

// ERROR: Inconsistent casing
Profile: someProfile
Parent: Patient
Id: SomeProfile  // Should be: some-profile
Title: "some profile"  // Should be: "Some Profile"
