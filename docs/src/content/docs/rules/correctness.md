---
title: Correctness Rules
description: Rules for FHIR specification compliance
---

## Overview

Correctness rules ensure that your FSH code complies with the FHIR specification and
prevent syntax errors, semantic violations, and runtime issues.

## Rules

### `correctness/required-field-present`

**Name**: Required Field Present
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: AST

Ensures that Profiles, CodeSystems, and ValueSets have required fields (Name, Id, Title)

**Tags**: correctness, blocking, metadata, required-fields

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/required-field-present": "error"
    }
  }
}
```

**Learn more**: [Required Field Present](https://octofhir.github.io/maki-rs/rules/correctness/required-field-present)

---

### `correctness/invalid-cardinality`

**Name**: Invalid Cardinality
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: AST

Detects invalid cardinality expressions such as reversed bounds (1..0), invalid syntax, and non-numeric values

**Tags**: correctness, blocking, cardinality, constraints

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/invalid-cardinality": "error"
    }
  }
}
```

**Learn more**: [Invalid Cardinality](https://octofhir.github.io/maki-rs/rules/correctness/invalid-cardinality)

---

### `correctness/binding-strength-present`

**Name**: Binding Strength Present
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: AST

Ensures that bindings to value sets specify strength (required, extensible, preferred, or example) and use valid strength values

**Tags**: correctness, blocking, binding, terminology

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/binding-strength-present": "error"
    }
  }
}
```

**Learn more**: [Binding Strength Present](https://octofhir.github.io/maki-rs/rules/correctness/binding-strength-present)

---

### `correctness/duplicate-definition`

**Name**: Duplicate Definition
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: AST

Prevents duplicate resource names, IDs, and canonical URLs which would cause conflicts

**Tags**: correctness, blocking, duplicates, conflicts

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/duplicate-definition": "error"
    }
  }
}
```

**Learn more**: [Duplicate Definition](https://octofhir.github.io/maki-rs/rules/correctness/duplicate-definition)

---

### `correctness/invalid-keyword`

**Name**: Invalid Keyword
**Severity**: 🔴 Error
**Fixable**: Yes
**Implementation**: GritQL

Detects invalid or misspelled FSH keywords like 'Profil' instead of 'Profile'

**Tags**: correctness, keywords

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/invalid-keyword": "error"
    }
  }
}
```

**Learn more**: [Invalid Keyword](https://octofhir.github.io/maki-rs/rules/correctness/invalid-keyword)

---

### `correctness/malformed-alias`

**Name**: Malformed Alias
**Severity**: 🔴 Error
**Fixable**: Yes
**Implementation**: GritQL

Detects malformed alias declarations with syntax errors

**Tags**: correctness, alias

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/malformed-alias": "error"
    }
  }
}
```

**Learn more**: [Malformed Alias](https://octofhir.github.io/maki-rs/rules/correctness/malformed-alias)

---

### `correctness/invalid-caret-path`

**Name**: Invalid Caret Path
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: GritQL

Detects invalid caret path syntax in FSH rules

**Tags**: correctness, caret, path

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/invalid-caret-path": "error"
    }
  }
}
```

**Learn more**: [Invalid Caret Path](https://octofhir.github.io/maki-rs/rules/correctness/invalid-caret-path)

---

### `correctness/missing-profile-id`

**Name**: Missing Profile ID
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: GritQL

Detects profile declarations without proper identifiers

**Tags**: correctness, profile, id

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/missing-profile-id": "error"
    }
  }
}
```

**Learn more**: [Missing Profile ID](https://octofhir.github.io/maki-rs/rules/correctness/missing-profile-id)

---

### `correctness/invalid-identifier`

**Name**: Invalid Identifier
**Severity**: 🔴 Error
**Fixable**: Yes
**Implementation**: GritQL

Detects invalid identifier syntax in FSH files

**Tags**: correctness, identifier

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/invalid-identifier": "error"
    }
  }
}
```

**Learn more**: [Invalid Identifier](https://octofhir.github.io/maki-rs/rules/correctness/invalid-identifier)

---

### `correctness/invalid-slicing`

**Name**: Invalid Slicing
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: GritQL

Detects invalid slicing rule syntax and semantic issues

**Tags**: correctness, slicing

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/invalid-slicing": "error"
    }
  }
}
```

**Learn more**: [Invalid Slicing](https://octofhir.github.io/maki-rs/rules/correctness/invalid-slicing)

---

### `correctness/duplicate-canonical-url`

**Name**: Duplicate Canonical URL
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: GritQL

Detects duplicate canonical URLs across FHIR resources

**Tags**: correctness, url, duplicate

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/duplicate-canonical-url": "error"
    }
  }
}
```

**Learn more**: [Duplicate Canonical URL](https://octofhir.github.io/maki-rs/rules/correctness/duplicate-canonical-url)

---

### `correctness/duplicate-identifier`

**Name**: Duplicate Identifier
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: GritQL

Detects duplicate resource identifiers within FSH files

**Tags**: correctness, identifier, duplicate

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/duplicate-identifier": "error"
    }
  }
}
```

**Learn more**: [Duplicate Identifier](https://octofhir.github.io/maki-rs/rules/correctness/duplicate-identifier)

---

### `correctness/invalid-constraint`

**Name**: Invalid Constraint
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: GritQL

Detects invalid constraint expressions and FHIRPath syntax

**Tags**: correctness, constraint, fhirpath

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/invalid-constraint": "error"
    }
  }
}
```

**Learn more**: [Invalid Constraint](https://octofhir.github.io/maki-rs/rules/correctness/invalid-constraint)

---

### `correctness/missing-parent-profile`

**Name**: Missing Parent Profile
**Severity**: 🟡 Warning
**Fixable**: Yes
**Implementation**: GritQL

Detects profiles without explicit parent profile declarations

**Tags**: correctness, profile, parent

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/missing-parent-profile": "warn"
    }
  }
}
```

**Learn more**: [Missing Parent Profile](https://octofhir.github.io/maki-rs/rules/correctness/missing-parent-profile)

---

### `correctness/invalid-status`

**Name**: Invalid Status
**Severity**: 🔴 Error
**Fixable**: Yes
**Implementation**: GritQL

Detects invalid status values in FHIR resource metadata

**Tags**: correctness, metadata, status

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/invalid-status": "error"
    }
  }
}
```

**Learn more**: [Invalid Status](https://octofhir.github.io/maki-rs/rules/correctness/invalid-status)

---

### `correctness/profile-assignment-present`

**Name**: Profile Assignment Present
**Severity**: 🟡 Warning
**Fixable**: No
**Implementation**: AST

Ensures that profiles have ^status and ^abstract assignments, and Parent declarations

**Tags**: correctness, profile, metadata

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/profile-assignment-present": "warn"
    }
  }
}
```

**Learn more**: [Profile Assignment Present](https://octofhir.github.io/maki-rs/rules/correctness/profile-assignment-present)

---

### `correctness/extension-context-missing`

**Name**: Extension Context Missing
**Severity**: 🔴 Error
**Fixable**: No
**Implementation**: AST

Ensures that extensions have ^context specifications indicating where they can be applied

**Tags**: correctness, extension, context

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "correctness/extension-context-missing": "error"
    }
  }
}
```

**Learn more**: [Extension Context Missing](https://octofhir.github.io/maki-rs/rules/correctness/extension-context-missing)

---

