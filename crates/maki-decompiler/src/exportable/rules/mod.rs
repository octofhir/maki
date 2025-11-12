//! FSH rule types
//!
//! This module contains all FSH rule types that can appear in profiles,
//! extensions, logical models, value sets, and code systems.

use super::FshValue;

/// Trait for rules that can be exported to FSH
pub trait ExportableRule: std::fmt::Debug {
    /// Convert this rule to FSH syntax
    fn to_fsh(&self) -> String;

    /// Get rule type name (for debugging)
    fn rule_type(&self) -> &'static str;

    /// Support downcasting to concrete rule types
    fn as_any(&self) -> &dyn std::any::Any;

    /// Support mutable downcasting to concrete rule types
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Cardinality Rule: * element 0..1
#[derive(Debug, Clone, PartialEq)]
pub struct CardinalityRule {
    pub path: String,
    pub min: u32,
    pub max: String, // Can be "*"
}

impl ExportableRule for CardinalityRule {
    fn to_fsh(&self) -> String {
        format!("{} {}..{}", self.path, self.min, self.max)
    }

    fn rule_type(&self) -> &'static str {
        "CardinalityRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Combined Cardinality and Flag Rule: * element 0..1 MS
#[derive(Debug, Clone, PartialEq)]
pub struct CardinalityFlagRule {
    pub path: String,
    pub min: u32,
    pub max: String, // Can be "*"
    pub flags: Vec<Flag>,
}

impl ExportableRule for CardinalityFlagRule {
    fn to_fsh(&self) -> String {
        let flags_str = self
            .flags
            .iter()
            .map(|f| f.to_fsh())
            .collect::<Vec<_>>()
            .join(" ");
        format!("{} {}..{} {}", self.path, self.min, self.max, flags_str)
    }

    fn rule_type(&self) -> &'static str {
        "CardinalityFlagRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Assignment Rule: * element = value
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentRule {
    pub path: String,
    pub value: FshValue,
    pub exactly: bool, // true for = (exactly), false for regular assignment
}

impl ExportableRule for AssignmentRule {
    fn to_fsh(&self) -> String {
        format!("{} = {}", self.path, self.value.to_fsh())
    }

    fn rule_type(&self) -> &'static str {
        "AssignmentRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Binding Rule: * element from ValueSet (strength)
#[derive(Debug, Clone, PartialEq)]
pub struct BindingRule {
    pub path: String,
    pub value_set: String,
    pub strength: BindingStrength,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingStrength {
    Required,
    Extensible,
    Preferred,
    Example,
}

impl BindingStrength {
    pub fn to_fsh(&self) -> &'static str {
        match self {
            BindingStrength::Required => "required",
            BindingStrength::Extensible => "extensible",
            BindingStrength::Preferred => "preferred",
            BindingStrength::Example => "example",
        }
    }
}

impl ExportableRule for BindingRule {
    fn to_fsh(&self) -> String {
        format!(
            "{} from {} ({})",
            self.path,
            self.value_set,
            self.strength.to_fsh()
        )
    }

    fn rule_type(&self) -> &'static str {
        "BindingRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Type Rule: * element only Type1 or Type2
#[derive(Debug, Clone, PartialEq)]
pub struct TypeRule {
    pub path: String,
    pub types: Vec<TypeReference>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeReference {
    pub type_name: String,
    pub profiles: Vec<String>,
    pub target_profiles: Vec<String>,
}

impl TypeReference {
    pub fn to_fsh(&self) -> String {
        let mut result = self.type_name.clone();

        if !self.profiles.is_empty() {
            result.push_str(&format!("({})", self.profiles.join(" or ")));
        }

        if !self.target_profiles.is_empty() {
            result.push_str(&format!(
                " References({})",
                self.target_profiles.join(" or ")
            ));
        }

        result
    }
}

impl ExportableRule for TypeRule {
    fn to_fsh(&self) -> String {
        let types_str = self
            .types
            .iter()
            .map(|t| t.to_fsh())
            .collect::<Vec<_>>()
            .join(" or ");
        format!("{} only {}", self.path, types_str)
    }

