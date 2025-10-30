//! Rule Documentation Generator
//!
//! Dynamically generates Markdown documentation for all built-in lint rules
//! by querying the rule registry.

use anyhow::Result;
use maki_core::{Rule, RuleCategory, Severity};
use maki_rules::BuiltinRules;
use std::fs;
use std::path::Path;
use tracing::info;

pub struct RuleDocGenerator;

impl RuleDocGenerator {
    /// Generate rule documentation files from the registry
    pub fn generate<P: AsRef<Path>>(output_dir: P) -> Result<()> {
        let output_dir = output_dir.as_ref();
        info!("Generating rule documentation in {:?}", output_dir);

        // Create output directory
        fs::create_dir_all(output_dir)?;

        // Get all rules from the registry
        let all_rules = BuiltinRules::all_rules();
        info!("Found {} total rules", all_rules.len());

        // Generate documentation for each category
        Self::generate_blocking_rules_doc(output_dir, &all_rules)?;
        Self::generate_correctness_rules_doc(output_dir, &all_rules)?;
        Self::generate_suspicious_rules_doc(output_dir, &all_rules)?;
        Self::generate_style_rules_doc(output_dir, &all_rules)?;
        Self::generate_documentation_rules_doc(output_dir, &all_rules)?;

        // Generate index page
        Self::generate_index(output_dir, &all_rules)?;

        info!("Rule documentation generated successfully");
        Ok(())
    }

    /// Generate index page with overview of all rules
    fn generate_index<P: AsRef<Path>>(output_dir: P, all_rules: &[Rule]) -> Result<()> {
        let mut content = String::from(
            r#"---
title: Built-in Rules
description: Overview of all built-in FSH Lint rules
---

FSH Lint comes with a comprehensive set of 25+ built-in rules organized into categories:

## Rule Categories

"#,
        );

        // Blocking Rules section
        let blocking_count = BuiltinRules::blocking_rules().len();
        content.push_str(&format!(
            r#"### Blocking Rules ({blocking_count} rules)

Rules that must pass before other rules can run. These validate critical requirements.

- [Blocking Rules](./blocking/) - Critical validation rules

"#
        ));

        // Count rules per category
        let correctness_count = all_rules
            .iter()
            .filter(|r| matches!(r.metadata.category, RuleCategory::Correctness))
            .count();
        let style_count = all_rules
            .iter()
            .filter(|r| matches!(r.metadata.category, RuleCategory::Style))
            .count();
        let suspicious_count = all_rules
            .iter()
            .filter(|r| matches!(r.metadata.category, RuleCategory::Suspicious))
            .count();
        let documentation_count = all_rules
            .iter()
            .filter(|r| matches!(r.metadata.category, RuleCategory::Documentation))
            .count();

        // Correctness Rules
        content.push_str(&format!(
            r#"### Correctness ({correctness_count} rules)

Rules that ensure FHIR specification compliance and prevent errors

- [Correctness Rules](./correctness/)

"#
        ));

        // Style Rules
        content.push_str(&format!(
            r#"### Style ({style_count} rules)

Rules that enforce consistent naming and formatting patterns

- [Style Rules](./style/)

"#
        ));

        // Suspicious Rules
        content.push_str(&format!(
            r#"### Suspicious ({suspicious_count} rules)

Rules that detect potentially problematic patterns in FSH code

- [Suspicious Rules](./suspicious/)

"#
        ));

        // Documentation Rules
        content.push_str(&format!(
            r#"### Documentation ({documentation_count} rules)

Rules that ensure proper resource metadata and documentation

- [Documentation Rules](./documentation/)

"#
        ));

        content.push_str(
            r#"
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
"#,
        );

        let path = output_dir.as_ref().join("index.md");
        fs::write(&path, content)?;
        println!("  ✓ Generated rule index: {}", path.display());
        Ok(())
    }

    /// Generate documentation for blocking rules
    fn generate_blocking_rules_doc<P: AsRef<Path>>(
        output_dir: P,
        _all_rules: &[Rule],
    ) -> Result<()> {
        let blocking_rules = BuiltinRules::blocking_rules();

        let mut content = String::from(
            r#"---
title: Blocking Rules
description: Critical validation rules that must pass first
---

## Overview

Blocking rules are executed **before** all other rules. These rules validate critical
requirements that, if violated, would make other rule checks unreliable or meaningless.

These rules must pass for the linting process to continue with non-blocking rules.

## Rules

"#,
        );

        for rule in &blocking_rules {
            Self::append_rule_documentation(&mut content, rule);
        }

        let path = output_dir.as_ref().join("blocking.md");
        fs::write(&path, content)?;
        println!("  ✓ Generated blocking rules: {}", path.display());
        Ok(())
    }

