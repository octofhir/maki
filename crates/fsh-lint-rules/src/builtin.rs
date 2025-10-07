//! Built-in rules for FSH linting

use fsh_lint_core::{AutofixTemplate, FixSafety, Rule, RuleCategory, RuleMetadata, Severity};

pub mod binding;
pub mod cardinality;
pub mod duplicates;
pub mod metadata;
pub mod naming;
pub mod profile;
pub mod required_fields;

/// Collection of built-in FSH linting rules
pub struct BuiltinRules;

impl BuiltinRules {
    /// Get blocking rules that must pass before other rules can run
    /// These validate critical required fields
    pub fn blocking_rules() -> Vec<Rule> {
        vec![
            Self::required_field_present_rule(),
            Self::invalid_cardinality_rule(),
            Self::binding_strength_present_rule(),
            Self::duplicate_definition_rule(),
        ]
    }

    /// Get all built-in correctness rules
    pub fn correctness_rules() -> Vec<Rule> {
        vec![
            Self::invalid_keyword_rule(),
            Self::malformed_alias_rule(),
            Self::invalid_caret_path_rule(),
            Self::missing_profile_id_rule(),
            Self::invalid_identifier_rule(),
            // invalid_cardinality_rule moved to blocking_rules()
            Self::invalid_slicing_rule(),
            // FIXME: duplicate_canonical_url_rule uses custom GritQL functions that hang
            // Self::duplicate_canonical_url_rule(),
            // FIXME: duplicate_identifier_rule uses custom GritQL functions that hang
            // Self::duplicate_identifier_rule(),
            Self::invalid_constraint_rule(),
            Self::missing_parent_profile_rule(),
            Self::invalid_status_rule(),
            Self::profile_assignment_present_rule(),
            Self::extension_context_missing_rule(),
        ]
    }

    /// Get all built-in suspicious pattern rules
    pub fn suspicious_rules() -> Vec<Rule> {
        vec![
            Self::trailing_text_rule(),
            Self::inconsistent_metadata_rule(),
        ]
    }

    /// Get all built-in style rules
    pub fn style_rules() -> Vec<Rule> {
        vec![
            Self::profile_naming_convention_rule(),
            Self::naming_convention_rule(),
        ]
    }

    /// Get all built-in documentation guidance rules
    pub fn documentation_rules() -> Vec<Rule> {
        vec![
            Self::missing_metadata_rule(),
            Self::missing_description_rule(),
            Self::missing_title_rule(),
            Self::missing_publisher_rule(),
        ]
    }

