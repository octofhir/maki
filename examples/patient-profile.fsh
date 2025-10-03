// Good example: Well-formed patient profile with proper metadata
Profile: USCorePatientProfile
Parent: Patient
Id: us-core-patient
Title: "US Core Patient Profile"
Description: "Defines constraints and extensions on the Patient resource for the minimal set of data to query and retrieve patient demographic information."
* ^version = "5.0.0"
* ^status = #active
* ^abstract = false
* ^date = "2022-04-20"
* ^publisher = "HL7 International - Cross-Group Projects"
* ^contact.telecom.system = #url
* ^contact.telecom.value = "http://www.hl7.org/Special/committees/cgp"

// Extensions
* extension contains
    USCoreRaceExtension named race 0..1 MS and
    USCoreEthnicityExtension named ethnicity 0..1 MS and
    USCoreBirthSexExtension named birthsex 0..1

// Identifier constraints
* identifier 1..* MS
* identifier.system 1..1 MS
* identifier.value 1..1 MS

// Name constraints
* name 1..* MS
* name.family 1..1 MS
* name.given 1..* MS

// Telecom
* telecom MS
* telecom.system 1..1 MS
* telecom.value 1..1 MS

// Demographics
* gender 1..1 MS
* birthDate MS

// Address
* address MS
* address.line MS
* address.city MS
* address.state MS
* address.postalCode MS
* address.country MS

// Communication
* communication MS
* communication.language MS
* communication.language from AllLanguages (extensible)