    fn rule_type(&self) -> &'static str {
        "TypeRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Flag Rule: * element MS SU ?!
#[derive(Debug, Clone, PartialEq)]
pub struct FlagRule {
    pub path: String,
    pub flags: Vec<Flag>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Flag {
    MustSupport, // MS
    Summary,     // SU
    Modifier,    // ?!
    TrialUse,    // TU
    Normative,   // N
    Draft,       // D
}

impl Flag {
    pub fn to_fsh(&self) -> &'static str {
        match self {
            Flag::MustSupport => "MS",
            Flag::Summary => "SU",
            Flag::Modifier => "?!",
            Flag::TrialUse => "TU",
            Flag::Normative => "N",
            Flag::Draft => "D",
        }
    }
}

impl ExportableRule for FlagRule {
    fn to_fsh(&self) -> String {
        let flags_str = self
            .flags
            .iter()
            .map(|f| f.to_fsh())
            .collect::<Vec<_>>()
            .join(" ");
        format!("{} {}", self.path, flags_str)
    }

    fn rule_type(&self) -> &'static str {
        "FlagRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Contains Rule: * element contains slice1 0..1 and slice2 0..*
#[derive(Debug, Clone, PartialEq)]
pub struct ContainsRule {
    pub path: String,
    pub items: Vec<ContainsItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainsItem {
    pub name: String,
    pub type_name: Option<String>, // For extension URLs
    pub min: u32,
    pub max: String,
}

impl ExportableRule for ContainsRule {
    fn to_fsh(&self) -> String {
        let items_str = self
            .items
            .iter()
            .map(|item| match &item.type_name {
                Some(type_name) => {
                    format!(
                        "{} named {} {}..{}",
                        type_name, item.name, item.min, item.max
                    )
                }
                None => {
                    format!("{} {}..{}", item.name, item.min, item.max)
                }
            })
            .collect::<Vec<_>>()
            .join(" and\n  ");
        format!("{} contains {}", self.path, items_str)
    }

    fn rule_type(&self) -> &'static str {
        "ContainsRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Obeys Rule: * obeys invariant-1
#[derive(Debug, Clone, PartialEq)]
pub struct ObeysRule {
    pub path: Option<String>, // None for item-level obeys
    pub invariant: String,
}

impl ExportableRule for ObeysRule {
    fn to_fsh(&self) -> String {
        match &self.path {
            Some(path) => format!("{} obeys {}", path, self.invariant),
            None => format!("obeys {}", self.invariant),
        }
    }

    fn rule_type(&self) -> &'static str {
        "ObeysRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Caret Rule: * ^metadata = value
#[derive(Debug, Clone, PartialEq)]
pub struct CaretValueRule {
    pub path: Option<String>, // None for root-level metadata
    pub caret_path: String,
    pub value: FshValue,
}

impl ExportableRule for CaretValueRule {
    fn to_fsh(&self) -> String {
        match &self.path {
            Some(path) => format!("{} ^{} = {}", path, self.caret_path, self.value.to_fsh()),
            None => format!("^{} = {}", self.caret_path, self.value.to_fsh()),
        }
    }

    fn rule_type(&self) -> &'static str {
        "CaretValueRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Insert Rule: * insert RuleSet
#[derive(Debug, Clone, PartialEq)]
pub struct InsertRule {
    pub rule_set_name: String,
    pub params: Vec<String>,
}

impl ExportableRule for InsertRule {
    fn to_fsh(&self) -> String {
        if self.params.is_empty() {
            format!("insert {}", self.rule_set_name)
        } else {
            format!("insert {}({})", self.rule_set_name, self.params.join(", "))
        }
    }