    /// Rule for detecting invalid FSH keywords
    fn invalid_keyword_rule() -> Rule {
        Rule {
            id: "correctness/invalid-keyword".to_string(),
            severity: Severity::Error,
            description: "Detects invalid or misspelled FSH keywords".to_string(),
            gritql_pattern: r#"
                identifier where {
                    $identifier <: or {
                        "Profil", "profil", "PROFILE",
                        "Extensio", "extensio", "EXTENSION", 
                        "ValueSe", "valuese", "VALUESET",
                        "CodeSyste", "codesyste", "CODESYSTEM",
                        "Instanc", "instanc", "INSTANCE",
                        "Invarian", "invarian", "INVARIANT"
                    }
                }
            "#
            .to_string(),
            autofix: Some(AutofixTemplate {
                description: "Correct the misspelled keyword".to_string(),
                replacement_template: "Profile".to_string(),
                safety: FixSafety::Safe,
            }),
            metadata: RuleMetadata {
                id: "correctness/invalid-keyword".to_string(),
                name: "Invalid Keyword".to_string(),
                description:
                    "Detects invalid or misspelled FSH keywords like 'Profil' instead of 'Profile'"
                        .to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec!["correctness".to_string(), "keywords".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/invalid-keyword"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting malformed alias declarations
    fn malformed_alias_rule() -> Rule {
        Rule {
            id: "correctness/malformed-alias".to_string(),
            severity: Severity::Error,
            description: "Detects malformed alias declarations".to_string(),
            gritql_pattern: r#"
                alias_declaration where {
                    or {
                        not contains "=",
                        contains "==" or contains "= =",
                        starts_with "Alias:",
                        $alias_name where $alias_name <: r"[^a-zA-Z0-9_-]"
                    }
                }
            "#
            .to_string(),
            autofix: Some(AutofixTemplate {
                description: "Fix the alias declaration syntax".to_string(),
                replacement_template: "Alias: $alias_name = $target".to_string(),
                safety: FixSafety::Unsafe,
            }),
            metadata: RuleMetadata {
                id: "correctness/malformed-alias".to_string(),
                name: "Malformed Alias".to_string(),
                description: "Detects malformed alias declarations with syntax errors".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec!["correctness".to_string(), "alias".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/malformed-alias"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting invalid caret paths
    fn invalid_caret_path_rule() -> Rule {
        Rule {
            id: "correctness/invalid-caret-path".to_string(),
            severity: Severity::Error,
            description: "Detects invalid caret path syntax".to_string(),
            gritql_pattern: r#"
                caret_rule where {
                    $path where {
                        or {
                            $path <: r"\.\.",
                            $path <: r"^\.",
                            $path <: r"\.$",
                            $path <: r"[^a-zA-Z0-9\.\[\]_-]",
                            $path <: r"\[\]"
                        }
                    }
                }
            "#
            .to_string(),
            autofix: None,
            metadata: RuleMetadata {
                id: "correctness/invalid-caret-path".to_string(),
                name: "Invalid Caret Path".to_string(),
                description: "Detects invalid caret path syntax in FSH rules".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec![
                    "correctness".to_string(),
                    "caret".to_string(),
                    "path".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/invalid-caret-path"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting trailing text after statements
    fn trailing_text_rule() -> Rule {
        Rule {
            id: "suspicious/trailing-text".to_string(),
            severity: Severity::Warning,
            description: "Detects unexpected trailing text after FSH statements".to_string(),
            gritql_pattern: r#"
                line where {
                    and {
                        or {
                            contains "Profile:",
                            contains "Extension:",
                            contains "ValueSet:",
                            contains "CodeSystem:",
                            contains "Instance:",
                            contains "Invariant:",
                            contains "*",
                            contains "^"
                        },
                        $trailing where {
                            and {
                                $trailing <: r"\S+\s+\S+.*$",
                                not $trailing <: r"//.*$"
                            }
                        }
                    }
                }
            "#
            .to_string(),
            autofix: Some(AutofixTemplate {
                description: "Remove trailing text or convert to comment".to_string(),
                replacement_template: "$statement // $trailing_text".to_string(),
                safety: FixSafety::Unsafe,
            }),
            metadata: RuleMetadata {
                id: "suspicious/trailing-text".to_string(),
                name: "Trailing Text".to_string(),
                description: "Detects unexpected trailing text after FSH statements".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Suspicious,
                tags: vec!["suspicious".to_string(), "formatting".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/suspicious/trailing-text"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting missing profile IDs
    fn missing_profile_id_rule() -> Rule {
        Rule {
            id: "correctness/missing-profile-id".to_string(),
            severity: Severity::Error,
            description: "Detects profile declarations without proper IDs".to_string(),
            gritql_pattern: r#"
                profile_declaration where {
                    or {
                        not contains ":",
                        $id where $id <: r"Profile:\s*$",
                        $id where {
                            and {
                                $id <: r"Profile:\s*(.+)",
                                not $id <: r"Profile:\s*[a-zA-Z][a-zA-Z0-9_-]*"
                            }
                        }
                    }
                }
            "#
            .to_string(),
            autofix: None,
            metadata: RuleMetadata {
                id: "correctness/missing-profile-id".to_string(),
                name: "Missing Profile ID".to_string(),
                description: "Detects profile declarations without proper identifiers".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec![
                    "correctness".to_string(),
                    "profile".to_string(),
                    "id".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/missing-profile-id"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting invalid identifiers
    fn invalid_identifier_rule() -> Rule {
        Rule {
            id: "correctness/invalid-identifier".to_string(),
            severity: Severity::Error,
            description: "Detects invalid identifier syntax".to_string(),
            gritql_pattern: r#"
                identifier where {
                    or {
                        $identifier <: r"^[0-9]",
                        $identifier <: r"[^a-zA-Z0-9_-]",
                        $identifier <: or {
                            "true", "false", "null",
                            "and", "or", "not",
                            "if", "then", "else"
                        }
                    }
                }
            "#
            .to_string(),
            autofix: Some(AutofixTemplate {
                description: "Suggest valid identifier format".to_string(),
                replacement_template: "_$identifier".to_string(),
                safety: FixSafety::Unsafe,
            }),
            metadata: RuleMetadata {
                id: "correctness/invalid-identifier".to_string(),
                name: "Invalid Identifier".to_string(),
                description: "Detects invalid identifier syntax in FSH files".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec!["correctness".to_string(), "identifier".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/invalid-identifier"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting invalid slicing rules
    fn invalid_slicing_rule() -> Rule {
        Rule {
            id: "correctness/invalid-slicing".to_string(),
            severity: Severity::Error,
            description: "Detects invalid slicing rule syntax and semantics".to_string(),
            gritql_pattern: r#"
                slicing_rule where {
                    or {
                        and {
                            contains "slicing",
                            not contains "discriminator"
                        },
                        and {
                            contains "discriminator.type",
                            not contains "discriminator.path"
                        }
                    }
                }
            "#
            .to_string(),
            autofix: None,
            metadata: RuleMetadata {
                id: "correctness/invalid-slicing".to_string(),
                name: "Invalid Slicing".to_string(),
                description: "Detects invalid slicing rule syntax and semantic issues".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec!["correctness".to_string(), "slicing".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/invalid-slicing"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting duplicate canonical URLs
    #[allow(dead_code)]
    fn duplicate_canonical_url_rule() -> Rule {
        Rule {
            id: "correctness/duplicate-canonical-url".to_string(),
            severity: Severity::Error,
            description: "Detects duplicate canonical URLs across resources".to_string(),
            gritql_pattern: r#"
                resource_declaration where {
                    $url where {
                        and {
                            $url <: r"\^url\s*=\s*\"([^\"]+)\"",
                            duplicate_url($url, $file)
                        }
                    }
                }
            "#.to_string(),
            autofix: None,
            metadata: RuleMetadata {
                id: "correctness/duplicate-canonical-url".to_string(),
                name: "Duplicate Canonical URL".to_string(),
                description: "Detects duplicate canonical URLs across FHIR resources".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec!["correctness".to_string(), "url".to_string(), "duplicate".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some("https://octofhir.github.io/fsh-lint-rs/rules/correctness/duplicate-canonical-url".to_string()),
            },
        is_ast_rule: false,
        }
    }

    /// Rule for detecting duplicate identifiers
    #[allow(dead_code)]
    fn duplicate_identifier_rule() -> Rule {
        Rule {
            id: "correctness/duplicate-identifier".to_string(),
            severity: Severity::Error,
            description: "Detects duplicate resource identifiers within a file".to_string(),
            gritql_pattern: r#"
                resource_declaration where {
                    $identifier where {
                        and {
                            $identifier <: r"(Profile|Extension|ValueSet|CodeSystem|Instance|Invariant):\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            duplicate_identifier($identifier, $file)
                        }
                    }
                }
            "#.to_string(),
            autofix: None,
            metadata: RuleMetadata {
                id: "correctness/duplicate-identifier".to_string(),
                name: "Duplicate Identifier".to_string(),
                description: "Detects duplicate resource identifiers within FSH files".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec!["correctness".to_string(), "identifier".to_string(), "duplicate".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some("https://octofhir.github.io/fsh-lint-rs/rules/correctness/duplicate-identifier".to_string()),
            },
        is_ast_rule: false,
        }
    }

    /// Rule for detecting invalid constraint expressions
    fn invalid_constraint_rule() -> Rule {
        Rule {
            id: "correctness/invalid-constraint".to_string(),
            severity: Severity::Error,
            description: "Detects invalid constraint expressions and FHIRPath".to_string(),
            gritql_pattern: r#"
                constraint_rule where {
                    or {
                        and {
                            contains "obeys",
                            not contains ":"
                        },
                        $expression where {
                            and {
                                $expression <: r":\s*(.+)",
                                $expression <: r":\s*$"
                            }
                        }
                    }
                }
            "#
            .to_string(),
            autofix: None,
            metadata: RuleMetadata {
                id: "correctness/invalid-constraint".to_string(),
                name: "Invalid Constraint".to_string(),
                description: "Detects invalid constraint expressions and FHIRPath syntax"
                    .to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec![
                    "correctness".to_string(),
                    "constraint".to_string(),
                    "fhirpath".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/invalid-constraint"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting missing parent profile declarations
    fn missing_parent_profile_rule() -> Rule {
        Rule {
            id: "correctness/missing-parent-profile".to_string(),
            severity: Severity::Warning,
            description: "Detects profiles without explicit parent declarations".to_string(),
            gritql_pattern: r#"
                profile_declaration where {
                    and {
                        $profile <: r"Profile:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                        not contains "Parent:"
                    }
                }
            "#.to_string(),
            autofix: Some(AutofixTemplate {
                description: "Add explicit parent declaration".to_string(),
                replacement_template: "$profile\nParent: Resource".to_string(),
                safety: FixSafety::Unsafe,
            }),
            metadata: RuleMetadata {
                id: "correctness/missing-parent-profile".to_string(),
                name: "Missing Parent Profile".to_string(),
                description: "Detects profiles without explicit parent profile declarations".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Correctness,
                tags: vec!["correctness".to_string(), "profile".to_string(), "parent".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some("https://octofhir.github.io/fsh-lint-rs/rules/correctness/missing-parent-profile".to_string()),
            },
        is_ast_rule: false,
        }
    }

    /// Rule for enforcing profile naming conventions
    fn profile_naming_convention_rule() -> Rule {
        Rule {
            id: "style/profile-naming-convention".to_string(),
            severity: Severity::Warning,
            description: "Enforces consistent naming conventions for profiles".to_string(),
            gritql_pattern: r#"
                profile_declaration where {
                    $profile_name where {
                        and {
                            $profile_name <: r"Profile:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            or {
                                not $profile_name <: r"^[A-Z]",
                                $profile_name <: r"[a-z][A-Z]|_[a-z]",
                                $profile_name <: r"[A-Z]{3,}",
                                $profile_name <: r"\d+$"
                            }
                        }
                    }
                }
            "#
            .to_string(),
            autofix: Some(AutofixTemplate {
                description: "Convert to PascalCase naming convention".to_string(),
                replacement_template: "$pascal_case_name".to_string(),
                safety: FixSafety::Unsafe,
            }),
            metadata: RuleMetadata {
                id: "style/profile-naming-convention".to_string(),
                name: "Profile Naming Convention".to_string(),
                description: "Enforces PascalCase naming convention for FHIR profiles".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Style,
                tags: vec![
                    "style".to_string(),
                    "naming".to_string(),
                    "profile".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/style/profile-naming-convention"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting missing description fields
    fn missing_description_rule() -> Rule {
        Rule {
            id: "documentation/missing-description".to_string(),
            severity: Severity::Warning,
            description: "Detects resources without description metadata".to_string(),
            gritql_pattern: r#"
                resource_declaration where {
                    and {
                        $resource <: or {
                            r"Profile:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"Extension:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"ValueSet:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"CodeSystem:\s*([a-zA-Z][a-zA-Z0-9_-]*)"
                        },
                        not contains "^description",
                        not contains "Description:"
                    }
                }
            "#.to_string(),
            autofix: Some(AutofixTemplate {
                description: "Add description field".to_string(),
                replacement_template: "$resource\n^description = \"Add description here\"".to_string(),
                safety: FixSafety::Safe,
            }),
            metadata: RuleMetadata {
                id: "documentation/missing-description".to_string(),
                name: "Missing Description".to_string(),
                description: "Detects FHIR resources without description metadata".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Documentation,
                tags: vec!["documentation".to_string(), "metadata".to_string(), "description".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some("https://octofhir.github.io/fsh-lint-rs/rules/documentation/missing-description".to_string()),
            },
        is_ast_rule: false,
        }
    }

    /// Rule for detecting missing title fields
    fn missing_title_rule() -> Rule {
        Rule {
            id: "documentation/missing-title".to_string(),
            severity: Severity::Info,
            description: "Detects resources without title metadata".to_string(),
            gritql_pattern: r#"
                resource_declaration where {
                    and {
                        $resource <: or {
                            r"Profile:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"Extension:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"ValueSet:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"CodeSystem:\s*([a-zA-Z][a-zA-Z0-9_-]*)"
                        },
                        not contains "^title",
                        not contains "Title:"
                    }
                }
            "#
            .to_string(),
            autofix: Some(AutofixTemplate {
                description: "Add title field".to_string(),
                replacement_template: "$resource\n^title = \"$resource_name\"".to_string(),
                safety: FixSafety::Safe,
            }),
            metadata: RuleMetadata {
                id: "documentation/missing-title".to_string(),
                name: "Missing Title".to_string(),
                description: "Detects FHIR resources without title metadata".to_string(),
                severity: Severity::Info,
                category: RuleCategory::Documentation,
                tags: vec![
                    "documentation".to_string(),
                    "metadata".to_string(),
                    "title".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/documentation/missing-title"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting inconsistent metadata fields
    fn inconsistent_metadata_rule() -> Rule {
        Rule {
            id: "suspicious/inconsistent-metadata".to_string(),
            severity: Severity::Warning,
            description: "Detects inconsistent metadata across related resources".to_string(),
            gritql_pattern: r#"
                resource_declaration where {
                    and {
                        $resource <: r"(Profile|Extension|ValueSet|CodeSystem):\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                        or {
                            contains "^publisher",
                            contains "^version"
                        }
                    }
                }
            "#.to_string(),
            autofix: None,
            metadata: RuleMetadata {
                id: "suspicious/inconsistent-metadata".to_string(),
                name: "Inconsistent Metadata".to_string(),
                description: "Detects inconsistent metadata fields across related FHIR resources".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Suspicious,
                tags: vec!["suspicious".to_string(), "metadata".to_string(), "consistency".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: Some("https://octofhir.github.io/fsh-lint-rs/rules/suspicious/inconsistent-metadata".to_string()),
            },
        is_ast_rule: false,
        }
    }

    /// Rule for detecting missing publisher information
    fn missing_publisher_rule() -> Rule {
        Rule {
            id: "documentation/missing-publisher".to_string(),
            severity: Severity::Info,
            description: "Detects resources without publisher metadata".to_string(),
            gritql_pattern: r#"
                resource_declaration where {
                    and {
                        $resource <: or {
                            r"Profile:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"Extension:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"ValueSet:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                            r"CodeSystem:\s*([a-zA-Z][a-zA-Z0-9_-]*)"
                        },
                        not contains "^publisher",
                        not contains "Publisher:"
                    }
                }
            "#
            .to_string(),
            autofix: Some(AutofixTemplate {
                description: "Add publisher field".to_string(),
                replacement_template: "$resource\n^publisher = \"Add publisher here\"".to_string(),
                safety: FixSafety::Safe,
            }),
            metadata: RuleMetadata {
                id: "documentation/missing-publisher".to_string(),
                name: "Missing Publisher".to_string(),
                description: "Detects FHIR resources without publisher metadata".to_string(),
                severity: Severity::Info,
                category: RuleCategory::Documentation,
                tags: vec![
                    "documentation".to_string(),
                    "metadata".to_string(),
                    "publisher".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/documentation/missing-publisher"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Rule for detecting invalid status values
    fn invalid_status_rule() -> Rule {
        Rule {
            id: "correctness/invalid-status".to_string(),
            severity: Severity::Error,
            description: "Detects invalid status values in resource metadata".to_string(),
            gritql_pattern: r#"
                status_field where {
                    and {
                        $status <: r"\^status\s*=\s*\"([^\"]+)\"",
                        not $status_value <: or {
                            "draft", "active", "retired", "unknown"
                        }
                    }
                }
            "#
            .to_string(),
            autofix: Some(AutofixTemplate {
                description: "Set status to draft".to_string(),
                replacement_template: "^status = \"draft\"".to_string(),
                safety: FixSafety::Unsafe,
            }),
            metadata: RuleMetadata {
                id: "correctness/invalid-status".to_string(),
                name: "Invalid Status".to_string(),
                description: "Detects invalid status values in FHIR resource metadata".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec![
                    "correctness".to_string(),
                    "metadata".to_string(),
                    "status".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/invalid-status"
                        .to_string(),
                ),
            },
            is_ast_rule: false,
        }
    }

    /// Get all built-in rules (convenience method)
    pub fn all_rules() -> Vec<Rule> {
        let mut rules = Vec::new();
        rules.extend(Self::blocking_rules());
        rules.extend(Self::correctness_rules());
        rules.extend(Self::suspicious_rules());
        rules.extend(Self::style_rules());
        rules.extend(Self::documentation_rules());
        rules
    }

    /// Rule for validating required fields are present
    /// This is a BLOCKING rule - implemented directly with AST for rich features
    fn required_field_present_rule() -> Rule {
        Rule {
            id: required_fields::REQUIRED_FIELD_PRESENT.to_string(),
            severity: Severity::Error,
            description: "Validates that FHIR resources have all required metadata fields".to_string(),
            // Empty GritQL pattern - this rule uses direct AST access
            gritql_pattern: String::new(),
            autofix: None, // Autofixes are generated per-instance in the AST checker
            metadata: RuleMetadata {
                id: required_fields::REQUIRED_FIELD_PRESENT.to_string(),
                name: "Required Field Present".to_string(),
                description: "Ensures that Profiles, CodeSystems, and ValueSets have required fields (Name, Id, Title)".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Custom("blocking".to_string()),
                tags: vec![
                    "correctness".to_string(),
                    "blocking".to_string(),
                    "metadata".to_string(),
                    "required-fields".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/blocking/required-field-present"
                        .to_string(),
                ),
            },
            is_ast_rule: true,
        }
    }

    /// Rule for validating cardinality constraints
    /// This is a BLOCKING rule - implemented directly with AST for rich features
    fn invalid_cardinality_rule() -> Rule {
        Rule {
            id: cardinality::INVALID_CARDINALITY.to_string(),
            severity: Severity::Error,
            description: "Validates cardinality constraints in element rules".to_string(),
            // Empty GritQL pattern - this rule uses direct AST access
            gritql_pattern: String::new(),
            autofix: None, // Autofixes are generated per-instance in the AST checker
            metadata: RuleMetadata {
                id: cardinality::INVALID_CARDINALITY.to_string(),
                name: "Invalid Cardinality".to_string(),
                description: "Detects invalid cardinality expressions such as reversed bounds (1..0), invalid syntax, and non-numeric values".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Custom("blocking".to_string()),
                tags: vec![
                    "correctness".to_string(),
                    "blocking".to_string(),
                    "cardinality".to_string(),
                    "constraints".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/blocking/invalid-cardinality"
                        .to_string(),
                ),
            },
            is_ast_rule: true,
        }
    }

    /// Rule for validating binding strength is present and valid
    /// This is a BLOCKING rule - implemented directly with AST for rich features
    fn binding_strength_present_rule() -> Rule {
        Rule {
            id: binding::BINDING_STRENGTH_PRESENT.to_string(),
            severity: Severity::Error,
            description: "Validates that bindings to value sets have proper strength specifications".to_string(),
            // Empty GritQL pattern - this rule uses direct AST access
            gritql_pattern: String::new(),
            autofix: None, // Autofixes are generated per-instance in the AST checker
            metadata: RuleMetadata {
                id: binding::BINDING_STRENGTH_PRESENT.to_string(),
                name: "Binding Strength Present".to_string(),
                description: "Ensures that bindings to value sets specify strength (required, extensible, preferred, or example) and use valid strength values".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Custom("blocking".to_string()),
                tags: vec![
                    "correctness".to_string(),
                    "blocking".to_string(),
                    "binding".to_string(),
                    "terminology".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/blocking/binding-strength-present"
                        .to_string(),
                ),
            },
            is_ast_rule: true,
        }
    }

    /// Rule for detecting missing metadata documentation
    /// This is a WARNING-level rule - implemented directly with AST for rich features
    fn missing_metadata_rule() -> Rule {
        Rule {
            id: metadata::MISSING_METADATA.to_string(),
            severity: Severity::Warning,
            description: "Validates that FHIR resources have proper documentation metadata".to_string(),
            // Empty GritQL pattern - this rule uses direct AST access
            gritql_pattern: String::new(),
            autofix: None, // Autofixes are generated per-instance in the AST checker
            metadata: RuleMetadata {
                id: metadata::MISSING_METADATA.to_string(),
                name: "Missing Metadata".to_string(),
                description: "Warns about missing documentation fields such as Description, Title, Publisher, and Contact to encourage good documentation practices".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Documentation,
                tags: vec![
                    "documentation".to_string(),
                    "metadata".to_string(),
                    "best-practices".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/documentation/missing-metadata"
                        .to_string(),
                ),
            },
            is_ast_rule: true,
        }
    }

    /// Rule for detecting missing profile assignments
    fn profile_assignment_present_rule() -> Rule {
        Rule {
            id: profile::PROFILE_ASSIGNMENT_PRESENT.to_string(),
            severity: Severity::Warning,
            description: "Validates that profiles have proper status and abstract assignments".to_string(),
            gritql_pattern: String::new(),
            autofix: None,
            metadata: RuleMetadata {
                id: profile::PROFILE_ASSIGNMENT_PRESENT.to_string(),
                name: "Profile Assignment Present".to_string(),
                description: "Ensures that profiles have ^status and ^abstract assignments, and Parent declarations".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Correctness,
                tags: vec![
                    "correctness".to_string(),
                    "profile".to_string(),
                    "metadata".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/profile-assignment-present"
                        .to_string(),
                ),
            },
            is_ast_rule: true,
        }
    }

    /// Rule for detecting missing extension context
    fn extension_context_missing_rule() -> Rule {
        Rule {
            id: profile::EXTENSION_CONTEXT_MISSING.to_string(),
            severity: Severity::Error,
            description: "Validates that extensions specify where they can be used".to_string(),
            gritql_pattern: String::new(),
            autofix: None,
            metadata: RuleMetadata {
                id: profile::EXTENSION_CONTEXT_MISSING.to_string(),
                name: "Extension Context Missing".to_string(),
                description: "Ensures that extensions have ^context specifications indicating where they can be applied".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Correctness,
                tags: vec![
                    "correctness".to_string(),
                    "extension".to_string(),
                    "context".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/correctness/extension-context-missing"
                        .to_string(),
                ),
            },
            is_ast_rule: true,
        }
    }

    /// Rule for detecting duplicate definitions
    /// This is a BLOCKING rule - detects duplicate names, IDs, and URLs
    fn duplicate_definition_rule() -> Rule {
        Rule {
            id: duplicates::DUPLICATE_DEFINITION.to_string(),
            severity: Severity::Error,
            description: "Detects duplicate resource names, IDs, and canonical URLs".to_string(),
            gritql_pattern: String::new(),
            autofix: None,
            metadata: RuleMetadata {
                id: duplicates::DUPLICATE_DEFINITION.to_string(),
                name: "Duplicate Definition".to_string(),
                description: "Prevents duplicate resource names, IDs, and canonical URLs which would cause conflicts".to_string(),
                severity: Severity::Error,
                category: RuleCategory::Custom("blocking".to_string()),
                tags: vec![
                    "correctness".to_string(),
                    "blocking".to_string(),
                    "duplicates".to_string(),
                    "conflicts".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/blocking/duplicate-definition"
                        .to_string(),
                ),
            },
            is_ast_rule: true,
        }
    }

    /// Rule for enforcing naming conventions across all resources
    /// This is an AST-based style rule for PascalCase names and kebab-case IDs
    fn naming_convention_rule() -> Rule {
        Rule {
            id: naming::NAMING_CONVENTION.to_string(),
            severity: Severity::Warning,
            description: "Enforces naming conventions: PascalCase for resource names, kebab-case for IDs".to_string(),
            gritql_pattern: String::new(),
            autofix: None,
            metadata: RuleMetadata {
                id: naming::NAMING_CONVENTION.to_string(),
                name: "Naming Convention".to_string(),
                description: "Enforces consistent naming conventions: PascalCase for Profile/Extension/ValueSet/CodeSystem names and kebab-case for resource IDs".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Style,
                tags: vec![
                    "style".to_string(),
                    "naming".to_string(),
                    "consistency".to_string(),
                    "best-practices".to_string(),
                ],
                version: Some("1.0.0".to_string()),
                docs_url: Some(
                    "https://octofhir.github.io/fsh-lint-rs/rules/style/naming-convention"
                        .to_string(),
                ),
            },
            is_ast_rule: true,
        }
    }
}
