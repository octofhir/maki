//! CodeSystem processor
//!
//! Converts FHIR CodeSystem resources into ExportableCodeSystem objects

use crate::{
    Result,
    exportable::{Exportable, ExportableCodeSystem, LocalCodeRule},
    lake::ResourceLake,
    models::{CodeSystem, ConceptDefinition},
};
use log::debug;

/// CodeSystem processor
pub struct CodeSystemProcessor<'a> {
    _lake: &'a ResourceLake,
}

impl<'a> CodeSystemProcessor<'a> {
    /// Create a new CodeSystem processor
    pub fn new(lake: &'a ResourceLake) -> Self {
        Self { _lake: lake }
    }

    /// Process a CodeSystem into an ExportableCodeSystem
    pub fn process(&self, cs: &CodeSystem) -> Result<ExportableCodeSystem> {
        debug!("Processing CodeSystem '{}'", cs.name);

        let mut exportable = ExportableCodeSystem::new(cs.name.clone());

        // Set optional fields
        if let Some(id) = &cs.id {
            exportable.id = Some(id.clone());
        }

        if let Some(title) = &cs.title {
            exportable.title = Some(title.clone());
        }

        if let Some(desc) = &cs.description {
            exportable.description = Some(desc.clone());
        }

        // Process concepts
        if let Some(concepts) = &cs.concept {
            self.process_concepts(&mut exportable, concepts, vec![])?;
        }

        debug!(
            "Created ExportableCodeSystem '{}' with {} rules",
            exportable.name(),
            exportable.rules.len()
        );

        Ok(exportable)
    }