    fn rule_type(&self) -> &'static str {
        "InsertRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Mapping Rule: * -> "mapping"
#[derive(Debug, Clone, PartialEq)]
pub struct MappingRule {
    pub path: Option<String>,
    pub map: String,
    pub comment: Option<String>,
    pub mime_type: Option<String>,
}

impl ExportableRule for MappingRule {
    fn to_fsh(&self) -> String {
        let mut result = match &self.path {
            Some(path) => format!("{} -> \"{}\"", path, super::escape_string(&self.map)),
            None => format!("-> \"{}\"", super::escape_string(&self.map)),
        };

        if let Some(comment) = &self.comment {
            result.push_str(&format!(" \"{}\"", super::escape_string(comment)));
        }

        if let Some(mime_type) = &self.mime_type {
            result.push_str(&format!(" #{}", mime_type));
        }

        result
    }

    fn rule_type(&self) -> &'static str {
        "MappingRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Add Element Rule: * name 0..* Type "short" "definition"
#[derive(Debug, Clone, PartialEq)]
pub struct AddElementRule {
    pub path: String,
    pub min: u32,
    pub max: String,
    pub types: Vec<String>,
    pub short: Option<String>,
    pub definition: Option<String>,
    pub flags: Vec<Flag>,
}

impl ExportableRule for AddElementRule {
    fn to_fsh(&self) -> String {
        let mut parts = vec![self.path.clone(), format!("{}..{}", self.min, self.max)];

        // Add flags
        for flag in &self.flags {
            parts.push(flag.to_fsh().to_string());
        }

        // Add types
        if !self.types.is_empty() {
            parts.push(self.types.join(" or "));
        }

        // Add short
        if let Some(short) = &self.short {
            parts.push(format!("\"{}\"", super::escape_string(short)));
        }

        // Add definition
        if let Some(def) = &self.definition {
            parts.push(format!("\"{}\"", super::escape_string(def)));
        }

        parts.join(" ")
    }

    fn rule_type(&self) -> &'static str {
        "AddElementRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Local Code Rule (CodeSystem): * #code "display" "definition"
#[derive(Debug, Clone, PartialEq)]
pub struct LocalCodeRule {
    pub code: String,
    pub display: Option<String>,
    pub definition: Option<String>,
}

impl ExportableRule for LocalCodeRule {
    fn to_fsh(&self) -> String {
        let mut result = format!("#{}", self.code);

        if let Some(display) = &self.display {
            result.push_str(&format!(" \"{}\"", super::escape_string(display)));
        }

        if let Some(definition) = &self.definition {
            result.push_str(&format!(" \"{}\"", super::escape_string(definition)));
        }

        result
    }

