//! Exportable Configuration (sushi-config.yaml equivalent)

/// Configuration for FSH project (maps to sushi-config.yaml)
#[derive(Debug, Clone)]
pub struct ExportableConfiguration {
    pub id: String,
    pub canonical: String,
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub version: String,
    pub fhir_version: Vec<String>,
    pub dependencies: Vec<FhirDependency>,
    pub parameters: Vec<ConfigParameter>,
    pub pages: Vec<PageDefinition>,
}

#[derive(Debug, Clone)]
pub struct FhirDependency {
    pub package_id: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ConfigParameter {
    pub code: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct PageDefinition {
    pub name: String,
    pub title: String,
    pub generation: String,
}

impl ExportableConfiguration {
    /// Create a new ExportableConfiguration
    pub fn new(id: String, canonical: String, name: String, version: String) -> Self {
        Self {
            id,
            canonical,
            name,
            title: None,
            description: None,
            status: "draft".to_string(),
            version,
            fhir_version: vec!["4.0.1".to_string()],
            dependencies: Vec::new(),
            parameters: Vec::new(),
            pages: Vec::new(),
        }
    }

    /// Convert to YAML format for sushi-config.yaml
    pub fn to_yaml(&self) -> String {
        let mut yaml = String::new();

        yaml.push_str(&format!("id: {}\n", self.id));
        yaml.push_str(&format!("canonical: {}\n", self.canonical));
        yaml.push_str(&format!("name: {}\n", self.name));

        if let Some(title) = &self.title {
            yaml.push_str(&format!("title: \"{}\"\n", title));
        }

        if let Some(desc) = &self.description {
            yaml.push_str(&format!("description: \"{}\"\n", desc));
        }

        yaml.push_str(&format!("status: {}\n", self.status));
        yaml.push_str(&format!("version: {}\n", self.version));

        // FHIR version
        yaml.push_str("fhirVersion:\n");
        for version in &self.fhir_version {
            yaml.push_str(&format!("  - {}\n", version));
        }

        // Dependencies
        if !self.dependencies.is_empty() {
            yaml.push_str("dependencies:\n");
            for dep in &self.dependencies {
                yaml.push_str(&format!("  {}:\n", dep.package_id));
                yaml.push_str(&format!("    version: {}\n", dep.version));
            }
        }

        // Parameters
        if !self.parameters.is_empty() {
            yaml.push_str("parameters:\n");
            for param in &self.parameters {
                yaml.push_str(&format!("  {}: {}\n", param.code, param.value));
            }
        }

        // Pages
        if !self.pages.is_empty() {
            yaml.push_str("pages:\n");
            for page in &self.pages {
                yaml.push_str(&format!("  {}:\n", page.name));
                yaml.push_str(&format!("    title: {}\n", page.title));
                yaml.push_str(&format!("    generation: {}\n", page.generation));
            }
        }

        yaml
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, package_id: String, version: String) {
        self.dependencies.push(FhirDependency {
            package_id,
            version,
        });
    }

    /// Add a parameter
    pub fn add_parameter(&mut self, code: String, value: String) {
        self.parameters.push(ConfigParameter { code, value });
    }

    /// Add a page
    pub fn add_page(&mut self, name: String, title: String, generation: String) {
        self.pages.push(PageDefinition {
            name,
            title,
            generation,
        });
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

    /// Set the status
    pub fn with_status(mut self, status: String) -> Self {
        self.status = status;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_config() {
        let config = ExportableConfiguration::new(
            "my.example.ig".to_string(),
            "http://example.org/fhir".to_string(),
            "MyIG".to_string(),
            "1.0.0".to_string(),
        );

        let yaml = config.to_yaml();
        assert!(yaml.contains("id: my.example.ig"));
        assert!(yaml.contains("canonical: http://example.org/fhir"));
        assert!(yaml.contains("name: MyIG"));
        assert!(yaml.contains("version: 1.0.0"));
    }

    #[test]
    fn test_config_with_metadata() {
        let config = ExportableConfiguration::new(
            "my.example.ig".to_string(),
            "http://example.org/fhir".to_string(),
            "MyIG".to_string(),
            "1.0.0".to_string(),
        )
        .with_title("My Implementation Guide".to_string())
        .with_description("An example IG".to_string())
        .with_status("active".to_string());

        let yaml = config.to_yaml();
        assert!(yaml.contains("title: \"My Implementation Guide\""));
        assert!(yaml.contains("description: \"An example IG\""));
        assert!(yaml.contains("status: active"));
    }

    #[test]
    fn test_config_with_dependencies() {
        let mut config = ExportableConfiguration::new(
            "my.example.ig".to_string(),
            "http://example.org/fhir".to_string(),
            "MyIG".to_string(),
            "1.0.0".to_string(),
        );

        config.add_dependency("hl7.fhir.us.core".to_string(), "5.0.1".to_string());

        let yaml = config.to_yaml();
        assert!(yaml.contains("dependencies:"));
        assert!(yaml.contains("hl7.fhir.us.core:"));
        assert!(yaml.contains("version: 5.0.1"));
    }

    #[test]
    fn test_config_with_parameters() {
        let mut config = ExportableConfiguration::new(
            "my.example.ig".to_string(),
            "http://example.org/fhir".to_string(),
            "MyIG".to_string(),
            "1.0.0".to_string(),
        );

        config.add_parameter("copyrightyear".to_string(), "2024".to_string());

        let yaml = config.to_yaml();
        assert!(yaml.contains("parameters:"));
        assert!(yaml.contains("copyrightyear: 2024"));
    }
}
