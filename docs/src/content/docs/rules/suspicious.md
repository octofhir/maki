---
title: Suspicious Rules
description: Rules for detecting potentially problematic patterns
---

## Overview

Suspicious rules detect patterns that are technically valid but often indicate bugs,
inconsistencies, or maintainability issues.

## Rules

### `suspicious/trailing-text`

**Name**: Trailing Text
**Severity**: 🟡 Warning
**Fixable**: Yes
**Implementation**: GritQL

Detects unexpected trailing text after FSH statements

**Tags**: suspicious, formatting

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "suspicious/trailing-text": "warn"
    }
  }
}
```

**Learn more**: [Trailing Text](https://octofhir.github.io/maki/rules/suspicious/trailing-text)

---

### `suspicious/inconsistent-metadata`

**Name**: Inconsistent Metadata
**Severity**: 🟡 Warning
**Fixable**: No
**Implementation**: GritQL

Detects inconsistent metadata fields across related FHIR resources

**Tags**: suspicious, metadata, consistency

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "suspicious/inconsistent-metadata": "warn"
    }
  }
}
```

**Learn more**: [Inconsistent Metadata](https://octofhir.github.io/maki/rules/suspicious/inconsistent-metadata)

---

