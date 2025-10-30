//! Configuration merging logic
//!
//! This module provides merge functionality for combining multiple configuration
//! sources (e.g., base configs and overrides via `extends`).

use super::maki_config::*;
use std::collections::HashMap;

impl MakiConfiguration {
    /// Merge another config into this one (current takes precedence)
    ///
    /// When merging configurations:
    /// - Current (child) values always take precedence over parent values
    /// - Collections (HashMaps, Vecs) are merged intelligently
    /// - The `schema`, `root`, and `extends` fields are not merged (file-specific)
    pub fn merge_with(&mut self, other: MakiConfiguration) {
        // Don't merge schema, root, or extends (these are file-specific)

        // Merge linter
        if let Some(other_linter) = other.linter {
            if let Some(ref mut linter) = self.linter {
                linter.merge_with(other_linter);
            } else {
                self.linter = Some(other_linter);
            }
        }

        // Merge formatter
        if let Some(other_formatter) = other.formatter {
            if let Some(ref mut formatter) = self.formatter {
                formatter.merge_with(other_formatter);
            } else {
                self.formatter = Some(other_formatter);
            }
        }

        // Merge files
        if let Some(other_files) = other.files {
            if let Some(ref mut files) = self.files {
                files.merge_with(other_files);
            } else {
                self.files = Some(other_files);
            }
        }
    }
}

impl LinterConfiguration {
    /// Merge linter configuration (current takes precedence)
    pub fn merge_with(&mut self, other: LinterConfiguration) {
        // Current value takes precedence
        if self.enabled.is_none() {
            self.enabled = other.enabled;
        }

        // Merge rules
        if let Some(other_rules) = other.rules {
            if let Some(ref mut rules) = self.rules {
                rules.merge_with(other_rules);
            } else {
                self.rules = Some(other_rules);
            }
        }

        // Append rule directories (don't override)
        if let Some(other_dirs) = other.rule_directories {
            if let Some(ref mut dirs) = self.rule_directories {
                // Append unique directories
                for dir in other_dirs {
                    if !dirs.contains(&dir) {
                        dirs.push(dir);
                    }
                }
            } else {
                self.rule_directories = Some(other_dirs);
            }
        }
    }
}

impl RulesConfiguration {
    /// Merge rules configuration (current takes precedence)
    pub fn merge_with(&mut self, other: RulesConfiguration) {
        // Merge boolean flags (current takes precedence)
        if self.recommended.is_none() {
            self.recommended = other.recommended;
        }

        if self.all.is_none() {
            self.all = other.all;
        }

        // Merge rule maps (current takes precedence)
        Self::merge_rule_map(&mut self.blocking, other.blocking);
        Self::merge_rule_map(&mut self.correctness, other.correctness);
        Self::merge_rule_map(&mut self.suspicious, other.suspicious);
        Self::merge_rule_map(&mut self.style, other.style);
        Self::merge_rule_map(&mut self.documentation, other.documentation);
    }

    /// Merge a rule severity map
    ///
    /// Rules from the target (child config) always take precedence.
    /// Rules that only exist in the source (parent config) are added.
    fn merge_rule_map(
        target: &mut Option<HashMap<String, RuleSeverity>>,
        source: Option<HashMap<String, RuleSeverity>>,
    ) {
        if let Some(source_map) = source {
            if let Some(target_map) = target {
                // Add rules from source that aren't in target
                for (rule, severity) in source_map {
                    target_map.entry(rule).or_insert(severity);
                }
            } else {
                *target = Some(source_map);
            }
        }
    }
}

impl FormatterConfiguration {
    /// Merge formatter configuration (current takes precedence)
    pub fn merge_with(&mut self, other: FormatterConfiguration) {
        if self.enabled.is_none() {
            self.enabled = other.enabled;
        }
        if self.indent_size.is_none() {
            self.indent_size = other.indent_size;
        }
        if self.line_width.is_none() {
            self.line_width = other.line_width;
        }
        if self.align_carets.is_none() {
            self.align_carets = other.align_carets;
        }
    }
}