    /// Process concept definitions recursively
    #[allow(clippy::only_used_in_recursion)]
    fn process_concepts(
        &self,
        exportable: &mut ExportableCodeSystem,
        concepts: &[ConceptDefinition],
        parent_path: Vec<String>,
    ) -> Result<()> {
        for concept in concepts {
            // Create LocalCodeRule for this concept
            let rule = LocalCodeRule {
                code: concept.code.clone(),
                display: concept.display.clone(),
                definition: concept.definition.clone(),
            };

            exportable.add_rule(Box::new(rule));

            // Process child concepts recursively (for hierarchical code systems)
            if let Some(child_concepts) = &concept.concept {
                let mut new_path = parent_path.clone();
                new_path.push(concept.code.clone());
                self.process_concepts(exportable, child_concepts, new_path)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
    use std::sync::Arc;

    async fn create_test_lake() -> ResourceLake {
        let options = CanonicalOptions {
            quick_init: true,
            auto_install_core: false,
            ..Default::default()
        };

        let facade = CanonicalFacade::new(options).await.unwrap();
        let session = facade.session(vec![FhirRelease::R4]).await.unwrap();

        ResourceLake::new(Arc::new(session))
    }

    fn create_test_code_system() -> CodeSystem {
        CodeSystem {
            resource_type: Some("CodeSystem".to_string()),
            id: Some("example-cs".to_string()),
            url: "http://example.org/CodeSystem/example".to_string(),
            name: "ExampleCodeSystem".to_string(),
            title: Some("Example Code System".to_string()),
            status: "active".to_string(),
            description: Some("An example code system".to_string()),
            content: "complete".to_string(),
            concept: Some(vec![
                ConceptDefinition {
                    code: "code1".to_string(),
                    display: Some("Code 1".to_string()),
                    definition: Some("Definition for code 1".to_string()),
                    concept: None,
                },
                ConceptDefinition {
                    code: "code2".to_string(),
                    display: Some("Code 2".to_string()),
                    definition: Some("Definition for code 2".to_string()),
                    concept: None,
                },
                ConceptDefinition {
                    code: "code3".to_string(),
                    display: Some("Code 3".to_string()),
                    definition: None,
                    concept: None,
                },
            ]),
            version: None,
            publisher: None,
            contact: None,
        }
    }

    fn create_hierarchical_code_system() -> CodeSystem {
        CodeSystem {
            resource_type: Some("CodeSystem".to_string()),
            id: Some("hierarchical-cs".to_string()),
            url: "http://example.org/CodeSystem/hierarchical".to_string(),
            name: "HierarchicalCodeSystem".to_string(),
            title: Some("Hierarchical Code System".to_string()),
            status: "active".to_string(),
            description: Some("A hierarchical code system".to_string()),
            content: "complete".to_string(),
            concept: Some(vec![
                ConceptDefinition {
                    code: "parent1".to_string(),
                    display: Some("Parent 1".to_string()),
                    definition: Some("Parent concept 1".to_string()),
                    concept: Some(vec![
                        ConceptDefinition {
                            code: "child1-1".to_string(),
                            display: Some("Child 1-1".to_string()),
                            definition: Some("Child concept 1-1".to_string()),
                            concept: None,
                        },
                        ConceptDefinition {
                            code: "child1-2".to_string(),
                            display: Some("Child 1-2".to_string()),
                            definition: Some("Child concept 1-2".to_string()),
                            concept: None,
                        },
                    ]),
                },
                ConceptDefinition {
                    code: "parent2".to_string(),
                    display: Some("Parent 2".to_string()),
                    definition: Some("Parent concept 2".to_string()),
                    concept: None,
                },
            ]),
            version: None,
            publisher: None,
            contact: None,
        }
    }

    #[tokio::test]
    async fn test_process_code_system() {
        let lake = create_test_lake().await;
        let processor = CodeSystemProcessor::new(&lake);

        let cs = create_test_code_system();
        let exportable = processor.process(&cs).unwrap();

        assert_eq!(exportable.name(), "ExampleCodeSystem");
        assert_eq!(exportable.id, Some("example-cs".to_string()));
        assert_eq!(exportable.title, Some("Example Code System".to_string()));
        assert_eq!(
            exportable.description,
            Some("An example code system".to_string())
        );
        assert_eq!(exportable.rules.len(), 3); // 3 concepts
    }

    #[tokio::test]
    async fn test_process_hierarchical_code_system() {
        let lake = create_test_lake().await;
        let processor = CodeSystemProcessor::new(&lake);

        let cs = create_hierarchical_code_system();
        let exportable = processor.process(&cs).unwrap();

        assert_eq!(exportable.name(), "HierarchicalCodeSystem");
        assert_eq!(exportable.rules.len(), 4); // 2 parents + 2 children
    }

    #[tokio::test]
    async fn test_process_empty_code_system() {
        let lake = create_test_lake().await;
        let processor = CodeSystemProcessor::new(&lake);

        let cs = CodeSystem {
            resource_type: Some("CodeSystem".to_string()),
            id: Some("empty-cs".to_string()),
            url: "http://example.org/CodeSystem/empty".to_string(),
            name: "EmptyCodeSystem".to_string(),
            title: None,
            status: "active".to_string(),
            description: None,
            content: "not-present".to_string(),
            concept: None,
            version: None,
            publisher: None,
            contact: None,
        };

        let exportable = processor.process(&cs).unwrap();

        assert_eq!(exportable.name(), "EmptyCodeSystem");
        assert_eq!(exportable.rules.len(), 0); // No concepts
    }

    #[tokio::test]
    async fn test_process_code_system_without_definitions() {
        let lake = create_test_lake().await;
        let processor = CodeSystemProcessor::new(&lake);

        let cs = CodeSystem {
            resource_type: Some("CodeSystem".to_string()),
            id: Some("no-def-cs".to_string()),
            url: "http://example.org/CodeSystem/no-def".to_string(),
            name: "NoDefCodeSystem".to_string(),
            title: None,
            status: "active".to_string(),
            description: None,
            content: "complete".to_string(),
            concept: Some(vec![
                ConceptDefinition {
                    code: "code1".to_string(),
                    display: Some("Code 1".to_string()),
                    definition: None,
                    concept: None,
                },
                ConceptDefinition {
                    code: "code2".to_string(),
                    display: None,
                    definition: None,
                    concept: None,
                },
            ]),
            version: None,
            publisher: None,
            contact: None,
        };

        let exportable = processor.process(&cs).unwrap();

        assert_eq!(exportable.rules.len(), 2);
    }
}
