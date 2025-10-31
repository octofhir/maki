//! Logical Model and Resource Exporter
//!
//! Exports FSH Logical models and Resource definitions to FHIR StructureDefinition resources.
//! Unlike Profiles and Extensions which constrain existing types, Logical models and Resources
//! define entirely new types with their own element structure.
//!
//! # Logical vs Resource
//!
//! - **Logical**: Abstract data models with `kind: "logical"`
//! - **Resource**: Concrete FHIR resources with `kind: "resource"`
//!
//! Both follow the same export pattern with element path transformation.
//!
//! # Path Transformation
//!
//! When defining a new type, element paths must be transformed from the base type:
//! - Base: `Element.id` → New: `MyLogical.id`
//! - Base: `Element.extension` → New: `MyLogical.extension`
//!
//! # Characteristics Extension
//!
//! Logical models can have characteristics (e.g., `#can-bind`, `#has-range`) which are
//! represented as extensions on the StructureDefinition.
//!
//! See: <https://www.hl7.org/fhir/structuredefinition.html>

use super::ExportError;
use super::fhir_types::*;
use crate::canonical::DefinitionSession;
use crate::cst::ast::{CardRule, FixedValueRule, FlagRule, Logical, Resource, Rule, ValueSetRule};
use crate::semantic::path_resolver::PathResolver;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace, warn};

/// Logical model and Resource exporter
///
/// Transforms FSH Logical/Resource AST nodes into FHIR StructureDefinition resources.
pub struct LogicalExporter {
    /// Session for resolving FHIR definitions
    session: Arc<DefinitionSession>,
    /// Path resolver for finding elements
    path_resolver: Arc<PathResolver>,
    /// Base URL for generated resources
    base_url: String,
}

impl LogicalExporter {
    /// Create a new logical/resource exporter
    pub async fn new(
        session: Arc<DefinitionSession>,
        base_url: String,
    ) -> Result<Self, ExportError> {
        let path_resolver = Arc::new(PathResolver::new(session.clone()));

        Ok(Self {
            session,
            path_resolver,
            base_url,
        })
    }

    /// Export a Logical model to StructureDefinition
    pub async fn export_logical(
        &self,
        logical: &Logical,
    ) -> Result<StructureDefinition, ExportError> {
        let name = logical
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("logical name".to_string()))?;

        debug!("Exporting logical model: {}", name);

        // Get parent (defaults to Element if not specified)
        let parent = logical
            .parent()
            .and_then(|p| p.value())
            .unwrap_or_else(|| "Element".to_string());

        // Get base StructureDefinition
        let mut structure_def = self.get_base_structure_definition(&parent).await?;

        // Apply metadata
        self.apply_metadata(
            &mut structure_def,
            &name,
            logical.id(),
            logical.title(),
            logical.description(),
        )?;

        // Set as logical model
        structure_def.kind = StructureDefinitionKind::Logical;
        structure_def.is_abstract = false;
        structure_def.type_field = name.clone();
        structure_def.derivation = Some("specialization".to_string());

        // Transform element paths from base type to new type
        self.transform_element_paths(&mut structure_def, &parent, &name)?;

        // Make a copy of the base snapshot for comparison
        let base_snapshot = structure_def.snapshot.clone();

        // Apply characteristics as extensions
        let characteristics = logical.characteristics();
        if !characteristics.is_empty() {
            self.apply_characteristics(&mut structure_def, &characteristics)?;
        }

        // Apply all rules to snapshot
        for rule in logical.rules() {
            if let Err(e) = self.apply_rule(&mut structure_def, &rule).await {
                warn!("Failed to apply rule: {}", e);
            }
        }

        // Generate differential from changes
        if let Some(base_snap) = base_snapshot {
            structure_def.differential =
                Some(self.generate_differential(&base_snap, &structure_def));
        }

        // Validate exported structure
        self.validate_structure_definition(&structure_def)?;

