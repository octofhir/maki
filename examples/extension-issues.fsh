// Good example: Properly defined extension
Extension: USCoreRaceExtension
Id: us-core-race
Title: "US Core Race Extension"
Description: "Concepts classifying the person into a named category of humans sharing common history, traits, geographical origin or nationality."
* ^version = "5.0.0"
* ^status = #active
* ^date = "2019-05-21"
* ^publisher = "HL7 US Realm Steering Committee"
* ^context[+].type = #element
* ^context[=].expression = "Patient"

* extension contains
    ombCategory 0..5 MS and
    detailed 0..* and
    text 1..1 MS

* extension[ombCategory].value[x] only Coding
* extension[ombCategory].valueCoding from USCoreOMBRaceCodes (required)
* extension[detailed].value[x] only Coding
* extension[detailed].valueCoding from DetailedRaceCodes (required)
* extension[text].value[x] only string

// Bad example: Extension with problems
Extension: ProblematicExtension
// ERROR: Missing Id
// ERROR: Missing Title
// WARNING: Missing context - where can this be used?

// ERROR: Extension has both value[x] and sub-extensions (not allowed)
* value[x] only string
* extension contains subext 0..1

// WARNING: Sub-extension without proper constraints
* extension[subext].value[x] only BackboneElement

// ERROR: Invalid cardinality on value[x]
* value[x] 2..*

// Extension with conflicting constraints
Extension: ConflictingExtension
Id: conflicting-ext
Title: "Conflicting Extension"
* ^context[+].type = #element
* ^context[=].expression = "Patient"
* ^context[+].type = #element
* ^context[=].expression = "Observation"

// ERROR: value[x] constrained to incompatible types
* value[x] only string
* value[x] only integer

// Extension without required context
Extension: NoContextExtension
Id: no-context
Title: "Extension Without Context"
Description: "This extension doesn't specify where it can be used"
// ERROR: Missing ^context - extension must specify context
* value[x] only boolean
