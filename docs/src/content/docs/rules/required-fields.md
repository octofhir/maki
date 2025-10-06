---
title: Required Fields Rules
description: Rules for required field validation
---

## Overview

Required fields rules ensure that essential fields are present in resource definitions.

## Rules

### `correctness/profile-required-fields`

**Severity**: Warning
**Fixable**: No

Profiles should define constraints on required fields.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
* name 1..1 MS
* gender 1..1 MS
* birthDate 1..1 MS
```

### `correctness/valueset-required-fields`

**Severity**: Warning
**Fixable**: No

ValueSets must have title and description.

**Example**:

```fsh
// ✓ Good
ValueSet: AdministrativeGenderVS
Title: "Administrative Gender Value Set"
Description: "Codes for administrative gender"
* include codes from system http://hl7.org/fhir/administrative-gender

// ✗ Bad
ValueSet: AdministrativeGenderVS
* include codes from system http://hl7.org/fhir/administrative-gender
```

### `correctness/codesystem-required-fields`

**Severity**: Warning
**Fixable**: No

CodeSystems must have title, description, and content type.

**Example**:

```fsh
// ✓ Good
CodeSystem: ContactPointUseCS
Title: "Contact Point Use"
Description: "Codes for contact point usage"
* ^content = #complete
```

## Rationale

Required fields ensure:
- Complete resource definitions
- Proper IG documentation
- FHIR specification compliance
