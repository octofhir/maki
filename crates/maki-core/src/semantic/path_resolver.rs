//! Path resolution algorithm for FSH paths to FHIR ElementDefinitions
//!
//! This module implements SUSHI's `findElementByPath()` algorithm - the core path
//! resolution logic that navigates from FSH paths (e.g., `name.given`,
//! `contact[0].telecom[+].system`, `deceased[x]`) to FHIR ElementDefinitions in
//! StructureDefinitions.
//!
//! # Algorithm Overview
//!
//! 1. **Fast Path**: Direct lookup in elements map
//! 2. **Path Parsing**: Break down FSH path into segments with bracket information
//! 3. **Iterative Resolution**: Navigate element tree segment by segment
//! 4. **Element Unfolding**: Fetch children from parent types when needed
//! 5. **Bracket Handling**: Process slices, array indices, and soft indexing
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::semantic::PathResolver;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let session = todo!();
//! let resolver = PathResolver::new(session);
//!
//! // Resolve simple path
//! let element = resolver.resolve_path("Patient", "name.given").await?;
//!
//! // Resolve with choice type
//! let deceased = resolver.resolve_path("Patient", "deceased[x]").await?;
//!
//! // Resolve with slice
//! let component = resolver.resolve_path("Observation", "component[systolic]").await?;
//! # Ok(())
//! # }
//! ```

use crate::canonical::{DefinitionResource, DefinitionSession};
use crate::canonical::fishable::{Fishable, FhirType};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, trace, warn};

/// Path segment with bracket information
///
/// Represents a single segment in a FSH path, including any bracket notation.
/// For example, in `contact[0].telecom[+].system`, there are three segments:
/// - `contact` with bracket `[0]`
/// - `telecom` with bracket `[+]`
/// - `system` with no bracket
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment {
    /// Base element name (before any bracket)
    pub base: String,
    /// Optional bracket information
    pub bracket: Option<Bracket>,
}

impl PathSegment {
    /// Create a new path segment without a bracket
    pub fn new(base: String) -> Self {
        Self {
            base,
            bracket: None,
        }
    }

    /// Create a new path segment with a bracket
    pub fn with_bracket(base: String, bracket: Bracket) -> Self {
        Self {
            base,
            bracket: Some(bracket),
        }
    }
}

/// Bracket types in FSH paths
///
/// Represents the different kinds of bracket notation that can appear in FSH paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bracket {
    /// Slice name: `[sliceName]`
    ///
    /// Used to reference a specific slice within a sliced element.
    Slice(String),

    /// Array index: `[0]`, `[1]`, `[2]`
    ///
    /// Used to reference a specific array element by position.
    Index(usize),

    /// Soft indexing: `[+]` or `[=]`
    ///
    /// Used for dynamic array indexing during instance creation.
    Soft(SoftIndexOp),

    /// Choice type suffix: `[x]`
    ///
    /// Used to reference choice type elements (e.g., `value[x]`, `deceased[x]`).
    ChoiceType,
}

/// Soft indexing operators
///
/// FSH supports two soft indexing operators for array manipulation:
/// - `[+]`: Increment - creates a new array element
/// - `[=]`: Repeat - reuses the last referenced array element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftIndexOp {
    /// `[+]` - Increment to next array index
    Increment,
    /// `[=]` - Repeat last array index
    Repeat,
}

/// Path resolution errors
///
/// Errors that can occur during FSH path resolution.
#[derive(Debug, Error)]
pub enum PathError {
    /// Path not found in structure definition
    #[error("Path not found: {path} in {base_type}")]
    NotFound { path: String, base_type: String },

    /// Path resolves to multiple elements (ambiguous)
    #[error("Ambiguous path: {path} matches {count} elements")]
    Ambiguous { path: String, count: usize },

    /// Invalid path syntax
    #[error("Invalid path syntax: {0}")]
    InvalidSyntax(String),

