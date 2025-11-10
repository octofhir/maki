//! CodeSystem model for FHIR terminology

use serde::{Deserialize, Serialize};
use super::common::ContactDetail;

/// FHIR CodeSystem resource
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSystem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>, // Optional for tagged enum deserialization
    pub id: Option<String>,
    pub url: String,
    pub name: String,
    pub title: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub content: String,
    pub concept: Option<Vec<ConceptDefinition>>,

    // Additional metadata
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub contact: Option<Vec<ContactDetail>>,
}

/// Code concept definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConceptDefinition {
    pub code: String,
    pub display: Option<String>,
    pub definition: Option<String>,
    pub concept: Option<Vec<ConceptDefinition>>, // For hierarchical codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_code_system() {
        let json = r#"{
            "resourceType": "CodeSystem",
            "url": "http://example.org/CodeSystem/example",
            "name": "ExampleCS",
            "status": "active",
            "content": "complete",
            "concept": [
                {
                    "code": "code1",
                    "display": "Display 1",
                    "definition": "Definition 1"
                },
                {
                    "code": "code2",
                    "display": "Display 2"
                }
            ]
        }"#;

        let cs: CodeSystem = serde_json::from_str(json).unwrap();
        assert_eq!(cs.name, "ExampleCS");
        assert_eq!(cs.content, "complete");
        assert!(cs.concept.is_some());

        let concepts = cs.concept.unwrap();
        assert_eq!(concepts.len(), 2);
        assert_eq!(concepts[0].code, "code1");
        assert_eq!(concepts[0].display, Some("Display 1".to_string()));
        assert_eq!(concepts[0].definition, Some("Definition 1".to_string()));
    }
}
