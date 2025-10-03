// Good example: Well-defined ValueSet
ValueSet: USCoreVitalSigns_VS
Id: us-core-vital-signs
Title: "US Core Vital Signs"
Description: "This value set indicates the allowed vital sign result types."
* ^status = #active
* ^version = "5.0.0"
* ^experimental = false
* ^publisher = "HL7 International - Cross-Group Projects"

* include codes from system http://loinc.org where concept is-a #8310-5 "Body temperature"
* include codes from system http://loinc.org where concept is-a #8302-2 "Body height"
* include codes from system http://loinc.org where concept is-a #8306-3 "Body weight"
* include codes from system http://loinc.org where concept is-a #8867-4 "Heart rate"
* include codes from system http://loinc.org where concept is-a #8480-6 "Systolic blood pressure"

// Bad example: Problems with ValueSet definition
ValueSet: ProblematicVitalSigns_VS
// ERROR: Missing Id
// WARNING: Missing Title
Parent: USCoreVitalSigns_VS
// ERROR: ValueSets don't support Parent keyword

// ERROR: Invalid include syntax
* include all from system http://loinc.org

// WARNING: Duplicate include statements
* include codes from system http://loinc.org where concept is-a #8310-5
* include codes from system http://loinc.org where concept is-a #8310-5

// WARNING: Empty value set with no actual content
ValueSet: EmptyValueSet_VS
Id: empty-vs
Title: "Empty Value Set"
Description: "This value set has no content"
* ^status = #draft
// WARNING: No include/exclude rules - value set is empty

// Good example: ValueSet with proper composition
ValueSet: COVID19TestCodes_VS
Id: covid-19-test-codes
Title: "COVID 19 Test Codes"
Description: "Value set for COVID-19 laboratory test codes"
* ^status = #active
* ^experimental = false

* include codes from system http://loinc.org where concept is-a #94500-6
* exclude http://loinc.org#94563-4 "SARS-CoV-2 (COVID-19) IgG Ab [Presence] in Serum or Plasma by Immunoassay"

// ERROR: Invalid URL format
ValueSet: BadURL_VS
Id: bad-url
Title: "Bad URL Value Set"
* include codes from system not-a-valid-url
* include codes from system http://example..com/double-dot

// WARNING: Mixing multiple code systems without proper organization
ValueSet: MixedSystems_VS
Id: mixed-systems
Title: "Mixed Systems Value Set"
* include codes from system http://loinc.org
* include codes from system http://snomed.info/sct
* include codes from system http://hl7.org/fhir/sid/icd-10
// INFO: Consider documenting why multiple code systems are needed