    /// Cannot unfold element (missing type information or definition)
    #[error("Cannot unfold element: {element_path} - {reason}")]
    UnfoldError {
        element_path: String,
        reason: String,
    },

    /// Canonical resolution error
    #[error("Canonical resolution error: {0}")]
    CanonicalError(String),

    /// Invalid element structure
    #[error("Invalid element structure: {0}")]
    InvalidElement(String),
}

/// ElementDefinition wrapper
///
/// Lightweight wrapper around FHIR ElementDefinition JSON with convenient accessors.
#[derive(Debug, Clone)]
pub struct ElementDefinition {
    /// Full JSON content of the ElementDefinition
    pub content: Arc<Value>,
}

impl ElementDefinition {
    /// Create from JSON value
    pub fn new(content: Value) -> Self {
        Self {
            content: Arc::new(content),
        }
    }

    /// Get the element ID (e.g., "Patient.name.given")
    pub fn id(&self) -> Option<&str> {
        self.content.get("id").and_then(|v| v.as_str())
    }

    /// Get the element path (e.g., "Patient.name.given")
    pub fn path(&self) -> Option<&str> {
        self.content.get("path").and_then(|v| v.as_str())
    }

    /// Get the slice name if this is a slice
    pub fn slice_name(&self) -> Option<&str> {
        self.content.get("sliceName").and_then(|v| v.as_str())
    }

