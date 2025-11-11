//! Exportable CodeSystem type

use super::{Exportable, ExportableRule, escape_string, format_multiline_string};

/// Exportable CodeSystem
#[derive(Debug)]
pub struct ExportableCodeSystem {
    pub name: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub rules: Vec<Box<dyn ExportableRule>>,
}

impl Exportable for ExportableCodeSystem {
    fn to_fsh(&self) -> String {
        let mut fsh = String::new();

        // CodeSystem declaration
        fsh.push_str(&format!("CodeSystem: {}\n", self.name));

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

        // Rules (local codes)
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

impl ExportableCodeSystem {
    /// Create a new ExportableCodeSystem
    pub fn new(name: String) -> Self {
        Self {
            name,
            id: None,
            title: None,
            description: None,
            rules: Vec::new(),
        }
    }

    /// Add a rule to this code system
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
    fn test_minimal_code_system() {
        let cs = ExportableCodeSystem::new("MyCodeSystem".to_string());

        let fsh = cs.to_fsh();
        assert!(fsh.contains("CodeSystem: MyCodeSystem"));
    }

    #[test]
    fn test_code_system_with_metadata() {
        let cs = ExportableCodeSystem::new("MyCodeSystem".to_string())
            .with_id("my-codesystem".to_string())
            .with_title("My Code System".to_string())
            .with_description("A custom code system".to_string());

        let fsh = cs.to_fsh();
        assert!(fsh.contains("Id: my-codesystem"));
        assert!(fsh.contains("Title: \"My Code System\""));
        assert!(fsh.contains("Description: \"A custom code system\""));
    }

    #[test]
    fn test_code_system_with_codes() {
        let mut cs = ExportableCodeSystem::new("MyCodeSystem".to_string());
        cs.add_rule(Box::new(LocalCodeRule {
            code: "active".to_string(),
            display: Some("Active".to_string()),
            definition: Some("The entity is active".to_string()),
        }));

        cs.add_rule(Box::new(LocalCodeRule {
            code: "inactive".to_string(),
            display: Some("Inactive".to_string()),
            definition: None,
        }));

        let fsh = cs.to_fsh();
        assert!(fsh.contains("* #active \"Active\" \"The entity is active\""));
        assert!(fsh.contains("* #inactive \"Inactive\""));
    }
}
