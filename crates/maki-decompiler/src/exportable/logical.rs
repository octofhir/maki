//! Exportable Logical Model type

use super::{Exportable, ExportableRule, escape_string, format_multiline_string};

/// Exportable Logical Model
#[derive(Debug)]
pub struct ExportableLogical {
    pub name: String,
    pub id: Option<String>,
    pub parent: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub characteristics: Vec<String>,
    pub rules: Vec<Box<dyn ExportableRule + Send + Sync>>,
}

impl Exportable for ExportableLogical {
    fn to_fsh(&self) -> String {
        let mut fsh = String::new();

        // Logical declaration
        fsh.push_str(&format!("Logical: {}\n", self.name));

        // Parent (optional for logical models)
        if let Some(parent) = &self.parent {
            fsh.push_str(&format!("Parent: {}\n", parent));
        }

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

        // Characteristics
        for characteristic in &self.characteristics {
            fsh.push_str(&format!("Characteristics: {}\n", characteristic));
        }

        // Rules
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

    fn get_rules_mut(&mut self) -> &mut Vec<Box<dyn ExportableRule + Send + Sync>> {
        &mut self.rules
    }
}

impl ExportableLogical {
    /// Create a new ExportableLogical
    pub fn new(name: String) -> Self {
        Self {
            name,
            id: None,
            parent: None,
            title: None,
            description: None,
            characteristics: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Add a rule to this logical model
    pub fn add_rule(&mut self, rule: Box<dyn ExportableRule + Send + Sync>) {
        self.rules.push(rule);
    }

    /// Add a characteristic
    pub fn add_characteristic(&mut self, characteristic: String) {
        self.characteristics.push(characteristic);
    }

    /// Set the id
    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the parent
    pub fn with_parent(mut self, parent: String) -> Self {
        self.parent = Some(parent);
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
    fn test_minimal_logical() {
        let logical = ExportableLogical::new("MyModel".to_string());

        let fsh = logical.to_fsh();
        assert!(fsh.contains("Logical: MyModel"));
    }

    #[test]
    fn test_logical_with_metadata() {
        let logical = ExportableLogical::new("MyModel".to_string())
            .with_id("my-model".to_string())
            .with_title("My Logical Model".to_string())
            .with_description("A custom logical model".to_string());

        let fsh = logical.to_fsh();
        assert!(fsh.contains("Id: my-model"));
        assert!(fsh.contains("Title: \"My Logical Model\""));
        assert!(fsh.contains("Description: \"A custom logical model\""));
    }

    #[test]
    fn test_logical_with_characteristics() {
        let mut logical = ExportableLogical::new("MyModel".to_string());
        logical.add_characteristic("#can-be-target".to_string());

        let fsh = logical.to_fsh();
        assert!(fsh.contains("Characteristics: #can-be-target"));
    }

    #[test]
    fn test_logical_with_rules() {
        let mut logical = ExportableLogical::new("MyModel".to_string());
        logical.add_rule(Box::new(AddElementRule {
            path: "identifier".to_string(),
            min: 0,
            max: "*".to_string(),
            types: vec!["Identifier".to_string()],
            short: Some("Identifier".to_string()),
            definition: None,
            flags: vec![],
        }));

        let fsh = logical.to_fsh();
        assert!(fsh.contains("* identifier 0..* Identifier"));
    }
}
