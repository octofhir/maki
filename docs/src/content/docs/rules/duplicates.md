---
title: Duplicate Detection Rules
description: Rules for detecting duplicate definitions
---

## Overview

Duplicate detection rules identify duplicate or conflicting resource definitions.

## Rules

### `correctness/duplicate-definition`

**Severity**: Error
**Fixable**: Yes (suggests renaming)

Resource names must be unique within the project.

**Example**:

```fsh
// ✗ Bad - Duplicate profile names
Profile: PatientProfile
Parent: Patient
* name 1..1

Profile: PatientProfile  // Duplicate!
Parent: Patient
* gender 1..1
```

**Fix**: Rename one of the profiles:

```fsh
Profile: PatientProfile
Parent: Patient
* name 1..1

Profile: PatientDemographicsProfile
Parent: Patient
* gender 1..1
```

### `correctness/duplicate-id`

**Severity**: Error
**Fixable**: No

Resource IDs must be unique across all resources.

**Example**:

```fsh
// ✗ Bad
Profile: PatientProfile
Id: patient-profile

Extension: PatientExtension
Id: patient-profile  // Duplicate ID!
```

### `suspicious/duplicate-alias`

**Severity**: Warning
**Fixable**: Yes (safe)

Alias definitions should not be duplicated.

**Example**:

```fsh
// ✗ Bad
Alias: $SCT = http://snomed.info/sct
Alias: $SCT = http://snomed.info/sct  // Duplicate
```

## Rationale

Preventing duplicates ensures:
- Clear resource identification
- Prevents compilation errors
- Avoids ambiguity in references
