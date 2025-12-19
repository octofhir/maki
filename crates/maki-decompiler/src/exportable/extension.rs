//! Exportable Extension type

use super::{Exportable, ExportableRule, escape_string, format_multiline_string};

/// Extension context type
#[derive(Debug, Clone, PartialEq)]
pub enum ContextType {
    Element,
    Extension,
    Fhirpath,
}

impl ContextType {
    pub fn to_fsh(&self) -> &'static str {
        match self {
            ContextType::Element => "element",
            ContextType::Extension => "extension",
            ContextType::Fhirpath => "fhirpath",
        }
    }
}

/// Extension context
#[derive(Debug, Clone)]
pub struct Context {
    pub type_: ContextType,
    pub expression: String,
}

impl Context {
    pub fn to_fsh(&self) -> String {
        format!("{} = {}", self.type_.to_fsh(), self.expression)
    }
}

/// Exportable Extension
#[derive(Debug)]
pub struct ExportableExtension {
    pub name: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub contexts: Vec<Context>,
    pub rules: Vec<Box<dyn ExportableRule + Send + Sync>>,
}

impl Exportable for ExportableExtension {
    fn to_fsh(&self) -> String {
        let mut fsh = String::new();

        // Extension declaration
        fsh.push_str(&format!("Extension: {}\n", self.name));

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

        // Contexts
        for context in &self.contexts {
            fsh.push_str(&format!("Context: {}\n", context.to_fsh()));
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

impl ExportableExtension {
    /// Create a new ExportableExtension
    pub fn new(name: String) -> Self {
        Self {
            name,
            id: None,
            title: None,
            description: None,
            contexts: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Add a rule to this extension
    pub fn add_rule(&mut self, rule: Box<dyn ExportableRule + Send + Sync>) {
        self.rules.push(rule);
    }

    /// Add a context to this extension
    pub fn add_context(&mut self, context: Context) {
        self.contexts.push(context);
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
    fn test_minimal_extension() {
        let extension = ExportableExtension::new("MyExtension".to_string());

        let fsh = extension.to_fsh();
        assert!(fsh.contains("Extension: MyExtension"));
    }

    #[test]
    fn test_extension_with_metadata() {
        let extension = ExportableExtension::new("MyExtension".to_string())
            .with_id("my-extension".to_string())
            .with_title("My Extension".to_string())
            .with_description("A custom extension".to_string());

        let fsh = extension.to_fsh();
        assert!(fsh.contains("Id: my-extension"));
        assert!(fsh.contains("Title: \"My Extension\""));
        assert!(fsh.contains("Description: \"A custom extension\""));
    }

    #[test]
    fn test_extension_with_context() {
        let mut extension = ExportableExtension::new("MyExtension".to_string());
        extension.add_context(Context {
            type_: ContextType::Element,
            expression: "Patient".to_string(),
        });

        let fsh = extension.to_fsh();
        assert!(fsh.contains("Context: element = Patient"));
    }

    #[test]
    fn test_extension_with_rules() {
        let mut extension = ExportableExtension::new("MyExtension".to_string());
        extension.add_rule(Box::new(CardinalityRule {
            path: "value[x]".to_string(),
            min: 0,
            max: "1".to_string(),
        }));

        let fsh = extension.to_fsh();
        assert!(fsh.contains("* value[x] 0..1"));
    }
}
