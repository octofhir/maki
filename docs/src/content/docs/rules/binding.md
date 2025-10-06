---
title: Binding Rules
description: Rules for terminology binding validation
---

## Overview

Binding rules validate that coded elements are properly bound to value sets or code systems.

## Rules

### `correctness/binding-strength`

**Severity**: Warning
**Fixable**: No

Binding strength should be appropriate for the use case.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
* gender from AdministrativeGenderVS (required)
* maritalStatus from MaritalStatusVS (extensible)

// ⚠ Warning - Consider if 'example' is appropriate
* language from LanguageVS (example)
```

**Binding Strengths**:
- `required` - Must use a code from the value set
- `extensible` - Should use from value set, can use others if needed
- `preferred` - Recommended to use from value set
- `example` - For illustration only

### `correctness/invalid-binding-reference`

**Severity**: Error
**Fixable**: No

Bindings must reference existing value sets.

**Example**:

```fsh
// ✗ Bad
Profile: PatientProfile
* gender from NonExistentVS (required)
```

### `suspicious/weak-binding`

**Severity**: Info
**Fixable**: No

Consider using stronger binding for critical coded elements.

**Example**:

```fsh
// ⚠ Consider using 'required' or 'extensible'
Profile: PatientProfile
* gender from AdministrativeGenderVS (example)
```

## Rationale

Proper bindings ensure:
- Consistent terminology use
- Interoperability
- Semantic clarity
- Validation effectiveness
