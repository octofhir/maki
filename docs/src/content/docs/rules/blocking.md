---
title: Blocking Rules
description: Critical validation rules that must pass first
---

## Overview

Blocking rules are executed **before** all other rules. These rules validate critical
requirements that, if violated, would make other rule checks unreliable or meaningless.

These rules must pass for the linting process to continue with non-blocking rules.

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

**Learn more**: [Required Field Present](https://octofhir.github.io/maki/rules/correctness/required-field-present)

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

**Learn more**: [Invalid Cardinality](https://octofhir.github.io/maki/rules/correctness/invalid-cardinality)

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

**Learn more**: [Binding Strength Present](https://octofhir.github.io/maki/rules/correctness/binding-strength-present)

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

**Learn more**: [Duplicate Definition](https://octofhir.github.io/maki/rules/correctness/duplicate-definition)

---

