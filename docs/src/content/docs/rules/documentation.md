---
title: Documentation Rules
description: Rules for proper resource metadata and documentation
---

## Overview

Documentation rules ensure that FHIR resources have proper metadata, descriptions,
and identifying information required for implementation guides and resource discovery.

## Rules

### `documentation/missing-metadata`

**Name**: Missing Metadata
**Severity**: 🟡 Warning
**Fixable**: No
**Implementation**: AST

Warns about missing documentation fields such as Description, Title, Publisher, and Contact to encourage good documentation practices

**Tags**: documentation, metadata, best-practices

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "documentation/missing-metadata": "warn"
    }
  }
}
```

**Learn more**: [Missing Metadata](https://octofhir.github.io/maki-rs/rules/documentation/missing-metadata)

---

### `documentation/missing-description`

**Name**: Missing Description
**Severity**: 🟡 Warning
**Fixable**: Yes
**Implementation**: GritQL

Detects FHIR resources without description metadata

**Tags**: documentation, metadata, description

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "documentation/missing-description": "warn"
    }
  }
}
```

**Learn more**: [Missing Description](https://octofhir.github.io/maki-rs/rules/documentation/missing-description)

---

### `documentation/missing-title`

**Name**: Missing Title
**Severity**: 🔵 Info
**Fixable**: Yes
**Implementation**: GritQL

Detects FHIR resources without title metadata

**Tags**: documentation, metadata, title

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "documentation/missing-title": "info"
    }
  }
}
```

**Learn more**: [Missing Title](https://octofhir.github.io/maki-rs/rules/documentation/missing-title)

---

### `documentation/missing-publisher`

**Name**: Missing Publisher
**Severity**: 🔵 Info
**Fixable**: Yes
**Implementation**: GritQL

Detects FHIR resources without publisher metadata

**Tags**: documentation, metadata, publisher

**Configuration**:

```jsonc
{
  "linter": {
    "rules": {
      "documentation/missing-publisher": "info"
    }
  }
}
```

**Learn more**: [Missing Publisher](https://octofhir.github.io/maki-rs/rules/documentation/missing-publisher)

---

