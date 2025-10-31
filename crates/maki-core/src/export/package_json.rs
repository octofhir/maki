//! NPM package.json generator for FHIR IGs
//!
//! Generates package.json files for FHIR Implementation Guides
//! following the FHIR NPM package specification.
//!
//! **Reference**: <https://confluence.hl7.org/display/FHIR/NPM+Package+Specification>

use crate::config::SushiConfiguration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FHIR NPM package metadata
///
/// This structure represents the package.json file that describes
/// a FHIR IG package for distribution via NPM registries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    /// Package name (NPM package identifier)
    pub name: String,

    /// Package version (semver)
    pub version: String,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Author information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// License identifier (SPDX)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Package type (always "fhir.ig" for FHIR IGs)
    #[serde(rename = "type")]
    pub package_type: String,

    /// Canonical URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,

    /// Homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Package title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Package dependencies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,

    /// Dev dependencies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<HashMap<String, String>>,

    /// Keywords for NPM search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Maintainers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintainers: Option<Vec<Maintainer>>,

    /// Repository information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Repository>,

    /// FHIR-specific metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fhir_version_list: Option<Vec<String>>,

    /// Jurisdiction codes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<String>,

    /// Additional custom fields
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

/// Maintainer information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Maintainer {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Repository {
    #[serde(rename = "type")]
    pub repo_type: String,

    pub url: String,
}

impl PackageJson {
    /// Create a package.json from SUSHI configuration
    pub fn from_sushi_config(config: &SushiConfiguration) -> Self {
        let mut pkg = PackageJson {
            name: config.package_id().unwrap_or("unknown").to_string(),
            version: config
                .version
                .clone()
                .unwrap_or_else(|| "0.1.0".to_string()),
            description: config.description.clone(),
            author: config
                .publisher
                .as_ref()
                .and_then(|p| p.name())
                .map(|s| s.to_string()),
            license: config.license.clone(),
            package_type: "fhir.ig".to_string(),
            canonical: Some(config.canonical.clone()),
            url: config.url.clone(),
            title: config.title.clone(),
            dependencies: None,
            dev_dependencies: None,
            keywords: None,
            maintainers: None,
            repository: None,
            fhir_version_list: Some(config.fhir_version.clone()),
            jurisdiction: None,
            additional: HashMap::new(),
        };

        // Convert SUSHI dependencies to NPM dependencies
        if let Some(ref deps) = config.dependencies {
            let mut npm_deps = HashMap::new();
            for (package_id, version) in deps {
                let version_str = match version {
                    crate::config::DependencyVersion::Simple(v) => v.clone(),
                    crate::config::DependencyVersion::Complex { version, .. } => version.clone(),
                };
                npm_deps.insert(package_id.clone(), version_str);
            }
            if !npm_deps.is_empty() {
                pkg.dependencies = Some(npm_deps);
            }
        }

        // Convert contact to maintainers
        if let Some(ref contacts) = config.contact {
            let maintainers: Vec<Maintainer> = contacts
                .iter()
                .filter_map(|contact| {
                    contact.name.as_ref().map(|name| {
                        let email = contact.telecom.as_ref().and_then(|telecom| {
                            telecom
                                .iter()
                                .find(|t| t.system == "email")
                                .map(|t| t.value.clone())
                        });

                        let url = contact.telecom.as_ref().and_then(|telecom| {
                            telecom
                                .iter()
                                .find(|t| t.system == "url")
                                .map(|t| t.value.clone())
                        });

                        Maintainer {
                            name: name.clone(),
                            email,
                            url,
                        }
                    })
                })
                .collect();

            if !maintainers.is_empty() {
                pkg.maintainers = Some(maintainers);
            }
        }

        // Add FHIR keyword
        pkg.keywords = Some(vec!["fhir".to_string(), "fhir-ig".to_string()]);

        // Extract jurisdiction if present
        if let Some(ref jurisdictions) = config.jurisdiction
            && let Some(first_jurisdiction) = jurisdictions.first()
            && let Some(ref coding) = first_jurisdiction.coding
            && let Some(first_code) = coding.first()
            && let Some(ref code) = first_code.code
        {
            pkg.jurisdiction = Some(code.clone());
        }

        pkg
    }