    fn rule_type(&self) -> &'static str {
        "LocalCodeRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Include Rule (ValueSet): * include codes from system
#[derive(Debug, Clone, PartialEq)]
pub struct IncludeRule {
    pub system: String,
    pub version: Option<String>,
    pub concepts: Vec<IncludeConcept>,
    pub filters: Vec<ValueSetFilter>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncludeConcept {
    pub code: String,
    pub display: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueSetFilter {
    pub property: String,
    pub operator: String,
    pub value: String,
}

impl ExportableRule for IncludeRule {
    fn to_fsh(&self) -> String {
        if !self.concepts.is_empty() {
            // Include specific codes
            let codes_str = self
                .concepts
                .iter()
                .map(|c| match &c.display {
                    Some(display) => {
                        format!(
                            "{}#{} \"{}\"",
                            self.system,
                            c.code,
                            super::escape_string(display)
                        )
                    }
                    None => {
                        format!("{}#{}", self.system, c.code)
                    }
                })
                .collect::<Vec<_>>()
                .join(" and ");
            format!("include {}", codes_str)
        } else if !self.filters.is_empty() {
            // Include with filter
            let filter_str = self
                .filters
                .iter()
                .map(|f| format!("{} {} {}", f.property, f.operator, f.value))
                .collect::<Vec<_>>()
                .join(" and ");
            format!("include codes from {} where {}", self.system, filter_str)
        } else {
            // Include all from system
            match &self.version {
                Some(version) => format!("include codes from {}|{}", self.system, version),
                None => format!("include codes from {}", self.system),
            }
        }
    }

    fn rule_type(&self) -> &'static str {
        "IncludeRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Exclude Rule (ValueSet): * exclude codes from system
#[derive(Debug, Clone, PartialEq)]
pub struct ExcludeRule {
    pub system: String,
    pub version: Option<String>,
    pub concepts: Vec<IncludeConcept>,
    pub filters: Vec<ValueSetFilter>,
}

impl ExportableRule for ExcludeRule {
    fn to_fsh(&self) -> String {
        if !self.concepts.is_empty() {
            let codes_str = self
                .concepts
                .iter()
                .map(|c| format!("{}#{}", self.system, c.code))
                .collect::<Vec<_>>()
                .join(" and ");
            format!("exclude {}", codes_str)
        } else if !self.filters.is_empty() {
            let filter_str = self
                .filters
                .iter()
                .map(|f| format!("{} {} {}", f.property, f.operator, f.value))
                .collect::<Vec<_>>()
                .join(" and ");
            format!("exclude codes from {} where {}", self.system, filter_str)
        } else {
            format!("exclude codes from {}", self.system)
        }
    }

    fn rule_type(&self) -> &'static str {
        "ExcludeRule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinality_rule() {
        let rule = CardinalityRule {
            path: "identifier".to_string(),
            min: 0,
            max: "1".to_string(),
        };
        assert_eq!(rule.to_fsh(), "identifier 0..1");
    }

    #[test]
    fn test_assignment_rule() {
        let rule = AssignmentRule {
            path: "status".to_string(),
            value: FshValue::Code(super::super::FshCode {
                system: None,
                code: "active".to_string(),
            }),
            exactly: false,
        };
        assert_eq!(rule.to_fsh(), "status = #active");
    }

    #[test]
    fn test_binding_rule() {
        let rule = BindingRule {
            path: "gender".to_string(),
            value_set: "http://hl7.org/fhir/ValueSet/administrative-gender".to_string(),
            strength: BindingStrength::Required,
        };
        assert_eq!(
            rule.to_fsh(),
            "gender from http://hl7.org/fhir/ValueSet/administrative-gender (required)"
        );
    }

    #[test]
    fn test_type_rule() {
        let rule = TypeRule {
            path: "value[x]".to_string(),
            types: vec![TypeReference {
                type_name: "CodeableConcept".to_string(),
                profiles: vec![],
                target_profiles: vec![],
            }],
        };
        assert_eq!(rule.to_fsh(), "value[x] only CodeableConcept");
    }

    #[test]
    fn test_flag_rule() {
        let rule = FlagRule {
            path: "identifier".to_string(),
            flags: vec![Flag::MustSupport, Flag::Summary],
        };
        assert_eq!(rule.to_fsh(), "identifier MS SU");
    }

    #[test]
    fn test_contains_rule() {
        let rule = ContainsRule {
            path: "extension".to_string(),
            items: vec![ContainsItem {
                name: "race".to_string(),
                type_name: Some(
                    "http://hl7.org/fhir/us/core/StructureDefinition/us-core-race".to_string(),
                ),
                min: 0,
                max: "1".to_string(),
            }],
        };
        assert!(rule.to_fsh().contains("extension contains"));
        assert!(rule.to_fsh().contains("race 0..1"));
    }

    #[test]
    fn test_obeys_rule() {
        let rule = ObeysRule {
            path: Some("identifier".to_string()),
            invariant: "us-core-1".to_string(),
        };
        assert_eq!(rule.to_fsh(), "identifier obeys us-core-1");
    }

    #[test]
    fn test_caret_value_rule() {
        let rule = CaretValueRule {
            path: None,
            caret_path: "experimental".to_string(),
            value: FshValue::Boolean(true),
        };
        assert_eq!(rule.to_fsh(), "^experimental = true");
    }

    #[test]
    fn test_local_code_rule() {
        let rule = LocalCodeRule {
            code: "example".to_string(),
            display: Some("Example Code".to_string()),
            definition: Some("This is an example".to_string()),
        };
        assert_eq!(
            rule.to_fsh(),
            "#example \"Example Code\" \"This is an example\""
        );
    }
}