    /// Generate documentation for correctness rules
    fn generate_correctness_rules_doc<P: AsRef<Path>>(
        output_dir: P,
        all_rules: &[Rule],
    ) -> Result<()> {
        let correctness_rules: Vec<_> = all_rules
            .iter()
            .filter(|r| matches!(r.metadata.category, RuleCategory::Correctness))
            .collect();

        let mut content = String::from(
            r#"---
title: Correctness Rules
description: Rules for FHIR specification compliance
---

## Overview

Correctness rules ensure that your FSH code complies with the FHIR specification and
prevent syntax errors, semantic violations, and runtime issues.

## Rules

"#,
        );

        for rule in correctness_rules {
            Self::append_rule_documentation(&mut content, rule);
        }

        let path = output_dir.as_ref().join("correctness.md");
        fs::write(&path, content)?;
        println!("  ✓ Generated correctness rules: {}", path.display());
        Ok(())
    }

    /// Generate documentation for suspicious rules
    fn generate_suspicious_rules_doc<P: AsRef<Path>>(
        output_dir: P,
        all_rules: &[Rule],
    ) -> Result<()> {
        let suspicious_rules: Vec<_> = all_rules
            .iter()
            .filter(|r| matches!(r.metadata.category, RuleCategory::Suspicious))
            .collect();

        let mut content = String::from(
            r#"---
title: Suspicious Rules
description: Rules for detecting potentially problematic patterns
---

## Overview

Suspicious rules detect patterns that are technically valid but often indicate bugs,
inconsistencies, or maintainability issues.

## Rules

"#,
        );

        for rule in suspicious_rules {
            Self::append_rule_documentation(&mut content, rule);
        }

        let path = output_dir.as_ref().join("suspicious.md");
        fs::write(&path, content)?;
        println!("  ✓ Generated suspicious rules: {}", path.display());
        Ok(())
    }

    /// Generate documentation for style rules
    fn generate_style_rules_doc<P: AsRef<Path>>(output_dir: P, all_rules: &[Rule]) -> Result<()> {
        let style_rules: Vec<_> = all_rules
            .iter()
            .filter(|r| matches!(r.metadata.category, RuleCategory::Style))
            .collect();

        let mut content = String::from(
            r#"---
title: Style Rules
description: Rules for consistent naming and formatting
---

## Overview

Style rules enforce consistent naming conventions and formatting patterns across your
FSH project, improving readability and maintainability.

## Rules

"#,
        );

        for rule in style_rules {
            Self::append_rule_documentation(&mut content, rule);
        }

        let path = output_dir.as_ref().join("style.md");
        fs::write(&path, content)?;
        println!("  ✓ Generated style rules: {}", path.display());
        Ok(())
    }

    /// Generate documentation for documentation rules
    fn generate_documentation_rules_doc<P: AsRef<Path>>(
        output_dir: P,
        all_rules: &[Rule],
    ) -> Result<()> {
        let doc_rules: Vec<_> = all_rules
            .iter()
            .filter(|r| matches!(r.metadata.category, RuleCategory::Documentation))
            .collect();

        let mut content = String::from(
            r#"---
title: Documentation Rules
description: Rules for proper resource metadata and documentation
---

## Overview

Documentation rules ensure that FHIR resources have proper metadata, descriptions,
and identifying information required for implementation guides and resource discovery.

## Rules

"#,
        );

        for rule in doc_rules {
            Self::append_rule_documentation(&mut content, rule);
        }

        let path = output_dir.as_ref().join("documentation.md");
        fs::write(&path, content)?;
        println!("  ✓ Generated documentation rules: {}", path.display());
        Ok(())
    }

    /// Append formatted documentation for a single rule
    fn append_rule_documentation(content: &mut String, rule: &Rule) {
        let fixable = if rule.autofix.is_some() { "Yes" } else { "No" };

        let severity_str = Self::format_severity(&rule.metadata.severity);
        let implementation = if rule.is_ast_rule { "AST" } else { "GritQL" };

        content.push_str(&format!(
            r#"### `{}`

**Name**: {}
**Severity**: {}
**Fixable**: {}
**Implementation**: {}

{}

**Tags**: {}

"#,
            rule.metadata.id,
            rule.metadata.name,
            severity_str,
            fixable,
            implementation,
            rule.metadata.description,
            rule.metadata.tags.join(", ")
        ));

        // Add configuration example
        content.push_str(&format!(
            r#"**Configuration**:

```jsonc
{{
  "linter": {{
    "rules": {{
      "{}": "{}"
    }}
  }}
}}
```

"#,
            rule.metadata.id,
            Self::severity_to_config(&rule.metadata.severity)
        ));

        // Add link to detailed docs if available
        if let Some(url) = &rule.metadata.docs_url {
            content.push_str(&format!(
                "**Learn more**: [{}]({})\n\n",
                rule.metadata.name, url
            ));
        }

        content.push_str("---\n\n");
    }

    /// Format severity for display
    fn format_severity(severity: &Severity) -> &'static str {
        match severity {
            Severity::Error => "🔴 Error",
            Severity::Warning => "🟡 Warning",
            Severity::Info => "🔵 Info",
            Severity::Hint => "💡 Hint",
        }
    }

    /// Convert severity to configuration value
    fn severity_to_config(severity: &Severity) -> &'static str {
        match severity {
            Severity::Error => "error",
            Severity::Warning => "warn",
            Severity::Info => "info",
            Severity::Hint => "hint",
        }
    }
}
