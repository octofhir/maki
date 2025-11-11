//! ValueSet processor
//!
//! Converts FHIR ValueSet resources into ExportableValueSet objects

use crate::{
    models::{ValueSet, ValueSetCompose, ValueSetInclude, ValueSetExpansion},
    exportable::{ExportableValueSet, IncludeRule, ExcludeRule, IncludeConcept, ValueSetFilter, Exportable},
    lake::ResourceLake,
    Result,
};
use log::{debug, warn};

/// ValueSet processor
pub struct ValueSetProcessor<'a> {
    lake: &'a ResourceLake,
}

impl<'a> ValueSetProcessor<'a> {
    /// Create a new ValueSet processor
    pub fn new(lake: &'a ResourceLake) -> Self {
        Self { lake }
    }

    /// Process a ValueSet into an ExportableValueSet
    pub fn process(&self, vs: &ValueSet) -> Result<ExportableValueSet> {
        debug!("Processing ValueSet '{}'", vs.name);

        let mut exportable = ExportableValueSet::new(vs.name.clone());

        // Set optional fields
        if let Some(id) = &vs.id {
            exportable.id = Some(id.clone());
        }

        if let Some(title) = &vs.title {
            exportable.title = Some(title.clone());
        }

        if let Some(desc) = &vs.description {
            exportable.description = Some(desc.clone());
        }

        // Process compose (preferred) or expansion (fallback)
        if let Some(compose) = &vs.compose {
            self.process_compose(&mut exportable, compose)?;
        } else if let Some(expansion) = &vs.expansion {
            self.process_expansion(&mut exportable, expansion)?;
        } else {
            debug!("ValueSet '{}' has no compose or expansion", vs.name);
        }

        debug!(
            "Created ExportableValueSet '{}' with {} rules",
            exportable.name(),
            exportable.rules.len()
        );

        Ok(exportable)
    }

    /// Process compose element
    fn process_compose(
        &self,
        exportable: &mut ExportableValueSet,
        compose: &ValueSetCompose,
    ) -> Result<()> {
        // Process includes
        for include in &compose.include {
            self.process_include(exportable, include)?;
        }

        // Process excludes
        if let Some(excludes) = &compose.exclude {
            for exclude in excludes {
                self.process_exclude(exportable, exclude)?;
            }
        }

        Ok(())
    }

    /// Process include element
    fn process_include(
        &self,
        exportable: &mut ExportableValueSet,
        include: &ValueSetInclude,
    ) -> Result<()> {
        if let Some(system) = &include.system {
            // Convert concepts
            let concepts = if let Some(fhir_concepts) = &include.concept {
                fhir_concepts
                    .iter()
                    .map(|c| IncludeConcept {
                        code: c.code.clone(),
                        display: c.display.clone(),
                    })
                    .collect()
            } else {
                vec![]
            };

            // Convert filters
            let filters = if let Some(fhir_filters) = &include.filter {
                fhir_filters
                    .iter()
                    .map(|f| ValueSetFilter {
                        property: f.property.clone(),
                        operator: f.op.clone(),
                        value: f.value.clone(),
                    })
                    .collect()
            } else {
                vec![]
            };

            let rule = IncludeRule {
                system: system.clone(),
                version: include.version.clone(),
                concepts,
                filters,
            };
            exportable.add_rule(Box::new(rule));
        } else if let Some(_value_sets) = &include.value_set {
            // Note: ValueSet references will be handled in optimization phase
            warn!("Include from other ValueSets not yet supported");
        }

        Ok(())
    }

