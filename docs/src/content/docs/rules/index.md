---
title: Built-in Rules
description: Overview of all built-in FSH Lint rules
---

FSH Lint comes with a comprehensive set of 25+ built-in rules organized into categories:

## Rule Categories

### Blocking Rules (4 rules)

Rules that must pass before other rules can run. These validate critical requirements.

- [Blocking Rules](./blocking/) - Critical validation rules

### Correctness (17 rules)

Rules that ensure FHIR specification compliance and prevent errors

- [Correctness Rules](./correctness/)

### Style (2 rules)

Rules that enforce consistent naming and formatting patterns

- [Style Rules](./style/)

### Suspicious (2 rules)

Rules that detect potentially problematic patterns in FSH code

- [Suspicious Rules](./suspicious/)

### Documentation (4 rules)

Rules that ensure proper resource metadata and documentation

- [Documentation Rules](./documentation/)


## Rule Severity Levels

- **Error** - Must be fixed; prevents compilation or causes runtime issues
- **Warning** - Should be fixed; best practice violation or potential issue
- **Info** - Informational; suggestions for improvement
- **Hint** - Optional; minor style suggestions

## Configuring Rules

See the [Rule Configuration](/configuration/rules/) guide for details on:
- Enabling/disabling rules
- Changing rule severity
- Rule-specific options
- Creating custom rules

## Rule Statistics

Total built-in rules: **25**

By severity:
- Error: Most critical rules
- Warning: Best practice violations
- Info: Documentation suggestions
