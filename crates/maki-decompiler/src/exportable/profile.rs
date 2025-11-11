//! Exportable Profile type

use super::{Exportable, ExportableRule, escape_string, format_multiline_string};

/// Exportable Profile
#[derive(Debug)]
pub struct ExportableProfile {
    pub name: String,
    pub id: Option<String>,
    pub parent: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub rules: Vec<Box<dyn ExportableRule>>,
}

impl Exportable for ExportableProfile {
    fn to_fsh(&self) -> String {
        let mut fsh = String::new();

        // Profile declaration
        fsh.push_str(&format!("Profile: {}\n", self.name));
        fsh.push_str(&format!("Parent: {}\n", self.parent));

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

impl ExportableProfile {
    /// Create a new ExportableProfile
    pub fn new(name: String, parent: String) -> Self {
        Self {
            name,
            id: None,
            parent,
            title: None,
            description: None,
            rules: Vec::new(),
        }
    }

    /// Add a rule to this profile
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
    use crate::exportable::FshValue;

    #[test]
    fn test_minimal_profile() {
        let profile = ExportableProfile::new(
            "MyPatient".to_string(),
            "Patient".to_string(),
        );

        let fsh = profile.to_fsh();
        assert!(fsh.contains("Profile: MyPatient"));
        assert!(fsh.contains("Parent: Patient"));
    }

    #[test]
    fn test_profile_with_metadata() {
        let profile = ExportableProfile::new(
            "MyPatient".to_string(),
            "Patient".to_string(),
        )
        .with_id("my-patient".to_string())
        .with_title("My Patient Profile".to_string())
        .with_description("A custom patient profile".to_string());

        let fsh = profile.to_fsh();
        assert!(fsh.contains("Id: my-patient"));
        assert!(fsh.contains("Title: \"My Patient Profile\""));
        assert!(fsh.contains("Description: \"A custom patient profile\""));
    }

    #[test]
    fn test_profile_with_rules() {
        let mut profile = ExportableProfile::new(
            "MyPatient".to_string(),
            "Patient".to_string(),
        );

        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "identifier".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        let fsh = profile.to_fsh();
        assert!(fsh.contains("* identifier 1..*"));
        assert!(fsh.contains("* identifier MS"));
    }

    #[test]
    fn test_profile_with_multiline_description() {
        let profile = ExportableProfile::new(
            "MyPatient".to_string(),
            "Patient".to_string(),
        )
        .with_description("Line 1\nLine 2\nLine 3".to_string());

        let fsh = profile.to_fsh();
        assert!(fsh.contains("Description: \"\"\"Line 1\nLine 2\nLine 3\"\"\""));
    }
}
