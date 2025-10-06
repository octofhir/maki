---
title: Cardinality Rules
description: Rules for element cardinality validation
---

## Overview

Cardinality rules validate that element cardinality constraints are valid and consistent.

## Rules

### `correctness/invalid-cardinality`

**Severity**: Error
**Fixable**: Yes (safe)

Cardinality must be specified as `min..max` where `min <= max`.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
* name 1..1
* telecom 0..*

// ✗ Bad
Profile: PatientProfile
* name 2..1  // min > max
```

### `correctness/cardinality-conflict`

**Severity**: Error
**Fixable**: No

Multiple cardinality rules on the same element must be compatible.

**Example**:

```fsh
// ✗ Bad - Conflicting cardinality
Profile: PatientProfile
* name 1..1
* name 0..*  // Conflicts with previous rule
```

### `suspicious/redundant-cardinality`

**Severity**: Info
**Fixable**: Yes (safe)

Cardinality that matches the parent is redundant.

**Example**:

```fsh
// ⚠ Redundant
Profile: PatientProfile
Parent: Patient
* identifier 0..*  // Same as base Patient.identifier
```

## Rationale

Valid cardinality ensures:
- FHIR validation works correctly
- Clear data requirements
- Prevents impossible constraints
