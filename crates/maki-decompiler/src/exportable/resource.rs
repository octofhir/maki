//! Exportable Resource type

use super::{Exportable, ExportableRule, escape_string, format_multiline_string};

/// Exportable Resource
#[derive(Debug)]
pub struct ExportableResource {
    pub name: String,
    pub id: Option<String>,
    pub parent: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub characteristics: Vec<String>,
    pub rules: Vec<Box<dyn ExportableRule>>,
}

impl Exportable for ExportableResource {
    fn to_fsh(&self) -> String {
        let mut fsh = String::new();

        // Resource declaration
        fsh.push_str(&format!("Resource: {}\n", self.name));

        // Parent (optional for resources)
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
}

impl ExportableResource {
    /// Create a new ExportableResource
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

    /// Add a rule to this resource
    pub fn add_rule(&mut self, rule: Box<dyn ExportableRule>) {
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
    fn test_minimal_resource() {
        let resource = ExportableResource::new("MyResource".to_string());

        let fsh = resource.to_fsh();
        assert!(fsh.contains("Resource: MyResource"));
    }

    #[test]
    fn test_resource_with_metadata() {
        let resource = ExportableResource::new("MyResource".to_string())
            .with_id("my-resource".to_string())
            .with_title("My Resource".to_string())
            .with_description("A custom resource".to_string());

        let fsh = resource.to_fsh();
        assert!(fsh.contains("Id: my-resource"));
        assert!(fsh.contains("Title: \"My Resource\""));
        assert!(fsh.contains("Description: \"A custom resource\""));
    }

    #[test]
    fn test_resource_with_parent() {
        let resource = ExportableResource::new("MyResource".to_string())
            .with_parent("DomainResource".to_string());

        let fsh = resource.to_fsh();
        assert!(fsh.contains("Parent: DomainResource"));
    }

    #[test]
    fn test_resource_with_rules() {
        let mut resource = ExportableResource::new("MyResource".to_string());
        resource.add_rule(Box::new(AddElementRule {
            path: "status".to_string(),
            min: 1,
            max: "1".to_string(),
            types: vec!["code".to_string()],
            short: Some("Status".to_string()),
            definition: None,
            flags: vec![],
        }));

        let fsh = resource.to_fsh();
        assert!(fsh.contains("* status 1..1 code"));
    }
}
