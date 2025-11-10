//! ValueSet model for FHIR terminology

use serde::{Deserialize, Serialize};
use super::common::ContactDetail;

/// FHIR ValueSet resource
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>, // Optional for tagged enum deserialization
    pub id: Option<String>,
    pub url: String,
    pub name: String,
    pub title: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub compose: Option<ValueSetCompose>,
    pub expansion: Option<ValueSetExpansion>,

    // Additional metadata
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub contact: Option<Vec<ContactDetail>>,
}

/// ValueSet compose element
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValueSetCompose {
    pub include: Vec<ValueSetInclude>,
    pub exclude: Option<Vec<ValueSetInclude>>,
}

/// ValueSet include/exclude element
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueSetInclude {
    pub system: Option<String>,
    pub version: Option<String>,
    pub concept: Option<Vec<ValueSetConcept>>,
    pub filter: Option<Vec<ValueSetFilter>>,
    pub value_set: Option<Vec<String>>,
}

/// ValueSet concept
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValueSetConcept {
    pub code: String,
    pub display: Option<String>,
}

/// ValueSet filter
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValueSetFilter {
    pub property: String,
    pub op: String,
    pub value: String,
}

/// ValueSet expansion (for pre-expanded value sets)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValueSetExpansion {
    pub contains: Option<Vec<ValueSetExpansionContains>>,
}

/// ValueSet expansion contains
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValueSetExpansionContains {
    pub system: Option<String>,
    pub code: Option<String>,
    pub display: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_value_set() {
        let json = r#"{
            "resourceType": "ValueSet",
            "url": "http://example.org/ValueSet/example",
            "name": "ExampleVS",
            "status": "active",
            "compose": {
                "include": [
                    {
                        "system": "http://example.org/CodeSystem/example",
                        "concept": [
                            {
                                "code": "code1",
                                "display": "Display 1"
                            }
                        ]
                    }
                ]
            }
        }"#;

        let vs: ValueSet = serde_json::from_str(json).unwrap();
        assert_eq!(vs.name, "ExampleVS");
        assert!(vs.compose.is_some());

        let compose = vs.compose.unwrap();
        assert_eq!(compose.include.len(), 1);

        let include = &compose.include[0];
        assert_eq!(
            include.system,
            Some("http://example.org/CodeSystem/example".to_string())
        );
        assert!(include.concept.is_some());

        let concepts = include.concept.as_ref().unwrap();
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].code, "code1");
    }
}