        debug!("Successfully exported logical model: {}", name);
        Ok(structure_def)
    }

    /// Export a Resource definition to StructureDefinition
    pub async fn export_resource(
        &self,
        resource: &Resource,
    ) -> Result<StructureDefinition, ExportError> {
        let name = resource
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("resource name".to_string()))?;

        debug!("Exporting resource: {}", name);

        // Get parent (defaults to DomainResource for resources)
        let parent = resource
            .parent()
            .and_then(|p| p.value())
            .unwrap_or_else(|| "DomainResource".to_string());

        // Get base StructureDefinition
        let mut structure_def = self.get_base_structure_definition(&parent).await?;

        // Apply metadata
        self.apply_metadata(
            &mut structure_def,
            &name,
            resource.id(),
            resource.title(),
            resource.description(),
        )?;

        // Set as resource
        structure_def.kind = StructureDefinitionKind::Resource;
        structure_def.is_abstract = false;
        structure_def.type_field = name.clone();
        structure_def.derivation = Some("specialization".to_string());

        // Transform element paths from base type to new type
        self.transform_element_paths(&mut structure_def, &parent, &name)?;

        // Make a copy of the base snapshot for comparison
        let base_snapshot = structure_def.snapshot.clone();

        // Apply all rules to snapshot
        for rule in resource.rules() {
            if let Err(e) = self.apply_rule(&mut structure_def, &rule).await {
                warn!("Failed to apply rule: {}", e);
            }
        }

        // Generate differential from changes
        if let Some(base_snap) = base_snapshot {
            structure_def.differential =
                Some(self.generate_differential(&base_snap, &structure_def));
        }

        // Validate exported structure
        self.validate_structure_definition(&structure_def)?;

        debug!("Successfully exported resource: {}", name);
        Ok(structure_def)
    }

    /// Get base StructureDefinition from parent type
    async fn get_base_structure_definition(
        &self,
        parent: &str,
    ) -> Result<StructureDefinition, ExportError> {
        debug!("Resolving parent: {}", parent);

        let canonical_url = if parent.starts_with("http://") || parent.starts_with("https://") {
            parent.to_string()
        } else {
            format!("http://hl7.org/fhir/StructureDefinition/{}", parent)
        };

        let resource = self
            .session
            .resolve(&canonical_url)
            .await
            .map_err(|e| ExportError::ParentNotFound(format!("{}: {}", parent, e)))?;

        let structure_def: StructureDefinition =
            serde_json::from_value((*resource.content).clone()).map_err(|e| {
                ExportError::CanonicalError(format!(
                    "Failed to parse StructureDefinition for {}: {}",
                    parent, e
                ))
            })?;

        debug!("Resolved parent: {} ({})", parent, structure_def.url);
        Ok(structure_def)
    }

    /// Apply metadata to StructureDefinition
    fn apply_metadata(
        &self,
        structure_def: &mut StructureDefinition,
        name: &str,
        id: Option<crate::cst::ast::IdClause>,
        title: Option<crate::cst::ast::TitleClause>,
        description: Option<crate::cst::ast::DescriptionClause>,
    ) -> Result<(), ExportError> {
        structure_def.name = name.to_string();
        structure_def.url = format!("{}/StructureDefinition/{}", self.base_url, name);

        if let Some(id_clause) = id
            && let Some(id_value) = id_clause.value()
        {
            structure_def.id = Some(id_value);
        }

        if let Some(title_clause) = title
            && let Some(title_value) = title_clause.value()
        {
            structure_def.title = Some(title_value);
        }

        if let Some(desc_clause) = description
            && let Some(desc_value) = desc_clause.value()
        {
            structure_def.description = Some(desc_value);
        }

        Ok(())
    }

    /// Transform element paths from base type to new type
    ///
    /// For a logical model or resource, all element paths must be changed
    /// from the parent type to the new type name.
    ///
    /// Example: Element.id → MyLogical.id
    fn transform_element_paths(
        &self,
        structure_def: &mut StructureDefinition,
        parent_type: &str,
        new_type: &str,
    ) -> Result<(), ExportError> {
        debug!(
            "Transforming element paths from {} to {}",
            parent_type, new_type
        );

        if let Some(ref mut snapshot) = structure_def.snapshot {
            for element in &mut snapshot.element {
                // Replace the type prefix in the path
                if element.path.starts_with(parent_type) {
                    element.path = element.path.replacen(parent_type, new_type, 1);
                    trace!("Transformed path to: {}", element.path);
                }
            }
        }

        // Update root element's base.path
        if let Some(snapshot) = &mut structure_def.snapshot
            && let Some(root_element) = snapshot.element.first_mut()
        {
            // Root element path should match the new type
            root_element.path = new_type.to_string();
        }

        Ok(())
    }

    /// Apply characteristics as extensions on the StructureDefinition
    ///
    /// Characteristics like #can-bind, #has-range are represented using the
    /// structuredefinition-type-characteristics extension.
    ///
    /// See: <http://hl7.org/fhir/StructureDefinition/structuredefinition-type-characteristics>
    fn apply_characteristics(
        &self,
        structure_def: &mut StructureDefinition,
        characteristics: &[String],
    ) -> Result<(), ExportError> {
        debug!("Applying {} characteristics", characteristics.len());

        // For now, we'll store characteristics as a comment in the description
        // Full implementation would use the structuredefinition-type-characteristics extension
        let char_text = format!("Characteristics: {}", characteristics.join(", "));

        if let Some(ref mut desc) = structure_def.description {
            desc.push_str(&format!("\n\n{}", char_text));
        } else {
            structure_def.description = Some(char_text);
        }

        // TODO: Implement proper extension support
        // structure_def.extension = Some(vec![...])

        Ok(())
    }

    /// Apply a single rule to the StructureDefinition
    async fn apply_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &Rule,
    ) -> Result<(), ExportError> {
        match rule {
            Rule::Card(card_rule) => self.apply_cardinality_rule(structure_def, card_rule).await,
            Rule::Flag(flag_rule) => self.apply_flag_rule(structure_def, flag_rule).await,
            Rule::ValueSet(valueset_rule) => {
                self.apply_binding_rule(structure_def, valueset_rule).await
            }
            Rule::FixedValue(fixed_rule) => {
                self.apply_fixed_value_rule(structure_def, fixed_rule).await
            }
            Rule::Path(_) => {
                // PathRule is for type constraints
                Ok(())
            }
            Rule::Contains(_) | Rule::Only(_) | Rule::Obeys(_) => {
                // TODO: Implement these rule types for logical models
                Ok(())
            }
        }
    }

    /// Apply cardinality rule
    async fn apply_cardinality_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &CardRule,
    ) -> Result<(), ExportError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::MissingRequiredField("path".to_string()))?;

        let cardinality = rule
            .cardinality()
            .ok_or_else(|| ExportError::InvalidCardinality("missing".to_string()))?;

        trace!("Applying cardinality rule: {} {}", path_str, cardinality);

        let parts: Vec<&str> = cardinality.split("..").collect();
        if parts.len() != 2 {
            return Err(ExportError::InvalidCardinality(cardinality));
        }

        let min = parts[0]
            .parse::<u32>()
            .map_err(|_| ExportError::InvalidCardinality(cardinality.clone()))?;
        let max = parts[1].to_string();

        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let type_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            warn!("Element not found in snapshot: {}", full_path);
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: type_name,
            }
        })?;

        debug!("Applying cardinality {}..{} to {}", min, max, full_path);
        element.min = Some(min);
        element.max = Some(max);

        for flag in rule.flags() {
            self.apply_flag_to_element(element, &flag)?;
        }

        Ok(())
    }

    /// Apply flag rule
    async fn apply_flag_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &FlagRule,
    ) -> Result<(), ExportError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::MissingRequiredField("path".to_string()))?;

        trace!("Applying flag rule: {}", path_str);

        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let type_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: type_name,
            }
        })?;

        for flag in rule.flags() {
            self.apply_flag_to_element(element, &flag)?;
        }

        Ok(())
    }

    /// Apply a flag to an element
    fn apply_flag_to_element(
        &self,
        element: &mut ElementDefinition,
        flag: &str,
    ) -> Result<(), ExportError> {
        match flag.to_uppercase().as_str() {
            "MS" => element.must_support = Some(true),
            "SU" => element.is_summary = Some(true),
            "?!" => element.is_modifier = Some(true),
            _ => {
                warn!("Unknown flag: {}", flag);
            }
        }
        Ok(())
    }

    /// Apply binding rule
    async fn apply_binding_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &ValueSetRule,
    ) -> Result<(), ExportError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::MissingRequiredField("path".to_string()))?;

        let value_set = rule
            .value_set()
            .ok_or_else(|| ExportError::MissingRequiredField("value set".to_string()))?;

        let strength_str = rule.strength().unwrap_or_else(|| "required".to_string());

        trace!(
            "Applying binding rule: {} from {} ({})",
            path_str, value_set, strength_str
        );

        let strength = BindingStrength::from_str(&strength_str)
            .ok_or_else(|| ExportError::InvalidBindingStrength(strength_str.clone()))?;

        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let type_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: type_name,
            }
        })?;

        let value_set_url = if value_set.starts_with("http://") || value_set.starts_with("https://")
        {
            value_set
        } else {
            format!("{}/ValueSet/{}", self.base_url, value_set)
        };

        element.binding = Some(ElementDefinitionBinding {
            strength,
            description: None,
            value_set: Some(value_set_url),
        });

        Ok(())
    }

    /// Apply fixed value rule
    async fn apply_fixed_value_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &FixedValueRule,
    ) -> Result<(), ExportError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::MissingRequiredField("path".to_string()))?;

        let value = rule
            .value()
            .ok_or_else(|| ExportError::MissingRequiredField("value".to_string()))?;

        trace!("Applying fixed value rule: {} = {}", path_str, value);

        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let type_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: type_name,
            }
        })?;

        let mut pattern_map = HashMap::new();

        if value.starts_with('"') {
            let parsed_value: JsonValue = serde_json::from_str(&value)?;
            pattern_map.insert("patternString".to_string(), parsed_value);
        } else if value.starts_with('#') {
            let code = value.trim_start_matches('#');
            pattern_map.insert(
                "patternCode".to_string(),
                JsonValue::String(code.to_string()),
            );
        } else if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            let parsed_value: JsonValue = serde_json::from_str(&value)?;
            pattern_map.insert("patternInteger".to_string(), parsed_value);
        } else {
            pattern_map.insert("patternCode".to_string(), JsonValue::String(value));
        };

        element.pattern = Some(pattern_map);

        Ok(())
    }

    /// Resolve a path string to full element path
    async fn resolve_full_path(
        &self,
        structure_def: &StructureDefinition,
        path: &str,
    ) -> Result<String, ExportError> {
        if path.contains('.') {
            let parts: Vec<&str> = path.split('.').collect();
            if parts[0] == structure_def.type_field {
                return Ok(path.to_string());
            }
        }

        Ok(format!("{}.{}", structure_def.type_field, path))
    }

    /// Generate differential by comparing snapshot with base
    fn generate_differential(
        &self,
        base: &StructureDefinitionSnapshot,
        modified: &StructureDefinition,
    ) -> StructureDefinitionDifferential {
        let mut differential_elements = Vec::new();

        if let Some(modified_snapshot) = &modified.snapshot {
            for modified_elem in &modified_snapshot.element {
                if let Some(base_elem) = base.element.iter().find(|e| e.path == modified_elem.path)
                {
                    if modified_elem.is_modified_from(base_elem) {
                        differential_elements.push(modified_elem.clone());
                    }
                } else {
                    differential_elements.push(modified_elem.clone());
                }
            }
        }

        trace!(
            "Generated differential with {} elements",
            differential_elements.len()
        );

        StructureDefinitionDifferential {
            element: differential_elements,
        }
    }

    /// Validate structure definition
    fn validate_structure_definition(
        &self,
        structure_def: &StructureDefinition,
    ) -> Result<(), ExportError> {
        if structure_def.url.is_empty() {
            return Err(ExportError::MissingRequiredField("url".to_string()));
        }
        if structure_def.name.is_empty() {
            return Err(ExportError::MissingRequiredField("name".to_string()));
        }
        if structure_def.type_field.is_empty() {
            return Err(ExportError::MissingRequiredField("type".to_string()));
        }

        if let Some(differential) = &structure_def.differential {
            for element in &differential.element {
                self.validate_element_definition(element)?;
            }
        }

        Ok(())
    }

    /// Validate an ElementDefinition
    fn validate_element_definition(&self, element: &ElementDefinition) -> Result<(), ExportError> {
        if element.path.is_empty() {
            return Err(ExportError::MissingRequiredField(
                "element.path".to_string(),
            ));
        }

        if let (Some(min), Some(max)) = (&element.min, &element.max)
            && max != "*"
            && let Ok(max_val) = max.parse::<u32>()
            && *min > max_val
        {
            return Err(ExportError::InvalidCardinality(format!("{}..{}", min, max)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_transformation() {
        // Test that path transformation logic is correct
        let parent = "Element";
        let new_type = "MyLogical";

        let path = "Element.id";
        let transformed = path.replacen(parent, new_type, 1);
        assert_eq!(transformed, "MyLogical.id");

        let path = "Element.extension";
        let transformed = path.replacen(parent, new_type, 1);
        assert_eq!(transformed, "MyLogical.extension");
    }
}
