//! ImplementationGuide Resource Generator
//!
//! This module generates FHIR ImplementationGuide resources from SUSHI
//! configuration and exported FSH definitions.
//!
//! The ImplementationGuide resource is the central resource that describes
//! the entire Implementation Guide, including metadata, dependencies,
//! resources, and pages.
//!
//! **FHIR Spec**: <http://hl7.org/fhir/R4/implementationguide.html>
//! **SUSHI Reference**: `src/ig/IGExporter.ts`

use crate::config::SushiConfiguration;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// ImplementationGuide resource generator
///
/// Converts SUSHI configuration and exported resources into a complete
/// FHIR ImplementationGuide resource ready for publication.
pub struct ImplementationGuideGenerator {
    config: SushiConfiguration,
}

impl ImplementationGuideGenerator {
    /// Effective IG id: prefer explicit id, then packageId, else fallback "ig"
    fn effective_id(&self) -> String {
        self.config
            .id
            .clone()
            .or_else(|| self.config.package_id().map(|p| p.to_string()))
            .unwrap_or_else(|| "ig".to_string())
    }

    /// Create a new IG generator from configuration
    pub fn new(config: SushiConfiguration) -> Self {
        Self { config }
    }

    /// Generate the ImplementationGuide resource
    ///
    /// Creates a complete IG resource with all metadata, resources,
    /// dependencies, and structure from the configuration.
    pub fn generate(&self) -> ImplementationGuide {
        let id = self.effective_id();

        let mut ig = ImplementationGuide {
            resource_type: "ImplementationGuide".to_string(),
            id: Some(id.clone()),
            meta: self.config.meta.clone(),
            implicit_rules: self.config.implicit_rules.clone(),
            language: self.config.language.clone(),
            text: self.config.text.clone(),
            contained: self.config.contained.clone(),
            extension: self.config.extension.clone(),
            modifier_extension: self.config.modifier_extension.clone(),
            url: self.get_ig_url(&id),
            version: self.config.version.clone(),
            name: self.sanitize_name(),
            title: self.config.title.clone(),
            status: self
                .config
                .status
                .clone()
                .unwrap_or_else(|| "draft".to_string()),
            experimental: self.config.experimental,
            date: self.config.date.clone(),
            publisher: self
                .config
                .publisher
                .as_ref()
                .and_then(|p| p.name())
                .map(|s| s.to_string()),
            contact: self.config.contact.clone(),
            description: self.config.description.clone(),
            use_context: self.config.use_context.clone(),
            jurisdiction: self.config.jurisdiction.clone(),
            copyright: self.config.copyright.clone(),
            copyright_label: self.config.copyright_label.clone(),
            version_algorithm_string: self.config.version_algorithm_string.clone(),
            version_algorithm_coding: self.config.version_algorithm_coding.clone(),
            package_id: self.config.package_id().map(String::from),
            license: self.config.license.clone(),
            fhir_version: self.config.fhir_version.clone(),
            depends_on: self.build_dependencies(),
            global: self.config.global.clone(),
            definition: self.build_definition(),
        };

        // Clean up empty arrays
        if ig.depends_on.as_ref().is_none_or(|d| d.is_empty()) {
            ig.depends_on = None;
        }

        if ig.global.as_ref().is_none_or(|g| g.is_empty()) {
            ig.global = None;
        }

        ig
    }

    /// Get the IG URL (either from config.url or constructed from canonical)
    fn get_ig_url(&self, id: &str) -> String {
        self.config
            .url
            .clone()
            .unwrap_or_else(|| format!("{}/ImplementationGuide/{}", self.config.canonical, id))
    }

    /// Sanitize the name to be alphanumeric with underscores
    fn sanitize_name(&self) -> Option<String> {
        self.config.name.as_ref().map(|name| {
            name.chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        })
    }

