// Warning example: Missing recommended metadata
Profile: IncompletePatient
Parent: Patient
// WARNING: Missing Id - should have explicit identifier
// WARNING: Missing Title - required for human readability
// WARNING: Missing Description - recommended for documentation

// WARNING: Missing version metadata
// WARNING: Missing status metadata
// WARNING: Missing abstract field
// WARNING: Missing publisher information
// WARNING: Missing contact information

* identifier 1..* MS
* name 1..* MS
* gender 1..1 MS

// INFO: Consider adding more constraints for better conformance
* birthDate MS

// Extension without proper documentation
Extension: UndocumentedExtension
// WARNING: Missing Id
// WARNING: Missing Title
// WARNING: Missing Description
// WARNING: Missing context - where can this be used?
* value[x] only string
