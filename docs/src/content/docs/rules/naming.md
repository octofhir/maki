---
title: Naming Convention Rules
description: Rules for consistent FSH resource naming
---

## Overview

Naming convention rules ensure consistent, predictable naming patterns across your FSH project.

## Rules

### `style/profile-naming`

**Severity**: Warning
**Fixable**: Yes (safe)

Profile names should follow PascalCase and match their resource type.

**Example**:

```fsh
// ✓ Good
Profile: PatientProfile
Parent: Patient

// ✗ Bad
Profile: patient_profile
Parent: Patient
```

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "style": {
        "profile-naming": "warn"
      }
    }
  }
}
```

### `style/valueset-naming`

**Severity**: Warning
**Fixable**: Yes (safe)

ValueSet names should use PascalCase and be suffixed with "VS" or "ValueSet".

**Example**:

```fsh
// ✓ Good
ValueSet: AdministrativeGenderVS

// ✗ Bad
ValueSet: administrativeGender
```

### `style/codesystem-naming`

**Severity**: Warning
**Fixable**: Yes (safe)

CodeSystem names should use PascalCase and be suffixed with "CS" or "CodeSystem".

**Example**:

```fsh
// ✓ Good
CodeSystem: ContactPointUseCS

// ✗ Bad
CodeSystem: contact_point_use
```

### `style/extension-naming`

**Severity**: Warning
**Fixable**: Yes (safe)

Extension names should use PascalCase and be suffixed with "Extension".

**Example**:

```fsh
// ✓ Good
Extension: PatientBirthPlaceExtension

// ✗ Bad
Extension: patient-birth-place
```

## Rationale

Consistent naming conventions:
- Improve code readability
- Make resource types immediately identifiable
- Follow FHIR community best practices
- Enable better IDE autocomplete
