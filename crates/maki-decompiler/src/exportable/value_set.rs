//! Exportable ValueSet type

use super::{Exportable, ExportableRule, escape_string, format_multiline_string};

/// Exportable ValueSet
#[derive(Debug)]
pub struct ExportableValueSet {
    pub name: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub rules: Vec<Box<dyn ExportableRule>>,
}

impl Exportable for ExportableValueSet {
    fn to_fsh(&self) -> String {
        let mut fsh = String::new();

        // ValueSet declaration
        fsh.push_str(&format!("ValueSet: {}\n", self.name));

        // Id
        if let Some(id) = &self.id {
            fsh.push_str(&format!("Id: {}\n", id));
        }

        // Title
        if let Some(title) = &self.title {
            fsh.push_str(&format!("Title: \"{}\"\n", escape_string(title)));
        }

        // Description
        if let Some(desc) = &self.description {
            if desc.contains('\n') {
                fsh.push_str(&format!("Description: {}\n", format_multiline_string(desc)));
            } else {
                fsh.push_str(&format!("Description: \"{}\"\n", escape_string(desc)));
            }
        }

        // Rules (include/exclude)
        for rule in &self.rules {
            fsh.push_str(&format!("* {}\n", rule.to_fsh()));
        }

        fsh
    }

    fn id(&self) -> &str {
        self.id.as_ref().unwrap_or(&self.name)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl ExportableValueSet {
    /// Create a new ExportableValueSet
    pub fn new(name: String) -> Self {
        Self {
            name,
            id: None,
            title: None,
            description: None,
            rules: Vec::new(),
        }
    }

    /// Add a rule to this value set
    pub fn add_rule(&mut self, rule: Box<dyn ExportableRule>) {
        self.rules.push(rule);
    }

    /// Set the id
    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the title
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::rules::*;

    #[test]
    fn test_minimal_value_set() {
        let vs = ExportableValueSet::new("MyValueSet".to_string());

        let fsh = vs.to_fsh();
        assert!(fsh.contains("ValueSet: MyValueSet"));
    }

    #[test]
    fn test_value_set_with_metadata() {
        let vs = ExportableValueSet::new("MyValueSet".to_string())
            .with_id("my-valueset".to_string())
            .with_title("My Value Set".to_string())
            .with_description("A custom value set".to_string());

        let fsh = vs.to_fsh();
        assert!(fsh.contains("Id: my-valueset"));
        assert!(fsh.contains("Title: \"My Value Set\""));
        assert!(fsh.contains("Description: \"A custom value set\""));
    }

    #[test]
    fn test_value_set_with_include_rule() {
        let mut vs = ExportableValueSet::new("MyValueSet".to_string());
        vs.add_rule(Box::new(IncludeRule {
            system: "http://snomed.info/sct".to_string(),
            version: None,
            concepts: vec![],
            filters: vec![],
        }));

        let fsh = vs.to_fsh();
        assert!(fsh.contains("* include codes from http://snomed.info/sct"));
    }

    #[test]
    fn test_value_set_with_specific_codes() {
        let mut vs = ExportableValueSet::new("MyValueSet".to_string());
        vs.add_rule(Box::new(IncludeRule {
            system: "http://example.org/cs".to_string(),
            version: None,
            concepts: vec![
                IncludeConcept {
                    code: "code1".to_string(),
                    display: Some("Code 1".to_string()),
                },
            ],
            filters: vec![],
        }));

        let fsh = vs.to_fsh();
        assert!(fsh.contains("* include"));
        assert!(fsh.contains("code1"));
    }
}
