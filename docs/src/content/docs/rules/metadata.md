---
title: Metadata Rules
description: Rules for FHIR resource metadata
---

## Overview

Metadata rules ensure all FHIR resources have proper documentation and identifying information.

## Rules

### `documentation/title-required`

**Severity**: Warning
**Fixable**: No

All profiles, extensions, and value sets must have a title.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
Title: "Patient Profile for Clinical Use"
* name 1..1

// ✗ Bad
Profile: PatientProfile
* name 1..1
```

### `documentation/description-required`

**Severity**: Warning
**Fixable**: No

All profiles, extensions, and value sets must have a description.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
Description: "Profile defining constraints for patient demographics in clinical workflows"
* name 1..1

// ✗ Bad
Profile: PatientProfile
* name 1..1
```

### `documentation/id-format`

**Severity**: Warning
**Fixable**: Yes (safe)

Resource IDs should use kebab-case format.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
Id: patient-profile

// ✗ Bad
Profile: PatientProfile
Id: PatientProfile
```

## Rationale

Proper metadata:
- Improves documentation quality
- Aids in resource discovery
- Required for IG publication
- Helps users understand resource purpose
