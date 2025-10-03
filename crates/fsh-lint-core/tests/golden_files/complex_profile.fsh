Profile: ComplexPatient
Parent: Patient
Id: complex-patient
Title: "Complex Patient"
Description: "A complex patient profile"
* name 1..1 MS
* name.family 1..1
* name.given 1..*
* gender 1..1
* birthDate 0..1
* ^status = #active
* ^experimental = false