impl FilesConfiguration {
    /// Merge files configuration
    ///
    /// For file patterns, we append rather than replace to allow
    /// progressive refinement across config hierarchy.
    pub fn merge_with(&mut self, other: FilesConfiguration) {
        // Append patterns (don't override)
        if let Some(other_include) = other.include {
            if let Some(ref mut include) = self.include {
                // Add unique patterns
                for pattern in other_include {
                    if !include.contains(&pattern) {
                        include.push(pattern);
                    }
                }
            } else {
                self.include = Some(other_include);
            }
        }

        if let Some(other_exclude) = other.exclude {
            if let Some(ref mut exclude) = self.exclude {
                // Add unique patterns
                for pattern in other_exclude {
                    if !exclude.contains(&pattern) {
                        exclude.push(pattern);
                    }
                }
            } else {
                self.exclude = Some(other_exclude);
            }
        }

        if let Some(other_ignore_files) = other.ignore_files {
            if let Some(ref mut ignore_files) = self.ignore_files {
                // Add unique ignore files
                for file in other_ignore_files {
                    if !ignore_files.contains(&file) {
                        ignore_files.push(file);
                    }
                }
            } else {
                self.ignore_files = Some(other_ignore_files);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_linter_configuration() {
        let mut base = LinterConfiguration {
            enabled: Some(true),
            rules: Some(RulesConfiguration::default()),
            rule_directories: Some(vec!["dir1".to_string()]),
        };

        let override_config = LinterConfiguration {
            enabled: None,
            rules: None,
            rule_directories: Some(vec!["dir2".to_string()]),
        };

        base.merge_with(override_config);

        assert_eq!(base.enabled, Some(true));
        assert!(base.rule_directories.is_some());
        assert_eq!(base.rule_directories.unwrap().len(), 2);
    }

    #[test]
    fn test_merge_rules_configuration() {
        let mut base = RulesConfiguration {
            recommended: Some(true),
            all: None,
            blocking: None,
            correctness: Some(HashMap::from([("rule1".to_string(), RuleSeverity::Error)])),
            suspicious: None,
            style: None,
            documentation: None,
        };

        let override_config = RulesConfiguration {
            correctness: Some(HashMap::from([
                ("rule1".to_string(), RuleSeverity::Warn),
                ("rule2".to_string(), RuleSeverity::Error),
            ])),
            ..Default::default()
        };

        base.merge_with(override_config);

        let correctness = base.correctness.unwrap();
        assert_eq!(correctness.get("rule1"), Some(&RuleSeverity::Error)); // Base takes precedence
        assert_eq!(correctness.get("rule2"), Some(&RuleSeverity::Error)); // New rule added
    }

    #[test]
    fn test_merge_formatter_configuration() {
        let mut base = FormatterConfiguration {
            enabled: Some(true),
            indent_size: Some(2),
            line_width: None,
            align_carets: Some(true),
        };

        let override_config = FormatterConfiguration {
            enabled: Some(false),
            indent_size: Some(4),
            line_width: Some(120),
            align_carets: None,
        };

        base.merge_with(override_config);

        assert_eq!(base.enabled, Some(true)); // Base takes precedence
        assert_eq!(base.indent_size, Some(2)); // Base takes precedence
        assert_eq!(base.line_width, Some(120)); // Filled from override
        assert_eq!(base.align_carets, Some(true)); // Base takes precedence
    }

    #[test]
    fn test_merge_files_configuration() {
        let mut base = FilesConfiguration {
            include: Some(vec!["**/*.fsh".to_string()]),
            exclude: Some(vec!["**/node_modules/**".to_string()]),
            ignore_files: Some(vec![".fshlintignore".to_string()]),
        };

        let override_config = FilesConfiguration {
            include: Some(vec!["src/**/*.fsh".to_string()]),
            exclude: Some(vec!["**/build/**".to_string()]),
            ignore_files: Some(vec![".customignore".to_string()]),
        };

        base.merge_with(override_config);

        assert_eq!(base.include.as_ref().unwrap().len(), 2);
        assert_eq!(base.exclude.as_ref().unwrap().len(), 2);
        assert_eq!(base.ignore_files.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_merge_full_configuration() {
        let mut base = MakiConfiguration {
            schema: Some("schema1".to_string()),
            root: Some(true),
            extends: Some(vec!["base.json".to_string()]),
            linter: Some(LinterConfiguration::default()),
            formatter: Some(FormatterConfiguration {
                enabled: Some(true),
                indent_size: Some(2),
                line_width: None,
                align_carets: Some(true),
            }),
            files: Some(FilesConfiguration::default()),
        };

        let override_config = MakiConfiguration {
            schema: Some("schema2".to_string()),
            root: Some(false),
            extends: Some(vec!["override.json".to_string()]),
            linter: None,
            formatter: Some(FormatterConfiguration {
                enabled: None,
                indent_size: Some(4),
                line_width: Some(120),
                align_carets: None,
            }),
            files: None,
        };

        base.merge_with(override_config);

        // Schema, root, extends should not be merged
        assert_eq!(base.schema, Some("schema1".to_string()));
        assert_eq!(base.root, Some(true));
        assert_eq!(base.extends, Some(vec!["base.json".to_string()]));

        // Formatter should be merged
        assert_eq!(base.formatter.as_ref().unwrap().indent_size, Some(2)); // Base takes precedence
        assert_eq!(base.formatter.as_ref().unwrap().line_width, Some(120)); // Filled from override
    }

    #[test]
    fn test_merge_none_values() {
        let mut base = MakiConfiguration {
            linter: None,
            ..Default::default()
        };

        let override_config = MakiConfiguration::default();

        base.merge_with(override_config);

        assert!(base.linter.is_some()); // Should get default linter from override
    }
}
