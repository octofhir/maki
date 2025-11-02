Profile: MyPatient
Parent: Patient
Id: my-patient
Title: "My Patient Profile"
Description: "An example patient profile for examples/test-build-ig"
* ^version = "0.1.0"
* ^status = #draft

* identifier 1..* MS
* identifier ^short = "Patient identifier"
* identifier ^definition = "A unique identifier for this patient"

* name 1..* MS
* name ^short = "Patient name"
* name ^definition = "The name(s) of the patient"

* birthDate 0..1 MS
* birthDate ^short = "Date of birth"