    /// Process exclude element
    fn process_exclude(
        &self,
        exportable: &mut ExportableValueSet,
        exclude: &ValueSetInclude,
    ) -> Result<()> {
        if let Some(system) = &exclude.system {
            // Convert concepts
            let concepts = if let Some(fhir_concepts) = &exclude.concept {
                fhir_concepts
                    .iter()
                    .map(|c| IncludeConcept {
                        code: c.code.clone(),
                        display: c.display.clone(),
                    })
                    .collect()
            } else {
                vec![]
            };

            // Convert filters
            let filters = if let Some(fhir_filters) = &exclude.filter {
                fhir_filters
                    .iter()
                    .map(|f| ValueSetFilter {
                        property: f.property.clone(),
                        operator: f.op.clone(),
                        value: f.value.clone(),
                    })
                    .collect()
            } else {
                vec![]
            };

            let rule = ExcludeRule {
                system: system.clone(),
                version: exclude.version.clone(),
                concepts,
                filters,
            };
            exportable.add_rule(Box::new(rule));
        } else {
            warn!("Exclude has no system");
        }

        Ok(())
    }

    /// Process expansion element (fallback when compose is not available)
    fn process_expansion(
        &self,
        exportable: &mut ExportableValueSet,
        expansion: &ValueSetExpansion,
    ) -> Result<()> {
        debug!("Processing expansion for '{}'", exportable.name());

        if let Some(contains) = &expansion.contains {
            for item in contains {
                if let (Some(system), Some(code)) = (&item.system, &item.code) {
                    let rule = IncludeRule {
                        system: system.clone(),
                        version: None,
                        concepts: vec![IncludeConcept {
                            code: code.clone(),
                            display: item.display.clone(),
                        }],
                        filters: vec![],
                    };
                    exportable.add_rule(Box::new(rule));
                } else {
                    warn!("Expansion contains item missing system or code");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ValueSetConcept, ValueSetFilter};
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

    fn create_test_value_set_with_compose() -> ValueSet {
        ValueSet {
            resource_type: Some("ValueSet".to_string()),
            id: Some("example-vs".to_string()),
            url: "http://example.org/ValueSet/example".to_string(),
            name: "ExampleValueSet".to_string(),
            title: Some("Example Value Set".to_string()),
            status: "active".to_string(),
            description: Some("An example value set".to_string()),
            compose: Some(ValueSetCompose {
                include: vec![ValueSetInclude {
                    system: Some("http://example.org/CodeSystem/example".to_string()),
                    version: None,
                    concept: Some(vec![
                        ValueSetConcept {
                            code: "code1".to_string(),
                            display: Some("Code 1".to_string()),
                        },
                        ValueSetConcept {
                            code: "code2".to_string(),
                            display: Some("Code 2".to_string()),
                        },
                    ]),
                    filter: None,
                    value_set: None,
                }],
                exclude: None,
            }),
            expansion: None,
            version: None,
            publisher: None,
            contact: None,
        }
    }

    fn create_test_value_set_with_expansion() -> ValueSet {
        use crate::models::ValueSetExpansionContains;

        ValueSet {
            resource_type: Some("ValueSet".to_string()),
            id: Some("expanded-vs".to_string()),
            url: "http://example.org/ValueSet/expanded".to_string(),
            name: "ExpandedValueSet".to_string(),
            title: Some("Expanded Value Set".to_string()),
            status: "active".to_string(),
            description: Some("A pre-expanded value set".to_string()),
            compose: None,
            expansion: Some(ValueSetExpansion {
                contains: Some(vec![
                    ValueSetExpansionContains {
                        system: Some("http://example.org/CodeSystem/example".to_string()),
                        code: Some("code1".to_string()),
                        display: Some("Code 1".to_string()),
                    },
                    ValueSetExpansionContains {
                        system: Some("http://example.org/CodeSystem/example".to_string()),
                        code: Some("code2".to_string()),
                        display: Some("Code 2".to_string()),
                    },
                ]),
            }),
            version: None,
            publisher: None,
            contact: None,
        }
    }

    #[tokio::test]
    async fn test_process_value_set_with_compose() {
        let lake = create_test_lake().await;
        let processor = ValueSetProcessor::new(&lake);

        let vs = create_test_value_set_with_compose();
        let exportable = processor.process(&vs).unwrap();

        assert_eq!(exportable.name(), "ExampleValueSet");
        assert_eq!(exportable.id, Some("example-vs".to_string()));
        assert_eq!(exportable.title, Some("Example Value Set".to_string()));
        assert_eq!(exportable.rules.len(), 1); // 1 include rule with 2 concepts
    }

    #[tokio::test]
    async fn test_process_value_set_with_expansion() {
        let lake = create_test_lake().await;
        let processor = ValueSetProcessor::new(&lake);

        let vs = create_test_value_set_with_expansion();
        let exportable = processor.process(&vs).unwrap();

        assert_eq!(exportable.name(), "ExpandedValueSet");
        assert_eq!(exportable.id, Some("expanded-vs".to_string()));
        assert_eq!(exportable.rules.len(), 2); // 2 concepts from expansion
    }

    #[tokio::test]
    async fn test_process_include_entire_system() {
        let lake = create_test_lake().await;
        let processor = ValueSetProcessor::new(&lake);

        let vs = ValueSet {
            resource_type: Some("ValueSet".to_string()),
            id: Some("system-vs".to_string()),
            url: "http://example.org/ValueSet/system".to_string(),
            name: "SystemValueSet".to_string(),
            title: None,
            status: "active".to_string(),
            description: None,
            compose: Some(ValueSetCompose {
                include: vec![ValueSetInclude {
                    system: Some("http://example.org/CodeSystem/example".to_string()),
                    version: None,
                    concept: None,
                    filter: None,
                    value_set: None,
                }],
                exclude: None,
            }),
            expansion: None,
            version: None,
            publisher: None,
            contact: None,
        };

        let exportable = processor.process(&vs).unwrap();

        assert_eq!(exportable.rules.len(), 1); // Entire system included
    }

    #[tokio::test]
    async fn test_process_exclude() {
        let lake = create_test_lake().await;
        let processor = ValueSetProcessor::new(&lake);

        let vs = ValueSet {
            resource_type: Some("ValueSet".to_string()),
            id: Some("exclude-vs".to_string()),
            url: "http://example.org/ValueSet/exclude".to_string(),
            name: "ExcludeValueSet".to_string(),
            title: None,
            status: "active".to_string(),
            description: None,
            compose: Some(ValueSetCompose {
                include: vec![ValueSetInclude {
                    system: Some("http://example.org/CodeSystem/example".to_string()),
                    version: None,
                    concept: None,
                    filter: None,
                    value_set: None,
                }],
                exclude: Some(vec![ValueSetInclude {
                    system: Some("http://example.org/CodeSystem/example".to_string()),
                    version: None,
                    concept: Some(vec![ValueSetConcept {
                        code: "excluded-code".to_string(),
                        display: Some("Excluded Code".to_string()),
                    }]),
                    filter: None,
                    value_set: None,
                }]),
            }),
            expansion: None,
            version: None,
            publisher: None,
            contact: None,
        };

        let exportable = processor.process(&vs).unwrap();

        assert_eq!(exportable.rules.len(), 2); // 1 include + 1 exclude
    }

    #[tokio::test]
    async fn test_process_with_filters() {
        let lake = create_test_lake().await;
        let processor = ValueSetProcessor::new(&lake);

        let vs = ValueSet {
            resource_type: Some("ValueSet".to_string()),
            id: Some("filter-vs".to_string()),
            url: "http://example.org/ValueSet/filter".to_string(),
            name: "FilterValueSet".to_string(),
            title: None,
            status: "active".to_string(),
            description: None,
            compose: Some(ValueSetCompose {
                include: vec![ValueSetInclude {
                    system: Some("http://example.org/CodeSystem/example".to_string()),
                    version: None,
                    concept: None,
                    filter: Some(vec![ValueSetFilter {
                        property: "status".to_string(),
                        op: "=".to_string(),
                        value: "active".to_string(),
                    }]),
                    value_set: None,
                }],
                exclude: None,
            }),
            expansion: None,
            version: None,
            publisher: None,
            contact: None,
        };

        let exportable = processor.process(&vs).unwrap();

        assert_eq!(exportable.rules.len(), 1); // 1 include with filter
    }
}
