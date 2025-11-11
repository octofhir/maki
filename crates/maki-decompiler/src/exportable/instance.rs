//! Exportable Instance type

use super::{Exportable, ExportableRule, escape_string, format_multiline_string};

/// Instance usage type
#[derive(Debug, Clone, PartialEq)]
pub enum InstanceUsage {
    Example,
    Definition,
    Inline,
}

impl InstanceUsage {
    pub fn to_fsh(&self) -> &'static str {
        match self {
            InstanceUsage::Example => "#example",
            InstanceUsage::Definition => "#definition",
            InstanceUsage::Inline => "#inline",
        }
    }
}

/// Exportable Instance
#[derive(Debug)]
pub struct ExportableInstance {
    pub name: String,
    pub instance_of: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub usage: Option<InstanceUsage>,
    pub rules: Vec<Box<dyn ExportableRule>>,
}

impl Exportable for ExportableInstance {
    fn to_fsh(&self) -> String {
        let mut fsh = String::new();

        // Instance declaration
        fsh.push_str(&format!("Instance: {}\n", self.name));
        fsh.push_str(&format!("InstanceOf: {}\n", self.instance_of));

        // Usage
        if let Some(usage) = &self.usage {
            fsh.push_str(&format!("Usage: {}\n", usage.to_fsh()));
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

        // Rules (assignments)
        for rule in &self.rules {
            fsh.push_str(&format!("* {}\n", rule.to_fsh()));
        }

        fsh
    }

    fn id(&self) -> &str {
        &self.name
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl ExportableInstance {
    /// Create a new ExportableInstance
    pub fn new(name: String, instance_of: String) -> Self {
        Self {
            name,
            instance_of,
            title: None,
            description: None,
            usage: None,
            rules: Vec::new(),
        }
    }

    /// Add a rule to this instance
    pub fn add_rule(&mut self, rule: Box<dyn ExportableRule>) {
        self.rules.push(rule);
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

    /// Set the usage
    pub fn with_usage(mut self, usage: InstanceUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::rules::*;
    use crate::exportable::{FshValue, FshCode};

    #[test]
    fn test_minimal_instance() {
        let instance = ExportableInstance::new(
            "example-patient".to_string(),
            "Patient".to_string(),
        );

        let fsh = instance.to_fsh();
        assert!(fsh.contains("Instance: example-patient"));
        assert!(fsh.contains("InstanceOf: Patient"));
    }

    #[test]
    fn test_instance_with_metadata() {
        let instance = ExportableInstance::new(
            "example-patient".to_string(),
            "Patient".to_string(),
        )
        .with_title("Example Patient".to_string())
        .with_description("An example patient instance".to_string())
        .with_usage(InstanceUsage::Example);

        let fsh = instance.to_fsh();
        assert!(fsh.contains("Usage: #example"));
        assert!(fsh.contains("Title: \"Example Patient\""));
        assert!(fsh.contains("Description: \"An example patient instance\""));
    }

    #[test]
    fn test_instance_with_assignments() {
        let mut instance = ExportableInstance::new(
            "example-patient".to_string(),
            "Patient".to_string(),
        );

        instance.add_rule(Box::new(AssignmentRule {
            path: "active".to_string(),
            value: FshValue::Boolean(true),
            exactly: false,
        }));

        instance.add_rule(Box::new(AssignmentRule {
            path: "gender".to_string(),
            value: FshValue::Code(FshCode {
                system: None,
                code: "female".to_string(),
            }),
            exactly: false,
        }));

        let fsh = instance.to_fsh();
        assert!(fsh.contains("* active = true"));
        assert!(fsh.contains("* gender = #female"));
    }
}
