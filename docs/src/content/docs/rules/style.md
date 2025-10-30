---
title: Style Rules
description: Rules for consistent naming and formatting
---

## Overview

Style rules enforce consistent naming conventions and formatting patterns across your
FSH project, improving readability and maintainability.

## Rules

### `style/profile-naming-convention`

**Name**: Profile Naming Convention
**Severity**: 🟡 Warning
**Fixable**: Yes
**Implementation**: GritQL

Enforces PascalCase naming convention for FHIR profiles

**Tags**: style, naming, profile

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "style/profile-naming-convention": "warn"
    }
  }
}
```

**Learn more**: [Profile Naming Convention](https://octofhir.github.io/maki/rules/style/profile-naming-convention)

---

### `style/naming-convention`

**Name**: Naming Convention
**Severity**: 🟡 Warning
**Fixable**: No
**Implementation**: AST

Enforces consistent naming conventions: PascalCase for Profile/Extension/ValueSet/CodeSystem names and kebab-case for resource IDs

**Tags**: style, naming, consistency, best-practices

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "style/naming-convention": "warn"
    }
  }
}
```

**Learn more**: [Naming Convention](https://octofhir.github.io/maki/rules/style/naming-convention)

---

