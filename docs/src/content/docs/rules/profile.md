---
title: Profile Rules
description: Rules for FHIR profile validation
---

## Overview

Profile rules ensure profiles are correctly defined with proper parent relationships and constraints.

## Rules

### `correctness/profile-parent-required`

**Severity**: Error
**Fixable**: No

All profiles must specify a parent resource.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
Parent: Patient
* name 1..1

// ✗ Bad
Profile: PatientProfile
* name 1..1
```

### `correctness/profile-constraint-strength`

**Severity**: Warning
**Fixable**: No

Profile constraints should not weaken parent constraints.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
Parent: Patient
// Patient.name is 0..* in base, making it 1..1 is strengthening
* name 1..1

// ✗ Bad
Profile: StrictPatientProfile
Parent: PatientProfile  // PatientProfile has name 1..1
// Cannot weaken to 0..1
* name 0..1
```

### `suspicious/profile-circular-parent`

**Severity**: Error
**Fixable**: No

Profiles cannot have circular parent relationships.

**Example**:

```fsh
// ✗ Bad - Circular dependency
Profile: ProfileA
Parent: ProfileB

Profile: ProfileB
Parent: ProfileA
```

## Rationale

Proper profile validation ensures:
- FHIR specification compliance
- Valid constraint hierarchies
- Prevents runtime validation errors
