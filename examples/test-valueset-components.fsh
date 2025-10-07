// Test file for ValueSet component parsing

// Simple concept include
ValueSet: SimpleVS
Id: simple-vs
Title: "Simple ValueSet"
* include http://loinc.org#12345-6 "Blood pressure"

// Include all codes from system
ValueSet: AllLoincVS
Id: all-loinc-vs
Title: "All LOINC Codes"
* include codes from system http://loinc.org

// Include with filter
ValueSet: FilteredVS
Id: filtered-vs
Title: "Filtered ValueSet"
* include codes from system http://loinc.org where concept is-a #8310-5

// Multiple filters
ValueSet: ComplexVS
Id: complex-vs
Title: "Complex ValueSet with Multiple Filters"
* include codes from system http://snomed.info/sct where concept descendent-of #12345 and concept regex /^(A|B)/

// System and valueset
ValueSet: MixedVS
Id: mixed-vs
Title: "Mixed Sources ValueSet"
* include codes from system http://loinc.org and valueset http://example.org/vs/vital-signs

// Exclude example
ValueSet: ExcludeVS
Id: exclude-vs
Title: "ValueSet with Exclusions"
* include codes from system http://loinc.org
* exclude http://loinc.org#12345-6

// Real-world US Core example
ValueSet: USCoreVitalSigns
Id: us-core-vital-signs
Title: "US Core Vital Signs"
Description: "This value set indicates the allowed vital sign result types."
* ^status = #active
* include codes from system http://loinc.org where concept is-a #8310-5
* include codes from system http://loinc.org where concept is-a #8302-2
* include codes from system http://loinc.org where concept is-a #8306-3

// Implicit include (no include keyword)
ValueSet: ImplicitInclude
Id: implicit-include
Title: "Implicit Include"
* http://snomed.info/sct#12345
* codes from system http://loinc.org
