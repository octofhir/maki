// Test file for CodeSystem concept parsing

// Simple concepts
CodeSystem: SimpleCS
Id: simple-cs
Title: "Simple Code System"
Description: "Test code system with simple concepts"
* #active "Active" "The resource is currently active"
* #inactive "Inactive" "The resource is not active"
* #pending "Pending"

// Hierarchical concepts (child concepts)
CodeSystem: HierarchicalCS
Id: hierarchical-cs
Title: "Hierarchical Code System"
Description: "Code system with hierarchical concepts"
* #active "Active"
* #active #suspended "Suspended" "Child of active, temporarily inactive"
* #active #on-hold "On Hold" "Child of active, paused"
* #inactive "Inactive"
* #inactive #deleted "Deleted" "Child of inactive, permanently removed"

// Concepts with caret rules
CodeSystem: ConceptsWithMetadata
Id: concepts-metadata
Title: "Concepts with Metadata"
^status = #draft
* #code1 "Code One"
* #code2 "Code Two"
* #code3 "Code Three"

// Real-world example
CodeSystem: ObservationStatus
Id: observation-status
Title: "ObservationStatus"
Description: "Codes providing the status of an observation."
* ^version = "5.0.0"
* ^status = #active
* ^experimental = false
* #registered "Registered" "The existence of the observation is registered, but there is no result yet available."
* #preliminary "Preliminary" "This is an initial or interim observation: data may be incomplete or unverified."
* #final "Final" "The observation is complete and there are no further actions needed."
* #amended "Amended" "Subsequent to being Final, the observation has been modified."
* #amended #corrected "Corrected" "Child of amended - the observation has been corrected."
* #cancelled "Cancelled" "The observation is unavailable because the measurement was not started or not completed."

// Minimal concepts (no display or definition)
CodeSystem: MinimalCS
Id: minimal-cs
Title: "Minimal Code System"
* #code1
* #code2
* #code3