    /// Write package.json to a JSON string
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Write package.json to a file
    pub fn write_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = self
            .to_json_string()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ContactDetail, ContactPoint, DependencyVersion, PublisherInfo};

    #[test]
    fn test_package_json_from_minimal_config() {
        let config = SushiConfiguration {
            canonical: "http://example.org/fhir/test".to_string(),
            fhir_version: vec!["4.0.1".to_string()],
            id: Some("example.test".to_string()),
            name: Some("ExampleTest".to_string()),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };

        let pkg = PackageJson::from_sushi_config(&config);

        assert_eq!(pkg.name, "example.test");
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.package_type, "fhir.ig");
        assert_eq!(
            pkg.canonical,
            Some("http://example.org/fhir/test".to_string())
        );
        assert_eq!(pkg.fhir_version_list, Some(vec!["4.0.1".to_string()]));
    }

    #[test]
    fn test_package_json_with_full_metadata() {
        let config = SushiConfiguration {
            canonical: "http://example.org/fhir/test".to_string(),
            fhir_version: vec!["4.0.1".to_string()],
            id: Some("example.test".to_string()),
            name: Some("ExampleTest".to_string()),
            title: Some("Example Test IG".to_string()),
            version: Some("1.0.0".to_string()),
            publisher: Some(PublisherInfo::String("Example Org".to_string())),
            description: Some("An example IG".to_string()),
            license: Some("Apache-2.0".to_string()),
            contact: Some(vec![ContactDetail {
                name: Some("Example Contact".to_string()),
                telecom: Some(vec![
                    ContactPoint {
                        system: "email".to_string(),
                        value: "contact@example.org".to_string(),
                        use_field: None,
                        rank: None,
                        period: None,
                    },
                    ContactPoint {
                        system: "url".to_string(),
                        value: "http://example.org".to_string(),
                        use_field: None,
                        rank: None,
                        period: None,
                    },
                ]),
            }]),
            dependencies: Some({
                let mut deps = HashMap::new();
                deps.insert(
                    "hl7.fhir.us.core".to_string(),
                    DependencyVersion::Simple("5.0.1".to_string()),
                );
                deps
            }),
            ..Default::default()
        };

        let pkg = PackageJson::from_sushi_config(&config);

        assert_eq!(pkg.name, "example.test");
        assert_eq!(pkg.title, Some("Example Test IG".to_string()));
        assert_eq!(pkg.author, Some("Example Org".to_string()));
        assert_eq!(pkg.description, Some("An example IG".to_string()));
        assert_eq!(pkg.license, Some("Apache-2.0".to_string()));

        // Check dependencies
        let deps = pkg.dependencies.unwrap();
        assert_eq!(deps.get("hl7.fhir.us.core"), Some(&"5.0.1".to_string()));

        // Check maintainers
        let maintainers = pkg.maintainers.unwrap();
        assert_eq!(maintainers.len(), 1);
        assert_eq!(maintainers[0].name, "Example Contact");
        assert_eq!(
            maintainers[0].email,
            Some("contact@example.org".to_string())
        );
        assert_eq!(maintainers[0].url, Some("http://example.org".to_string()));

        // Check keywords
        assert!(pkg.keywords.unwrap().contains(&"fhir".to_string()));
    }

    #[test]
    fn test_package_json_serialization() {
        let pkg = PackageJson {
            name: "test.package".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test package".to_string()),
            author: None,
            license: Some("MIT".to_string()),
            package_type: "fhir.ig".to_string(),
            canonical: Some("http://example.org/fhir/test".to_string()),
            url: None,
            title: None,
            dependencies: None,
            dev_dependencies: None,
            keywords: Some(vec!["fhir".to_string()]),
            maintainers: None,
            repository: None,
            fhir_version_list: Some(vec!["4.0.1".to_string()]),
            jurisdiction: None,
            additional: HashMap::new(),
        };

        let json = pkg.to_json_string().unwrap();

        assert!(json.contains("\"name\": \"test.package\""));
        assert!(json.contains("\"version\": \"1.0.0\""));
        assert!(json.contains("\"type\": \"fhir.ig\""));
        assert!(json.contains("\"fhirVersionList\""));
    }

    #[test]
    fn test_default_values() {
        let config = SushiConfiguration {
            canonical: "http://example.org/fhir/test".to_string(),
            fhir_version: vec!["4.0.1".to_string()],
            // No id, version, etc.
            ..Default::default()
        };

        let pkg = PackageJson::from_sushi_config(&config);

        // Should use fallback values
        assert_eq!(pkg.name, "unknown"); // Falls back when no id/packageId
        assert_eq!(pkg.version, "0.1.0"); // Default version
    }
}

// Need to implement Default for SushiConfiguration for tests
#[cfg(test)]
impl Default for SushiConfiguration {
    fn default() -> Self {
        Self {
            canonical: String::new(),
            fhir_version: Vec::new(),
            id: None,
            name: None,
            title: None,
            version: None,
            status: None,
            experimental: None,
            date: None,
            publisher: None,
            contact: None,
            description: None,
            use_context: None,
            jurisdiction: None,
            copyright: None,
            copyright_label: None,
            version_algorithm_string: None,
            version_algorithm_coding: None,
            package_id: None,
            license: None,
            dependencies: None,
            global: None,
            groups: None,
            resources: None,
            pages: None,
            index_page_content: None,
            parameters: None,
            templates: None,
            menu: None,
            fsh_only: None,
            apply_extension_metadata_to_root: None,
            instance_options: None,
            meta: None,
            implicit_rules: None,
            language: None,
            text: None,
            contained: None,
            extension: None,
            modifier_extension: None,
            url: None,
            definition: None,
        }
    }
}