    /// Build dependencies array
    fn build_dependencies(&self) -> Option<Vec<DependsOn>> {
        self.config.dependencies.as_ref().map(|deps| {
            deps.iter()
                .filter_map(|(package_id, version)| {
                    // Filter out virtual extension packages
                    if package_id.starts_with("hl7.fhir.extensions.r") {
                        return None;
                    }

                    let version_str = match version {
                        crate::config::DependencyVersion::Simple(v) => v.clone(),
                        crate::config::DependencyVersion::Complex { version, uri, .. } => {
                            // For complex dependencies, we use the version and uri
                            return Some(DependsOn {
                                package_id: Some(package_id.clone()),
                                uri: uri.clone(),
                                version: Some(version.clone()),
                                extension: None,
                            });
                        }
                    };

                    Some(DependsOn {
                        package_id: Some(package_id.clone()),
                        uri: None,
                        version: Some(version_str),
                        extension: None,
                    })
                })
                .collect()
        })
    }

    /// Build the definition section
    fn build_definition(&self) -> Definition {
        let mut definition = Definition {
            extension: self
                .config
                .definition
                .as_ref()
                .and_then(|d| d.extension.clone()),
            grouping: self.build_grouping(),
            resource: vec![], // Will be populated by addResources
            page: self.build_pages(),
            parameter: self.build_parameters(),
            template: self.config.templates.clone(),
        };

        // Clean up empty grouping
        if definition.grouping.as_ref().is_none_or(|g| g.is_empty()) {
            definition.grouping = None;
        }

        definition
    }

    /// Build grouping from configured groups
    fn build_grouping(&self) -> Option<Vec<Grouping>> {
        self.config.groups.as_ref().map(|groups| {
            groups
                .iter()
                .map(|g| Grouping {
                    id: g.id.clone(),
                    name: g.name.clone(),
                    description: g.description.clone(),
                })
                .collect()
        })
    }

    /// Build pages structure
    fn build_pages(&self) -> Page {
        // Root page is always toc.html
        let mut root = Page {
            name_url: Some("toc.html".to_string()),
            name_reference: None,
            title: "Table of Contents".to_string(),
            generation: "html".to_string(),
            page: Some(vec![]),
        };

        if let Some(ref pages_config) = self.config.pages {
            root.page = Some(self.parse_pages_config(pages_config));
        }

        root
    }

    /// Parse pages configuration from sushi-config.yaml
    ///
    /// Handles the map format used by SUSHI:
    /// ```yaml
    /// pages:
    ///   index.md:
    ///     title: Home
    ///   group-patient.md:
    ///     title: Patient Information
    /// ```
    fn parse_pages_config(&self, pages: &JsonValue) -> Vec<Page> {
        let Some(pages_map) = pages.as_object() else {
            return vec![];
        };

        pages_map
            .iter()
            .filter_map(|(filename, config)| self.parse_single_page(filename, config))
            .collect()
    }

    /// Parse a single page entry
    fn parse_single_page(&self, filename: &str, config: &JsonValue) -> Option<Page> {
        // Determine generation type from file extension
        let generation = if filename.ends_with(".md") {
            "markdown"
        } else {
            "html"
        };

        // Convert filename to nameUrl (.md → .html, etc.)
        let name_url = filename_to_html(filename);

        // Get title from config or auto-generate from filename
        let title = config
            .get("title")
            .and_then(|t| t.as_str())
            .map(String::from)
            .unwrap_or_else(|| generate_title_from_filename(filename));

        // Parse nested pages if present
        let subpages = config
            .get("page")
            .and_then(|p| p.as_object())
            .map(|nested| {
                nested
                    .iter()
                    .filter_map(|(name, cfg)| self.parse_single_page(name, cfg))
                    .collect()
            });

        Some(Page {
            name_url: Some(name_url),
            name_reference: None,
            title,
            generation: generation.to_string(),
            page: subpages,
        })
    }