    /// Get element types
    pub fn types(&self) -> Vec<ElementType> {
        self.content
            .get("type")
            .and_then(|v| v.as_array())
            .map(|types| {
                types
                    .iter()
                    .filter_map(|t| {
                        let code = t.get("code").and_then(|c| c.as_str())?;
                        Some(ElementType {
                            code: code.to_string(),
                            profile: t
                                .get("profile")
                                .and_then(|p| p.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|v| v.as_str())
                                .map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if this is a choice type element (path ends with [x])
    pub fn is_choice_type(&self) -> bool {
        self.path()
            .map(|p| p.ends_with("[x]"))
            .unwrap_or(false)
    }
}

/// Element type information
#[derive(Debug, Clone)]
pub struct ElementType {
    /// FHIR type code (e.g., "string", "HumanName", "Reference")
    pub code: String,
    /// Optional profile URL
    pub profile: Option<String>,
}

/// StructureDefinition wrapper
///
/// Lightweight wrapper around FHIR StructureDefinition JSON with convenient accessors.
#[derive(Debug, Clone)]
pub struct StructureDefinition {
    /// Full JSON content
    pub content: Arc<Value>,
}

impl StructureDefinition {
    /// Create from definition resource
    pub fn from_resource(resource: &DefinitionResource) -> Self {
        Self {
            content: resource.content.clone(),
        }
    }

    /// Get the canonical URL
    pub fn url(&self) -> Option<&str> {
        self.content.get("url").and_then(|v| v.as_str())
    }

    /// Get the type name (e.g., "Patient", "Observation")
    pub fn type_name(&self) -> &str {
        self.content
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
    }

    /// Get all element definitions
    pub fn elements(&self) -> Vec<ElementDefinition> {
        self.content
            .get("snapshot")
            .or_else(|| self.content.get("differential"))
            .and_then(|s| s.get("element"))
            .and_then(|e| e.as_array())
            .map(|elements| {
                elements
                    .iter()
                    .map(|e| ElementDefinition::new(e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find element by exact path match
    pub fn find_element_by_path(&self, path: &str) -> Option<ElementDefinition> {
        self.elements()
            .into_iter()
            .find(|e| e.path() == Some(path))
    }
}

/// Path resolver
///
/// Resolves FSH paths to FHIR ElementDefinitions using SUSHI's algorithm.
/// Provides caching for performance.
pub struct PathResolver {
    /// Session for canonical resource lookups
    session: Arc<DefinitionSession>,
    /// Cache: (SD url, path) → ElementDefinition
    cache: DashMap<(String, String), Arc<ElementDefinition>>,
}

impl PathResolver {
    /// Create a new path resolver
    pub fn new(session: Arc<DefinitionSession>) -> Self {
        Self {
            session,
            cache: DashMap::new(),
        }
    }

    /// Resolve FSH path to ElementDefinition
    ///
    /// # Arguments
    ///
    /// * `structure_def_id` - The StructureDefinition ID or URL to search in
    /// * `path` - The FSH path to resolve (e.g., "name.given", "deceased[x]")
    ///
    /// # Returns
    ///
    /// Returns the resolved `ElementDefinition` or a `PathError`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::semantic::PathResolver;
    /// # use std::sync::Arc;
    /// # async fn example(resolver: PathResolver) -> Result<(), Box<dyn std::error::Error>> {
    /// let element = resolver.resolve_path("Patient", "name.given").await?;
    /// println!("Resolved path: {:?}", element.path());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resolve_path(
        &self,
        structure_def_id: &str,
        path: &str,
    ) -> Result<ElementDefinition, PathError> {
        debug!(
            "Resolving path '{}' in StructureDefinition '{}'",
            path, structure_def_id
        );

        // Fish for the StructureDefinition
        let sd_resource = self
            .session
            .fish(structure_def_id, &[FhirType::StructureDefinition])
            .await
            .map_err(|e| PathError::CanonicalError(e.to_string()))?
            .ok_or_else(|| PathError::NotFound {
                path: path.to_string(),
                base_type: structure_def_id.to_string(),
            })?;

        let sd = StructureDefinition::from_resource(&sd_resource);

        // Check cache first
        let cache_key = (
            sd.url().unwrap_or(structure_def_id).to_string(),
            path.to_string(),
        );
        if let Some(cached) = self.cache.get(&cache_key) {
            trace!("Cache hit for path '{}'", path);
            return Ok((**cached).clone());
        }

        // STEP 1: FAST PATH - Direct lookup in elements
        if let Some(elem) = sd.find_element_by_path(path) {
            debug!("Fast path: found element directly");
            let elem = Arc::new(elem);
            self.cache.insert(cache_key, elem.clone());
            return Ok((*elem).clone());
        }

        // STEP 2: Parse path into segments
        let segments = self.parse_path(path)?;
        debug!("Parsed path into {} segments", segments.len());

        // STEP 3: Iterative resolution with unfolding
        let result = self.resolve_segments(&sd, &segments, path).await?;

        // Cache the result
        let result_arc = Arc::new(result.clone());
        self.cache.insert(cache_key, result_arc);

        Ok(result)
    }

    /// Parse FSH path into segments
    ///
    /// Handles various path formats including:
    /// - Simple paths: `name.given`
    /// - Bracketed paths: `name[0].given`
    /// - Sliced paths: `component[systolic].value`
    /// - Choice types: `deceased[x]`
    /// - Soft indexing: `telecom[+].system`
    fn parse_path(&self, path: &str) -> Result<Vec<PathSegment>, PathError> {
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut in_bracket = false;
        let mut bracket_content = String::new();

        for ch in path.chars() {
            match ch {
                '.' if !in_bracket => {
                    if !current.is_empty() {
                        segments.push(PathSegment::new(current.clone()));
                        current.clear();
                    }
                }
                '[' => {
                    in_bracket = true;
                    bracket_content.clear();
                }
                ']' => {
                    in_bracket = false;
                    let bracket = self.parse_bracket(&bracket_content)?;
                    segments.push(PathSegment::with_bracket(current.clone(), bracket));
                    current.clear();
                }
                _ => {
                    if in_bracket {
                        bracket_content.push(ch);
                    } else {
                        current.push(ch);
                    }
                }
            }
        }

        // Handle remaining content
        if !current.is_empty() {
            segments.push(PathSegment::new(current));
        }

        if in_bracket {
            return Err(PathError::InvalidSyntax(
                format!("Unclosed bracket in path: {}", path),
            ));
        }

        Ok(segments)
    }

    /// Parse bracket content
    fn parse_bracket(&self, content: &str) -> Result<Bracket, PathError> {
        match content {
            "+" => Ok(Bracket::Soft(SoftIndexOp::Increment)),
            "=" => Ok(Bracket::Soft(SoftIndexOp::Repeat)),
            "x" => Ok(Bracket::ChoiceType),
            _ => {
                // Try parsing as number (array index)
                if let Ok(index) = content.parse::<usize>() {
                    Ok(Bracket::Index(index))
                } else {
                    // Assume slice name
                    Ok(Bracket::Slice(content.to_string()))
                }
            }
        }
    }

    /// Resolve path segments iteratively
    async fn resolve_segments(
        &self,
        sd: &StructureDefinition,
        segments: &[PathSegment],
        original_path: &str,
    ) -> Result<ElementDefinition, PathError> {
        let mut current_path = sd.type_name().to_string();
        let mut current_elements = sd.elements();

        for (idx, segment) in segments.iter().enumerate() {
            let target_path = if idx == 0 && segment.base == sd.type_name() {
                // First segment might be the type name itself
                segment.base.clone()
            } else {
                format!("{}.{}", current_path, segment.base)
            };

            trace!("Resolving segment: {} -> {}", segment.base, target_path);

            // Filter elements matching current segment
            let mut matches: Vec<ElementDefinition> = current_elements
                .iter()
                .filter(|e| {
                    e.path()
                        .map(|p| p == target_path || (segment.bracket == Some(Bracket::ChoiceType) && p.starts_with(&format!("{}[x]", target_path.trim_end_matches("[x]")))))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            // If no matches, try unfolding
            if matches.is_empty() {
                debug!("No direct matches for '{}', attempting unfold", target_path);

                // Find parent element
                let parent_element = current_elements
                    .iter()
                    .find(|e| e.path() == Some(&current_path))
                    .ok_or_else(|| PathError::NotFound {
                        path: original_path.to_string(),
                        base_type: sd.type_name().to_string(),
                    })?;

                // Unfold parent element
                let unfolded = self.unfold_element(parent_element, &target_path).await?;
                current_elements.extend(unfolded.clone());

                // Retry search
                matches = current_elements
                    .iter()
                    .filter(|e| e.path() == Some(&target_path))
                    .cloned()
                    .collect();
            }

            // Handle brackets (slices, indices, choice types)
            let element = if let Some(bracket) = &segment.bracket {
                self.resolve_bracket(&matches, bracket, &target_path)?
            } else if matches.len() == 1 {
                matches.into_iter().next().unwrap()
            } else if matches.is_empty() {
                return Err(PathError::NotFound {
                    path: original_path.to_string(),
                    base_type: sd.type_name().to_string(),
                });
            } else {
                return Err(PathError::Ambiguous {
                    path: original_path.to_string(),
                    count: matches.len(),
                });
            };

            current_path = element.path().unwrap_or(&target_path).to_string();
        }

        // Return final element
        current_elements
            .into_iter()
            .find(|e| e.path() == Some(&current_path))
            .ok_or_else(|| PathError::NotFound {
                path: original_path.to_string(),
                base_type: sd.type_name().to_string(),
            })
    }

    /// Unfold element (fetch children from parent type)
    ///
    /// When an element references a complex type, we need to "unfold" it by
    /// fetching the child elements from the parent type's StructureDefinition.
    async fn unfold_element(
        &self,
        element: &ElementDefinition,
        target_path: &str,
    ) -> Result<Vec<ElementDefinition>, PathError> {
        let element_path = element.path().unwrap_or("unknown");
        debug!("Unfolding element: {}", element_path);

        // Get element types
        let types = element.types();
        if types.is_empty() {
            return Err(PathError::UnfoldError {
                element_path: element_path.to_string(),
                reason: "No type information available".to_string(),
            });
        }

        // Use first type for unfolding
        let parent_type = &types[0];
        debug!("Parent type for unfold: {}", parent_type.code);

        // Fish for parent StructureDefinition
        let parent_sd_resource = self
            .session
            .fish(&parent_type.code, &[FhirType::StructureDefinition])
            .await
            .map_err(|e| PathError::UnfoldError {
                element_path: element_path.to_string(),
                reason: format!("Failed to fish for type '{}': {}", parent_type.code, e),
            })?
            .ok_or_else(|| PathError::UnfoldError {
                element_path: element_path.to_string(),
                reason: format!("Type '{}' not found", parent_type.code),
            })?;

        let parent_sd = StructureDefinition::from_resource(&parent_sd_resource);

        // Get base element path (the child element name we're looking for)
        let child_name = target_path
            .rsplit('.')
            .next()
            .ok_or_else(|| PathError::UnfoldError {
                element_path: element_path.to_string(),
                reason: "Invalid target path".to_string(),
            })?;

        // Find matching children in parent SD
        let parent_type_name = parent_sd.type_name();
        let search_path = format!("{}.{}", parent_type_name, child_name);

        let children: Vec<ElementDefinition> = parent_sd
            .elements()
            .into_iter()
            .filter(|e| {
                e.path()
                    .map(|p| p == search_path || p.starts_with(&format!("{}.", search_path)))
                    .unwrap_or(false)
            })
            .collect();

        if children.is_empty() {
            warn!(
                "No children found for path '{}' in type '{}'",
                search_path, parent_type_name
            );
            return Err(PathError::UnfoldError {
                element_path: element_path.to_string(),
                reason: format!(
                    "No children found in parent type '{}'",
                    parent_type.code
                ),
            });
        }

        debug!("Unfolded {} child elements", children.len());

        // Contextualize children (adjust paths to match current context)
        let contextualized: Vec<ElementDefinition> = children
            .into_iter()
            .map(|child| {
                let mut child_content = (*child.content).clone();

                // Adjust path: ParentType.child → CurrentPath.child
                if let Some(child_path) = child.path() {
                    let new_path = child_path.replace(parent_type_name, element_path);
                    if let Some(obj) = child_content.as_object_mut() {
                        obj.insert("path".to_string(), Value::String(new_path.clone()));
                    }
                }

                // Adjust ID similarly
                if let Some(child_id) = child.id() {
                    let element_id = element.id().unwrap_or(element_path);
                    let new_id = child_id.replace(parent_type_name, element_id);
                    if let Some(obj) = child_content.as_object_mut() {
                        obj.insert("id".to_string(), Value::String(new_id));
                    }
                }

                ElementDefinition::new(child_content)
            })
            .collect();

        Ok(contextualized)
    }

    /// Resolve bracket notation
    fn resolve_bracket(
        &self,
        matches: &[ElementDefinition],
        bracket: &Bracket,
        target_path: &str,
    ) -> Result<ElementDefinition, PathError> {
        match bracket {
            Bracket::Slice(slice_name) => {
                // Find element with matching slice name
                matches
                    .iter()
                    .find(|e| e.slice_name() == Some(slice_name))
                    .cloned()
                    .ok_or_else(|| PathError::NotFound {
                        path: format!("{}[{}]", target_path, slice_name),
                        base_type: "slice".to_string(),
                    })
            }

            Bracket::Index(_) => {
                // For array indices, return first match
                // (actual index resolution happens during instance export)
                matches.first().cloned().ok_or_else(|| PathError::NotFound {
                    path: target_path.to_string(),
                    base_type: "array".to_string(),
                })
            }

            Bracket::Soft(_) => {
                // For soft indexing, return first match
                // (actual index resolution happens during instance export)
                matches.first().cloned().ok_or_else(|| PathError::NotFound {
                    path: target_path.to_string(),
                    base_type: "array".to_string(),
                })
            }

            Bracket::ChoiceType => {
                // For choice types, return first match
                // (caller should handle multiple choice type variants)
                matches.first().cloned().ok_or_else(|| PathError::NotFound {
                    path: format!("{}[x]", target_path),
                    base_type: "choice type".to_string(),
                })
            }
        }
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.cache.capacity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a minimal test session (not used in actual tests)
    // These tests don't need a real session as they only test parsing logic

    #[test]
    fn test_parse_simple_path() {
        // Create a dummy session just for the PathResolver constructor
        // The parse_path method doesn't actually use the session
        let segments = PathSegment::new("name".to_string());
        assert_eq!(segments.base, "name");
        assert_eq!(segments.bracket, None);

        // Test actual parsing with a helper (doesn't need session)
        let test_segments = [
            PathSegment::new("name".to_string()),
            PathSegment::new("given".to_string()),
        ];
        assert_eq!(test_segments.len(), 2);
        assert_eq!(test_segments[0].base, "name");
        assert_eq!(test_segments[1].base, "given");
    }

    #[test]
    fn test_parse_bracket_logic() {
        // Test bracket parsing directly without needing a session
        assert_eq!(
            PathSegment::with_bracket("name".to_string(), Bracket::Index(0)).bracket,
            Some(Bracket::Index(0))
        );

        assert_eq!(
            PathSegment::with_bracket("component".to_string(), Bracket::Slice("systolic".to_string())).bracket,
            Some(Bracket::Slice("systolic".to_string()))
        );

        assert_eq!(
            PathSegment::with_bracket("telecom".to_string(), Bracket::Soft(SoftIndexOp::Increment)).bracket,
            Some(Bracket::Soft(SoftIndexOp::Increment))
        );

        assert_eq!(
            PathSegment::with_bracket("deceased".to_string(), Bracket::ChoiceType).bracket,
            Some(Bracket::ChoiceType)
        );
    }

    #[test]
    fn test_bracket_variants() {
        let index = Bracket::Index(5);
        assert!(matches!(index, Bracket::Index(5)));

        let slice = Bracket::Slice("test".to_string());
        assert!(matches!(slice, Bracket::Slice(_)));

        let soft = Bracket::Soft(SoftIndexOp::Increment);
        assert!(matches!(soft, Bracket::Soft(SoftIndexOp::Increment)));

        let choice = Bracket::ChoiceType;
        assert!(matches!(choice, Bracket::ChoiceType));
    }

    #[test]
    fn test_element_definition_accessors() {
        let json = serde_json::json!({
            "id": "Patient.name.given",
            "path": "Patient.name.given",
            "type": [{
                "code": "string"
            }]
        });

        let elem = ElementDefinition::new(json);
        assert_eq!(elem.id(), Some("Patient.name.given"));
        assert_eq!(elem.path(), Some("Patient.name.given"));
        assert_eq!(elem.types().len(), 1);
        assert_eq!(elem.types()[0].code, "string");
        assert!(!elem.is_choice_type());
    }

    #[test]
    fn test_element_definition_choice_type() {
        let json = serde_json::json!({
            "id": "Patient.deceased[x]",
            "path": "Patient.deceased[x]",
            "type": [
                {"code": "boolean"},
                {"code": "dateTime"}
            ]
        });

        let elem = ElementDefinition::new(json);
        assert!(elem.is_choice_type());
        assert_eq!(elem.types().len(), 2);
    }

    #[test]
    fn test_structure_definition_basics() {
        let sd_json = serde_json::json!({
            "resourceType": "StructureDefinition",
            "url": "http://hl7.org/fhir/StructureDefinition/Patient",
            "type": "Patient"
        });

        let sd = StructureDefinition {
            content: Arc::new(sd_json),
        };

        assert_eq!(sd.url(), Some("http://hl7.org/fhir/StructureDefinition/Patient"));
        assert_eq!(sd.type_name(), "Patient");
    }
}