    /// Build parameters array
    fn build_parameters(&self) -> Option<Vec<crate::config::Parameter>> {
        // Convert HashMap to Vec<Parameter>
        let mut parameters: Vec<crate::config::Parameter> = self
            .config
            .parameters
            .as_ref()
            .map(|map| {
                map.iter()
                    .map(|(key, value)| crate::config::Parameter {
                        code: key.clone(),
                        value: value.to_string().trim_matches('"').to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Add path-history for HL7 IGs if not present
        if (self.config.canonical.starts_with("http://hl7.org/")
            || self.config.canonical.starts_with("https://hl7.org/"))
            && !parameters.iter().any(|p| p.code == "path-history")
        {
            parameters.push(crate::config::Parameter {
                code: "path-history".to_string(),
                value: format!("{}/history.html", self.config.canonical),
            });
        }

        if parameters.is_empty() {
            None
        } else {
            Some(parameters)
        }
    }

    /// Add a resource to the IG definition
    pub fn add_resource(&mut self, ig: &mut ImplementationGuide, resource: ResourceEntry) {
        ig.definition.resource.push(resource);
    }
}

/// ImplementationGuide resource
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationGuide {
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit_rules: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<JsonValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contained: Option<Vec<JsonValue>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Vec<JsonValue>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier_extension: Option<Vec<JsonValue>>,

    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    pub status: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Vec<crate::config::ContactDetail>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_context: Option<Vec<crate::config::UsageContext>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<Vec<crate::config::CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright_label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_algorithm_string: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_algorithm_coding: Option<crate::config::Coding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    pub fhir_version: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<DependsOn>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub global: Option<Vec<crate::config::GlobalProfile>>,

    pub definition: Definition,
}

/// Dependency on another IG
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DependsOn {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Vec<JsonValue>>,
}

/// IG definition section
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Vec<JsonValue>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping: Option<Vec<Grouping>>,

    pub resource: Vec<ResourceEntry>,

    pub page: Page,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<Vec<crate::config::Parameter>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Vec<crate::config::Template>>,
}

/// Resource grouping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Grouping {
    pub id: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Resource entry in IG
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceEntry {
    pub reference: Reference,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_boolean: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_canonical: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<Vec<JsonValue>>,
}

/// Resource reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reference {
    pub reference: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// Page definition for IG output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_reference: Option<Reference>,

    pub title: String,

    pub generation: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<Vec<Page>>,
}

/// Convert a page filename to HTML URL
///
/// Examples:
/// - `index.md` → `index.html`
/// - `group-patient.md` → `group-patient.html`
/// - `artifacts.html` → `artifacts.html`
fn filename_to_html(filename: &str) -> String {
    if let Some(base) = filename.strip_suffix(".md") {
        format!("{base}.html")
    } else if let Some(base) = filename.strip_suffix(".xml") {
        format!("{base}.html")
    } else {
        filename.to_string()
    }
}

/// Generate a title from a filename using title case
///
/// Examples:
/// - `group-patient.md` → `Group Patient`
/// - `conformance-general.md` → `Conformance General`
/// - `index.md` → `Index`
fn generate_title_from_filename(filename: &str) -> String {
    // Remove extension
    let base = filename
        .strip_suffix(".md")
        .or_else(|| filename.strip_suffix(".xml"))
        .or_else(|| filename.strip_suffix(".html"))
        .unwrap_or(filename);

    // Split by common separators and title-case each word
    base.split(|c| c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn minimal_config() -> SushiConfiguration {
        SushiConfiguration {
            canonical: "http://example.org/fhir/test".to_string(),
            fhir_version: vec!["4.0.1".to_string()],
            id: Some("test-ig".to_string()),
            name: Some("TestIG".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_generate_minimal_ig() {
        let config = minimal_config();
        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        assert_eq!(ig.resource_type, "ImplementationGuide");
        assert_eq!(ig.id, Some("test-ig".to_string()));
        assert_eq!(ig.name, Some("TestIG".to_string()));
        assert_eq!(ig.status, "draft");
        assert_eq!(ig.fhir_version, vec!["4.0.1"]);
        assert_eq!(
            ig.url,
            "http://example.org/fhir/test/ImplementationGuide/test-ig"
        );
    }

    #[test]
    fn test_sanitize_name() {
        let mut config = minimal_config();
        config.name = Some("Test-IG v1.0".to_string());

        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        assert_eq!(ig.name, Some("TestIGv10".to_string()));
    }

    #[test]
    fn test_custom_url() {
        let mut config = minimal_config();
        config.url = Some("http://custom.org/IG/test".to_string());

        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        assert_eq!(ig.url, "http://custom.org/IG/test");
    }

    #[test]
    fn test_dependencies() {
        let mut config = minimal_config();
        let mut deps = HashMap::new();
        deps.insert(
            "hl7.fhir.us.core".to_string(),
            crate::config::DependencyVersion::Simple("5.0.1".to_string()),
        );
        deps.insert(
            "hl7.fhir.extensions.r4".to_string(), // Should be filtered out
            crate::config::DependencyVersion::Simple("1.0.0".to_string()),
        );
        config.dependencies = Some(deps);

        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        let depends_on = ig.depends_on.unwrap();
        assert_eq!(depends_on.len(), 1);
        assert_eq!(
            depends_on[0].package_id,
            Some("hl7.fhir.us.core".to_string())
        );
        assert_eq!(depends_on[0].version, Some("5.0.1".to_string()));
    }

    #[test]
    fn test_path_history_hl7() {
        let mut config = minimal_config();
        config.canonical = "https://hl7.org/fhir/us/example".to_string();

        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        let params = ig.definition.parameter.unwrap();
        assert!(params.iter().any(|p| p.code == "path-history"));
        assert_eq!(
            params
                .iter()
                .find(|p| p.code == "path-history")
                .map(|p| &p.value),
            Some(&"https://hl7.org/fhir/us/example/history.html".to_string())
        );
    }

    #[test]
    fn test_no_path_history_non_hl7() {
        let config = minimal_config(); // Uses http://example.org

        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        if let Some(params) = ig.definition.parameter {
            assert!(!params.iter().any(|p| p.code == "path-history"));
        }
    }

    #[test]
    fn test_grouping() {
        let mut config = minimal_config();
        config.groups = Some(vec![crate::config::ResourceGroup {
            id: "profiles".to_string(),
            name: "Profiles".to_string(),
            description: Some("FHIR Profiles".to_string()),
            resources: None,
        }]);

        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        let grouping = ig.definition.grouping.unwrap();
        assert_eq!(grouping.len(), 1);
        assert_eq!(grouping[0].id, "profiles");
        assert_eq!(grouping[0].name, "Profiles");
    }

    #[test]
    fn test_filename_to_html() {
        assert_eq!(filename_to_html("index.md"), "index.html");
        assert_eq!(filename_to_html("group-patient.md"), "group-patient.html");
        assert_eq!(filename_to_html("artifacts.html"), "artifacts.html");
        assert_eq!(filename_to_html("page.xml"), "page.html");
    }

    #[test]
    fn test_generate_title_from_filename() {
        assert_eq!(generate_title_from_filename("index.md"), "Index");
        assert_eq!(
            generate_title_from_filename("group-patient.md"),
            "Group Patient"
        );
        assert_eq!(
            generate_title_from_filename("conformance-general.md"),
            "Conformance General"
        );
        assert_eq!(
            generate_title_from_filename("some_file_name.html"),
            "Some File Name"
        );
    }

    #[test]
    fn test_pages_parsing() {
        let mut config = minimal_config();
        config.pages = Some(serde_json::json!({
            "index.md": {
                "title": "Home"
            },
            "group-patient.md": {
                "title": "Patient Information"
            },
            "artifacts.html": {
                "title": "Artifacts Summary"
            }
        }));

        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        let pages = ig.definition.page.page.unwrap();
        assert_eq!(pages.len(), 3);

        // Find the index page
        let index_page = pages.iter().find(|p| p.name_url == Some("index.html".to_string()));
        assert!(index_page.is_some());
        let index_page = index_page.unwrap();
        assert_eq!(index_page.title, "Home");
        assert_eq!(index_page.generation, "markdown");

        // Find the artifacts page (already .html)
        let artifacts_page = pages
            .iter()
            .find(|p| p.name_url == Some("artifacts.html".to_string()));
        assert!(artifacts_page.is_some());
        let artifacts_page = artifacts_page.unwrap();
        assert_eq!(artifacts_page.title, "Artifacts Summary");
        assert_eq!(artifacts_page.generation, "html");
    }

    #[test]
    fn test_pages_auto_title() {
        let mut config = minimal_config();
        config.pages = Some(serde_json::json!({
            "group-patient.md": {}
        }));

        let generator = ImplementationGuideGenerator::new(config);
        let ig = generator.generate();

        let pages = ig.definition.page.page.unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Group Patient");
    }
}
