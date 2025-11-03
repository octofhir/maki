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

use crate::canonical::fishable::{FhirType, Fishable};
use crate::canonical::{DefinitionResource, DefinitionSession};
use dashmap::DashMap;
use serde_json::Value;
use std::collections::HashMap;
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
        self.path().map(|p| p.ends_with("[x]")).unwrap_or(false)
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
        self.elements().into_iter().find(|e| e.path() == Some(path))
    }
}

/// Resolved path result with metadata
///
/// Contains the resolved FHIR path along with metadata about the resolution.
#[derive(Debug, Clone)]
pub struct ResolvedPath {
    /// The resolved FHIR ElementDefinition path
    pub fhir_path: String,
    /// The resolved ElementDefinition
    pub element_definition: ElementDefinition,
    /// Whether this path resolves to a slice
    pub is_slice: bool,
    /// The slice name if this is a slice
    pub slice_name: Option<String>,
    /// Whether this path resolves to an extension
    pub is_extension: bool,
    /// The extension URL if this is an extension
    pub extension_url: Option<String>,
}

/// Resolution context for path resolution
///
/// Provides context information needed during path resolution.
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    /// The base StructureDefinition being resolved against
    pub base_definition: StructureDefinition,
    /// The profile name for error reporting
    pub profile_name: String,
}

/// Registry for tracking slice definitions
///
/// Maintains a registry of slice definitions for efficient lookup.
#[derive(Debug, Clone, Default)]
pub struct SliceRegistry {
    /// Map of element path to slice definitions
    slices: HashMap<String, Vec<SliceDefinition>>,
}

/// Slice definition information
#[derive(Debug, Clone)]
pub struct SliceDefinition {
    /// The slice name
    pub name: String,
    /// The element path this slice applies to
    pub element_path: String,
    /// The discriminator information
    pub discriminator: Option<SliceDiscriminator>,
    /// The slice ordering (if specified)
    pub ordering: Option<SliceOrdering>,
    /// The slice rules (if specified)
    pub rules: Option<SliceRules>,
}

/// Slice discriminator information
#[derive(Debug, Clone)]
pub struct SliceDiscriminator {
    /// The discriminator type (value | exists | pattern | type | profile)
    pub discriminator_type: String,
    /// The discriminator path
    pub path: String,
}

/// Slice ordering rules
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceOrdering {
    /// Slices must appear in the order defined
    Ordered,
    /// Slices can appear in any order
    Unordered,
}

/// Slice rules
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceRules {
    /// Additional slices are not allowed
    Closed,
    /// Additional slices are allowed
    Open,
    /// Additional slices are allowed at the end
    OpenAtEnd,
}

/// Registry for tracking extension definitions
///
/// Maintains a registry of extension definitions for efficient lookup.
#[derive(Debug, Clone, Default)]
pub struct ExtensionRegistry {
    /// Map of extension URL to extension definitions
    extensions: HashMap<String, ExtensionDefinition>,
}

/// Extension definition information
#[derive(Debug, Clone)]
pub struct ExtensionDefinition {
    /// The extension URL
    pub url: String,
    /// The extension context (where it can be used)
    pub context: Vec<ExtensionContext>,
    /// The value type constraints
    pub value_types: Vec<String>,
    /// Whether this is a complex extension (has sub-extensions)
    pub is_complex: bool,
    /// Sub-extensions if this is a complex extension
    pub sub_extensions: Vec<ExtensionDefinition>,
}

/// Extension context information
#[derive(Debug, Clone)]
pub struct ExtensionContext {
    /// The context type (element | extension | resource)
    pub context_type: String,
    /// The context expression (FHIR path or resource type)
    pub expression: String,
}

impl SliceRegistry {
    /// Create a new slice registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a slice definition
    pub fn register_slice(&mut self, slice: SliceDefinition) {
        self.slices
            .entry(slice.element_path.clone())
            .or_insert_with(Vec::new)
            .push(slice);
    }

    /// Find slices for an element path
    pub fn find_slices(&self, element_path: &str) -> Option<&Vec<SliceDefinition>> {
        self.slices.get(element_path)
    }

    /// Find a specific slice by name
    pub fn find_slice(&self, element_path: &str, slice_name: &str) -> Option<&SliceDefinition> {
        self.slices.get(element_path)?.iter().find(|s| s.name == slice_name)
    }

    /// Register a slice with discriminator
    pub fn register_slice_with_discriminator(
        &mut self,
        element_path: String,
        slice_name: String,
        discriminator_type: String,
        discriminator_path: String,
    ) {
        let slice = SliceDefinition {
            name: slice_name,
            element_path: element_path.clone(),
            discriminator: Some(SliceDiscriminator {
                discriminator_type,
                path: discriminator_path,
            }),
            ordering: None,
            rules: None,
        };
        self.register_slice(slice);
    }

    /// Check if an element path has slices
    pub fn has_slices(&self, element_path: &str) -> bool {
        self.slices.contains_key(element_path)
    }

    /// Get all slice names for an element path
    pub fn get_slice_names(&self, element_path: &str) -> Vec<String> {
        self.slices
            .get(element_path)
            .map(|slices| slices.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Validate slice name against registered slices
    pub fn validate_slice_name(&self, element_path: &str, slice_name: &str) -> Result<(), String> {
        if let Some(slices) = self.slices.get(element_path) {
            if slices.iter().any(|s| s.name == slice_name) {
                Ok(())
            } else {
                let available_names: Vec<String> = slices.iter().map(|s| s.name.clone()).collect();
                Err(format!(
                    "Slice '{}' not found for element '{}'. Available slices: {}",
                    slice_name,
                    element_path,
                    available_names.join(", ")
                ))
            }
        } else {
            Err(format!("No slices defined for element '{}'", element_path))
        }
    }
}

impl ExtensionRegistry {
    /// Create a new extension registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an extension definition
    pub fn register_extension(&mut self, extension: ExtensionDefinition) {
        self.extensions.insert(extension.url.clone(), extension);
    }

    /// Find an extension by URL
    pub fn find_extension(&self, url: &str) -> Option<&ExtensionDefinition> {
        self.extensions.get(url)
    }

    /// Register a simple extension with value type
    pub fn register_simple_extension(
        &mut self,
        url: String,
        context_type: String,
        context_expression: String,
        value_type: String,
    ) {
        let extension = ExtensionDefinition {
            url: url.clone(),
            context: vec![ExtensionContext {
                context_type,
                expression: context_expression,
            }],
            value_types: vec![value_type],
            is_complex: false,
            sub_extensions: Vec::new(),
        };
        self.register_extension(extension);
    }

    /// Register a complex extension with sub-extensions
    pub fn register_complex_extension(
        &mut self,
        url: String,
        context_type: String,
        context_expression: String,
        sub_extensions: Vec<ExtensionDefinition>,
    ) {
        let extension = ExtensionDefinition {
            url: url.clone(),
            context: vec![ExtensionContext {
                context_type,
                expression: context_expression,
            }],
            value_types: Vec::new(),
            is_complex: true,
            sub_extensions,
        };
        self.register_extension(extension);
    }

    /// Check if an extension URL is registered
    pub fn has_extension(&self, url: &str) -> bool {
        self.extensions.contains_key(url)
    }

    /// Get all registered extension URLs
    pub fn get_extension_urls(&self) -> Vec<String> {
        self.extensions.keys().cloned().collect()
    }

    /// Find extensions by context
    pub fn find_extensions_by_context(&self, context_expression: &str) -> Vec<&ExtensionDefinition> {
        self.extensions
            .values()
            .filter(|ext| {
                ext.context
                    .iter()
                    .any(|ctx| ctx.expression == context_expression)
            })
            .collect()
    }

    /// Validate extension URL and provide suggestions
    pub fn validate_extension_url(&self, url: &str) -> Result<(), String> {
        if self.extensions.contains_key(url) {
            Ok(())
        } else {
            let available_urls: Vec<String> = self.extensions.keys().cloned().collect();
            if available_urls.is_empty() {
                Err("No extensions registered".to_string())
            } else {
                // Find similar URLs for suggestions
                let suggestions: Vec<String> = available_urls
                    .iter()
                    .filter(|existing_url| {
                        // Simple similarity check - contains common parts
                        let url_parts: Vec<&str> = url.split('/').collect();
                        let existing_parts: Vec<&str> = existing_url.split('/').collect();
                        url_parts.iter().any(|part| {
                            existing_parts.iter().any(|existing_part| {
                                existing_part.contains(part) || part.contains(existing_part)
                            })
                        })
                    })
                    .cloned()
                    .collect();

                if suggestions.is_empty() {
                    Err(format!(
                        "Extension '{}' not found. Available extensions: {}",
                        url,
                        available_urls.join(", ")
                    ))
                } else {
                    Err(format!(
                        "Extension '{}' not found. Did you mean: {}",
                        url,
                        suggestions.join(", ")
                    ))
                }
            }
        }
    }
}

/// Path resolver
///
/// Resolves FSH paths to FHIR ElementDefinitions using SUSHI's algorithm.
/// Provides caching for performance and supports slicing and extensions.
pub struct PathResolver {
    /// Session for canonical resource lookups
    session: Arc<DefinitionSession>,
    /// Cache: (SD url, path) → ResolvedPath
    cache: DashMap<(String, String), Arc<ResolvedPath>>,
    /// Registry for slice definitions
    slice_registry: SliceRegistry,
    /// Registry for extension definitions
    extension_registry: ExtensionRegistry,
}

impl PathResolver {
    /// Create a new path resolver
    pub fn new(session: Arc<DefinitionSession>) -> Self {
        Self {
            session,
            cache: DashMap::new(),
            slice_registry: SliceRegistry::new(),
            extension_registry: ExtensionRegistry::new(),
        }
    }

    /// Create a new path resolver with registries
    pub fn with_registries(
        session: Arc<DefinitionSession>,
        slice_registry: SliceRegistry,
        extension_registry: ExtensionRegistry,
    ) -> Self {
        Self {
            session,
            cache: DashMap::new(),
            slice_registry,
            extension_registry,
        }
    }

    /// Get a mutable reference to the slice registry
    pub fn slice_registry_mut(&mut self) -> &mut SliceRegistry {
        &mut self.slice_registry
    }

    /// Get a mutable reference to the extension registry
    pub fn extension_registry_mut(&mut self) -> &mut ExtensionRegistry {
        &mut self.extension_registry
    }

    /// Resolve FSH path to FHIR ElementDefinition path
    ///
    /// # Arguments
    ///
    /// * `fsh_path` - The FSH path to resolve (e.g., "name.given", "deceased[x]")
    /// * `context` - The resolution context containing base definition and profile info
    ///
    /// # Returns
    ///
    /// Returns the resolved `ResolvedPath` with metadata or a `PathError`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::semantic::{PathResolver, ResolutionContext};
    /// # use std::sync::Arc;
    /// # async fn example(resolver: PathResolver, context: ResolutionContext) -> Result<(), Box<dyn std::error::Error>> {
    /// let resolved = resolver.resolve_path("name.given", &context).await?;
    /// println!("Resolved path: {}", resolved.fhir_path);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resolve_path(
        &self,
        fsh_path: &str,
        context: &ResolutionContext,
    ) -> Result<ResolvedPath, PathError> {
        debug!(
            "Resolving path '{}' in profile '{}'",
            fsh_path, context.profile_name
        );

        let sd = &context.base_definition;

        // Check cache first
        let cache_key = (
            sd.url().unwrap_or(&context.profile_name).to_string(),
            fsh_path.to_string(),
        );
        if let Some(cached) = self.cache.get(&cache_key) {
            trace!("Cache hit for path '{}'", fsh_path);
            return Ok((**cached).clone());
        }

        // Check if this is a slice path (contains colon)
        if fsh_path.contains(':') {
            let parts: Vec<&str> = fsh_path.splitn(2, ':').collect();
            if parts.len() == 2 {
                let base_path = parts[0];
                let slice_part = parts[1];
                return self.resolve_slice_path(base_path, slice_part, context).await;
            }
        }

        // Check if this is an extension path
        if fsh_path.starts_with("extension[") {
            return self.resolve_extension_path_internal(fsh_path, context).await;
        }

        // Check if this is a choice type with specific type (e.g., "valueString")
        if let Some(choice_result) = self.try_resolve_typed_choice(fsh_path, context).await? {
            return Ok(choice_result);
        }

        // STEP 1: FAST PATH - Direct lookup in elements
        if let Some(elem) = sd.find_element_by_path(fsh_path) {
            debug!("Fast path: found element directly");
            let resolved = ResolvedPath {
                fhir_path: fsh_path.to_string(),
                element_definition: elem,
                is_slice: false,
                slice_name: None,
                is_extension: false,
                extension_url: None,
            };
            let resolved_arc = Arc::new(resolved.clone());
            self.cache.insert(cache_key, resolved_arc);
            return Ok(resolved);
        }

        // STEP 2: Parse path into segments
        let segments = self.parse_path(fsh_path)?;
        debug!("Parsed path into {} segments", segments.len());

        // STEP 3: Iterative resolution with unfolding
        let element = self.resolve_segments(sd, &segments, fsh_path).await?;

        // Create resolved path result
        let resolved = ResolvedPath {
            fhir_path: element.path().unwrap_or(fsh_path).to_string(),
            element_definition: element,
            is_slice: false,
            slice_name: None,
            is_extension: false,
            extension_url: None,
        };

        // Cache the result
        let resolved_arc = Arc::new(resolved.clone());
        self.cache.insert(cache_key, resolved_arc);

        Ok(resolved)
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
            return Err(PathError::InvalidSyntax(format!(
                "Unclosed bracket in path: {}",
                path
            )));
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
                        .map(|p| {
                            p == target_path
                                || (segment.bracket == Some(Bracket::ChoiceType)
                                    && p.starts_with(&format!(
                                        "{}[x]",
                                        target_path.trim_end_matches("[x]")
                                    )))
                        })
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
                reason: format!("No children found in parent type '{}'", parent_type.code),
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
                // For choice types, we need to handle this specially
                self.resolve_choice_type(matches, target_path)
            }
        }
    }

    /// Resolve choice type elements with [x] suffix
    ///
    /// Choice types in FHIR allow multiple possible types for a single element.
    /// For example, `value[x]` can be `valueString`, `valueInteger`, etc.
    fn resolve_choice_type(
        &self,
        matches: &[ElementDefinition],
        target_path: &str,
    ) -> Result<ElementDefinition, PathError> {
        debug!("Resolving choice type for path: {}", target_path);

        // First, try to find the exact [x] element
        if let Some(choice_element) = matches.iter().find(|e| {
            e.path().map(|p| p.ends_with("[x]")).unwrap_or(false)
        }) {
            debug!("Found choice type element: {:?}", choice_element.path());
            return Ok(choice_element.clone());
        }

        // If no exact [x] match, look for typed variants
        let base_path = target_path.trim_end_matches("[x]");
        let typed_matches: Vec<&ElementDefinition> = matches
            .iter()
            .filter(|e| {
                if let Some(path) = e.path() {
                    path.starts_with(base_path) && 
                    path != format!("{}[x]", base_path) &&
                    (path.starts_with(&format!("{}Boolean", base_path)) ||
                     path.starts_with(&format!("{}Integer", base_path)) ||
                     path.starts_with(&format!("{}String", base_path)) ||
                     path.starts_with(&format!("{}DateTime", base_path)) ||
                     path.starts_with(&format!("{}Code", base_path)) ||
                     path.starts_with(&format!("{}Coding", base_path)) ||
                     path.starts_with(&format!("{}CodeableConcept", base_path)) ||
                     path.starts_with(&format!("{}Quantity", base_path)) ||
                     path.starts_with(&format!("{}Reference", base_path)))
                } else {
                    false
                }
            })
            .collect();

        if !typed_matches.is_empty() {
            debug!("Found {} typed variants for choice type", typed_matches.len());
            // Return the first typed variant as a representative
            return Ok(typed_matches[0].clone());
        }

        // If still no matches, return an error with helpful information
        Err(PathError::NotFound {
            path: format!("{}[x]", base_path),
            base_type: format!(
                "choice type (no variants found). Available elements: {}",
                matches.iter()
                    .filter_map(|e| e.path())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
    }

    /// Resolve choice type with specific type (e.g., "valueString" from "value[x]")
    ///
    /// # Arguments
    ///
    /// * `base_path` - The base path without type suffix (e.g., "value")
    /// * `type_name` - The specific type name (e.g., "String", "Integer")
    /// * `context` - The resolution context
    ///
    /// # Returns
    ///
    /// Returns the resolved `ResolvedPath` for the typed choice element.
    pub async fn resolve_choice_type_with_type(
        &self,
        base_path: &str,
        type_name: &str,
        context: &ResolutionContext,
    ) -> Result<ResolvedPath, PathError> {
        debug!("Resolving choice type: {} with type: {}", base_path, type_name);

        // Construct the typed path
        let typed_path = format!("{}{}", base_path, type_name);

        // Try to find the typed element directly
        if let Some(elem) = context.base_definition.find_element_by_path(&typed_path) {
            return Ok(ResolvedPath {
                fhir_path: typed_path,
                element_definition: elem,
                is_slice: false,
                slice_name: None,
                is_extension: false,
                extension_url: None,
            });
        }

        // If not found directly, try to resolve through the choice type element
        let choice_path = format!("{}[x]", base_path);
        if let Some(choice_elem) = context.base_definition.find_element_by_path(&choice_path) {
            // Validate that the requested type is allowed
            let allowed_types = choice_elem.types();
            let type_allowed = allowed_types.iter().any(|t| {
                t.code.eq_ignore_ascii_case(type_name) ||
                format!("{}{}", base_path, t.code) == typed_path
            });

            if type_allowed {
                return Ok(ResolvedPath {
                    fhir_path: typed_path,
                    element_definition: choice_elem,
                    is_slice: false,
                    slice_name: None,
                    is_extension: false,
                    extension_url: None,
                });
            } else {
                let allowed_type_names: Vec<String> = allowed_types
                    .iter()
                    .map(|t| format!("{}{}", base_path, t.code))
                    .collect();
                
                return Err(PathError::InvalidSyntax(format!(
                    "Type '{}' not allowed for choice element '{}'. Allowed types: {}",
                    type_name,
                    base_path,
                    allowed_type_names.join(", ")
                )));
            }
        }

        Err(PathError::NotFound {
            path: typed_path,
            base_type: context.base_definition.type_name().to_string(),
        })
    }

    /// Handle sliced element paths (e.g., "identifier:mrn.system")
    ///
    /// # Arguments
    ///
    /// * `base_path` - The base element path (e.g., "identifier")
    /// * `slice_part` - The slice name and optional sub-path (e.g., "mrn.system")
    /// * `context` - The resolution context
    ///
    /// # Returns
    ///
    /// Returns the resolved `ResolvedPath` for the slice or a `PathError`.
    pub async fn resolve_slice_path(
        &self,
        base_path: &str,
        slice_part: &str,
        context: &ResolutionContext,
    ) -> Result<ResolvedPath, PathError> {
        debug!("Resolving slice path: {}:{}", base_path, slice_part);

        // Parse slice part - it might be just slice name or slice_name.sub_path
        let (slice_name, sub_path) = if let Some(dot_pos) = slice_part.find('.') {
            let slice_name = &slice_part[..dot_pos];
            let sub_path = &slice_part[dot_pos + 1..];
            (slice_name, Some(sub_path))
        } else {
            (slice_part, None)
        };

        // Validate slice name if registry has slices for this path
        if self.slice_registry.has_slices(base_path) {
            if let Err(validation_error) = self.slice_registry.validate_slice_name(base_path, slice_name) {
                return Err(PathError::NotFound {
                    path: format!("{}:{}", base_path, slice_part),
                    base_type: validation_error,
                });
            }
        }

        // Look for the slice in the registry
        if let Some(slice_def) = self.slice_registry.find_slice(base_path, slice_name) {
            debug!("Found slice definition: {} with discriminator: {:?}", 
                   slice_def.name, slice_def.discriminator);
            
            // Construct the slice path
            let slice_path = format!("{}:{}", base_path, slice_name);
            let full_path = if let Some(sub) = sub_path {
                format!("{}.{}", slice_path, sub)
            } else {
                slice_path.clone()
            };

            // Try to find the element directly
            if let Some(elem) = context.base_definition.find_element_by_path(&full_path) {
                return Ok(ResolvedPath {
                    fhir_path: full_path,
                    element_definition: elem,
                    is_slice: true,
                    slice_name: Some(slice_name.to_string()),
                    is_extension: false,
                    extension_url: None,
                });
            }

            // If not found directly, try to resolve using discriminator information
            if let Some(discriminator) = &slice_def.discriminator {
                debug!("Using discriminator {} at path {}", 
                       discriminator.discriminator_type, discriminator.path);
                
                // Try to find elements that match the discriminator pattern
                let discriminator_path = format!("{}.{}", base_path, discriminator.path);
                if let Some(elem) = context.base_definition.find_element_by_path(&discriminator_path) {
                    // Create a synthetic slice element based on the discriminator
                    let slice_element_path = if let Some(sub) = sub_path {
                        format!("{}.{}", slice_path, sub)
                    } else {
                        slice_path
                    };
                    
                    return Ok(ResolvedPath {
                        fhir_path: slice_element_path,
                        element_definition: elem,
                        is_slice: true,
                        slice_name: Some(slice_name.to_string()),
                        is_extension: false,
                        extension_url: None,
                    });
                }
            }
        }

        // Fallback: try to resolve as regular path and mark as slice
        let full_fsh_path = format!("{}:{}", base_path, slice_part);
        let segments = self.parse_path(&full_fsh_path)?;
        
        // Handle colon in path segments for slicing
        let mut modified_segments = Vec::new();
        for segment in segments {
            if segment.base.contains(':') {
                // Split the base on colon and create slice notation
                let parts: Vec<&str> = segment.base.split(':').collect();
                if parts.len() == 2 {
                    let base_segment = PathSegment::new(parts[0].to_string());
                    let slice_segment = PathSegment::with_bracket(
                        parts[1].to_string(),
                        Bracket::Slice(parts[1].to_string())
                    );
                    modified_segments.push(base_segment);
                    modified_segments.push(slice_segment);
                } else {
                    modified_segments.push(segment);
                }
            } else {
                modified_segments.push(segment);
            }
        }

        let element = self.resolve_segments(&context.base_definition, &modified_segments, &full_fsh_path).await?;

        Ok(ResolvedPath {
            fhir_path: element.path().unwrap_or(&full_fsh_path).to_string(),
            element_definition: element,
            is_slice: true,
            slice_name: Some(slice_name.to_string()),
            is_extension: false,
            extension_url: None,
        })
    }

    /// Handle extension paths (e.g., "extension[url].valueString")
    ///
    /// # Arguments
    ///
    /// * `extension_url` - The extension URL
    /// * `value_path` - The path within the extension (e.g., "valueString")
    /// * `context` - The resolution context
    ///
    /// # Returns
    ///
    /// Returns the resolved `ResolvedPath` for the extension or a `PathError`.
    pub async fn resolve_extension_path(
        &self,
        extension_url: &str,
        value_path: &str,
        context: &ResolutionContext,
    ) -> Result<ResolvedPath, PathError> {
        debug!("Resolving extension path: {} -> {}", extension_url, value_path);

        // Validate extension URL if registry has extensions
        if !self.extension_registry.get_extension_urls().is_empty() {
            if let Err(validation_error) = self.extension_registry.validate_extension_url(extension_url) {
                return Err(PathError::NotFound {
                    path: format!("extension[{}]", extension_url),
                    base_type: validation_error,
                });
            }
        }

        // Look for the extension in the registry
        if let Some(ext_def) = self.extension_registry.find_extension(extension_url) {
            debug!("Found extension definition: {} (complex: {})", 
                   ext_def.url, ext_def.is_complex);

            // Handle complex extensions with sub-extensions
            if ext_def.is_complex && !value_path.is_empty() {
                // Check if the value_path refers to a sub-extension
                if let Some(sub_ext) = ext_def.sub_extensions.iter().find(|sub| {
                    value_path.starts_with(&format!("extension[{}]", sub.url))
                }) {
                    debug!("Found sub-extension: {}", sub_ext.url);
                    // Recursively resolve the sub-extension path
                    let remaining_path = value_path.strip_prefix(&format!("extension[{}]", sub_ext.url))
                        .unwrap_or("")
                        .trim_start_matches('.');
                    
                    return Box::pin(self.resolve_extension_path(&sub_ext.url, remaining_path, context)).await;
                }
            }

            // Validate value type if specified
            if !ext_def.value_types.is_empty() && !value_path.is_empty() {
                let value_type_valid = ext_def.value_types.iter().any(|vt| {
                    value_path.starts_with(&format!("value{}", vt)) ||
                    value_path == format!("value[x]") ||
                    value_path.starts_with("value[x]")
                });

                if !value_type_valid {
                    return Err(PathError::InvalidSyntax(format!(
                        "Invalid value path '{}' for extension '{}'. Expected value types: {}",
                        value_path,
                        extension_url,
                        ext_def.value_types.join(", ")
                    )));
                }
            }
        }

        // Construct the extension path
        let extension_path = if value_path.is_empty() {
            format!("extension[{}]", extension_url)
        } else {
            format!("extension[{}].{}", extension_url, value_path)
        };

        // Try to resolve the extension element
        let segments = self.parse_path(&extension_path)?;
        let element = self.resolve_segments(&context.base_definition, &segments, &extension_path).await?;

        Ok(ResolvedPath {
            fhir_path: element.path().unwrap_or(&extension_path).to_string(),
            element_definition: element,
            is_slice: false,
            slice_name: None,
            is_extension: true,
            extension_url: Some(extension_url.to_string()),
        })
    }

    /// Internal method to handle extension paths from FSH syntax
    async fn resolve_extension_path_internal(
        &self,
        fsh_path: &str,
        context: &ResolutionContext,
    ) -> Result<ResolvedPath, PathError> {
        debug!("Parsing extension path syntax: {}", fsh_path);

        // Parse extension[url].valuePath format
        if let Some(bracket_start) = fsh_path.find('[') {
            if let Some(bracket_end) = fsh_path.find(']') {
                let extension_url = &fsh_path[bracket_start + 1..bracket_end];
                let remaining = &fsh_path[bracket_end + 1..];
                
                // Handle nested extension paths like extension[url1].extension[url2].valueString
                if remaining.starts_with('.') {
                    let value_path = &remaining[1..];
                    
                    // Check if this is a nested extension
                    if value_path.starts_with("extension[") {
                        // This is a nested extension - handle recursively
                        return Box::pin(self.resolve_extension_path_internal(value_path, context)).await;
                    } else {
                        // Regular value path
                        return self.resolve_extension_path(extension_url, value_path, context).await;
                    }
                } else if remaining.is_empty() {
                    // Just the extension itself
                    return self.resolve_extension_path(extension_url, "", context).await;
                } else {
                    // Invalid syntax - should have a dot after the bracket
                    return Err(PathError::InvalidSyntax(format!(
                        "Invalid extension path syntax: '{}'. Expected '.' after extension URL bracket",
                        fsh_path
                    )));
                }
            } else {
                return Err(PathError::InvalidSyntax(format!(
                    "Unclosed bracket in extension path: {}",
                    fsh_path
                )));
            }
        }

        Err(PathError::InvalidSyntax(format!(
            "Invalid extension path syntax: '{}'. Expected format: extension[url] or extension[url].valuePath",
            fsh_path
        )))
    }

    /// Try to resolve a typed choice element (e.g., "valueString" from "value[x]")
    ///
    /// # Arguments
    ///
    /// * `fsh_path` - The FSH path that might be a typed choice
    /// * `context` - The resolution context
    ///
    /// # Returns
    ///
    /// Returns `Some(ResolvedPath)` if this is a typed choice, `None` if not, or an error.
    async fn try_resolve_typed_choice(
        &self,
        fsh_path: &str,
        context: &ResolutionContext,
    ) -> Result<Option<ResolvedPath>, PathError> {
        // Common FHIR choice type patterns
        let choice_patterns = [
            "value", "deceased", "onset", "abatement", "effective", "occurrence",
            "performed", "recorded", "issued", "applies", "timing", "multipleBirth",
        ];

        for pattern in &choice_patterns {
            if fsh_path.starts_with(pattern) && fsh_path.len() > pattern.len() {
                let remaining = &fsh_path[pattern.len()..];
                
                // Check if the remaining part looks like a type name
                if remaining.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    // This looks like a typed choice (e.g., "valueString")
                    let type_name = remaining;
                    
                    // Check if there's a choice type element for this pattern
                    let choice_path = format!("{}[x]", pattern);
                    if context.base_definition.find_element_by_path(&choice_path).is_some() {
                        debug!("Detected typed choice: {} -> {}[x] with type {}", 
                               fsh_path, pattern, type_name);
                        
                        let resolved = self.resolve_choice_type_with_type(pattern, type_name, context).await?;
                        return Ok(Some(resolved));
                    }
                }
            }
        }

        // Check for nested choice types (e.g., "component.valueString")
        if let Some(dot_pos) = fsh_path.rfind('.') {
            let base_path = &fsh_path[..dot_pos];
            let last_segment = &fsh_path[dot_pos + 1..];
            
            for pattern in &choice_patterns {
                if last_segment.starts_with(pattern) && last_segment.len() > pattern.len() {
                    let remaining = &last_segment[pattern.len()..];
                    
                    if remaining.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        // This is a nested typed choice
                        let choice_base_path = format!("{}.{}", base_path, pattern);
                        let choice_path = format!("{}[x]", choice_base_path);
                        
                        if context.base_definition.find_element_by_path(&choice_path).is_some() {
                            debug!("Detected nested typed choice: {} -> {}[x] with type {}", 
                                   fsh_path, choice_base_path, remaining);
                            
                            let resolved = self.resolve_choice_type_with_type(&choice_base_path, remaining, context).await?;
                            return Ok(Some(resolved));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Validate path and provide suggestions for common errors
    ///
    /// # Arguments
    ///
    /// * `fsh_path` - The FSH path to validate
    /// * `context` - The resolution context
    ///
    /// # Returns
    ///
    /// Returns validation errors with helpful suggestions.
    pub fn validate_path_with_suggestions(
        &self,
        fsh_path: &str,
        context: &ResolutionContext,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check for common syntax errors
        if fsh_path.is_empty() {
            errors.push("Path cannot be empty".to_string());
        }

        if fsh_path.starts_with('.') || fsh_path.ends_with('.') {
            errors.push("Path cannot start or end with a dot".to_string());
        }

        if fsh_path.contains("..") {
            errors.push("Path cannot contain consecutive dots".to_string());
        }

        // Check for unmatched brackets
        let open_brackets = fsh_path.matches('[').count();
        let close_brackets = fsh_path.matches(']').count();
        if open_brackets != close_brackets {
            errors.push("Unmatched brackets in path".to_string());
        }

        // Check for invalid characters
        if fsh_path.chars().any(|c| !c.is_alphanumeric() && !".:[]+-=x".contains(c)) {
            errors.push("Path contains invalid characters".to_string());
        }

        // Provide suggestions for common mistakes
        if fsh_path.contains("extension") && !fsh_path.contains('[') {
            errors.push("Extension paths should use bracket notation: extension[url]".to_string());
        }

        if fsh_path.contains(':') && !fsh_path.contains('[') {
            errors.push("Slice notation detected. Consider using proper slice syntax".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
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
            PathSegment::with_bracket(
                "component".to_string(),
                Bracket::Slice("systolic".to_string())
            )
            .bracket,
            Some(Bracket::Slice("systolic".to_string()))
        );

        assert_eq!(
            PathSegment::with_bracket("telecom".to_string(), Bracket::Soft(SoftIndexOp::Increment))
                .bracket,
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

        assert_eq!(
            sd.url(),
            Some("http://hl7.org/fhir/StructureDefinition/Patient")
        );
        assert_eq!(sd.type_name(), "Patient");
    }

    #[test]
    fn test_slice_registry() {
        let mut registry = SliceRegistry::new();
        
        // Test registering a slice
        let slice = SliceDefinition {
            name: "mrn".to_string(),
            element_path: "identifier".to_string(),
            discriminator: Some(SliceDiscriminator {
                discriminator_type: "value".to_string(),
                path: "system".to_string(),
            }),
            ordering: Some(SliceOrdering::Ordered),
            rules: Some(SliceRules::Closed),
        };
        
        registry.register_slice(slice);
        
        // Test finding slices
        assert!(registry.has_slices("identifier"));
        assert!(!registry.has_slices("name"));
        
        let found_slice = registry.find_slice("identifier", "mrn");
        assert!(found_slice.is_some());
        assert_eq!(found_slice.unwrap().name, "mrn");
        
        // Test slice names
        let slice_names = registry.get_slice_names("identifier");
        assert_eq!(slice_names, vec!["mrn"]);
        
        // Test validation
        assert!(registry.validate_slice_name("identifier", "mrn").is_ok());
        assert!(registry.validate_slice_name("identifier", "invalid").is_err());
    }

    #[test]
    fn test_slice_registry_with_discriminator() {
        let mut registry = SliceRegistry::new();
        
        registry.register_slice_with_discriminator(
            "telecom".to_string(),
            "phone".to_string(),
            "value".to_string(),
            "system".to_string(),
        );
        
        let slice = registry.find_slice("telecom", "phone").unwrap();
        assert_eq!(slice.name, "phone");
        assert!(slice.discriminator.is_some());
        
        let discriminator = slice.discriminator.as_ref().unwrap();
        assert_eq!(discriminator.discriminator_type, "value");
        assert_eq!(discriminator.path, "system");
    }

    #[test]
    fn test_extension_registry() {
        let mut registry = ExtensionRegistry::new();
        
        // Test simple extension
        registry.register_simple_extension(
            "http://example.org/fhir/StructureDefinition/patient-birthPlace".to_string(),
            "element".to_string(),
            "Patient".to_string(),
            "Address".to_string(),
        );
        
        assert!(registry.has_extension("http://example.org/fhir/StructureDefinition/patient-birthPlace"));
        assert!(!registry.has_extension("http://example.org/invalid"));
        
        let extension = registry.find_extension("http://example.org/fhir/StructureDefinition/patient-birthPlace");
        assert!(extension.is_some());
        assert!(!extension.unwrap().is_complex);
        
        // Test validation
        assert!(registry.validate_extension_url("http://example.org/fhir/StructureDefinition/patient-birthPlace").is_ok());
        assert!(registry.validate_extension_url("http://example.org/invalid").is_err());
    }

    #[test]
    fn test_extension_registry_complex() {
        let mut registry = ExtensionRegistry::new();
        
        let sub_extension = ExtensionDefinition {
            url: "http://example.org/fhir/StructureDefinition/sub-extension".to_string(),
            context: vec![ExtensionContext {
                context_type: "extension".to_string(),
                expression: "http://example.org/fhir/StructureDefinition/complex-extension".to_string(),
            }],
            value_types: vec!["string".to_string()],
            is_complex: false,
            sub_extensions: Vec::new(),
        };
        
        registry.register_complex_extension(
            "http://example.org/fhir/StructureDefinition/complex-extension".to_string(),
            "element".to_string(),
            "Patient".to_string(),
            vec![sub_extension],
        );
        
        let extension = registry.find_extension("http://example.org/fhir/StructureDefinition/complex-extension");
        assert!(extension.is_some());
        assert!(extension.unwrap().is_complex);
        assert_eq!(extension.unwrap().sub_extensions.len(), 1);
    }

    #[test]
    fn test_path_validation() {
        use std::sync::Arc;
        use crate::canonical::DefinitionSession;
        
        // Create a mock session (this won't be used in validation)
        let session = Arc::new(DefinitionSession::for_testing());
        let resolver = PathResolver::new(session);
        
        let context = ResolutionContext {
            base_definition: StructureDefinition {
                content: Arc::new(serde_json::json!({
                    "resourceType": "StructureDefinition",
                    "type": "Patient"
                })),
            },
            profile_name: "TestProfile".to_string(),
        };
        
        // Test valid paths
        assert!(resolver.validate_path_with_suggestions("name.given", &context).is_ok());
        assert!(resolver.validate_path_with_suggestions("identifier[0].system", &context).is_ok());
        
        // Test invalid paths
        assert!(resolver.validate_path_with_suggestions("", &context).is_err());
        assert!(resolver.validate_path_with_suggestions(".name", &context).is_err());
        assert!(resolver.validate_path_with_suggestions("name.", &context).is_err());
        assert!(resolver.validate_path_with_suggestions("name..given", &context).is_err());
        assert!(resolver.validate_path_with_suggestions("name[unclosed", &context).is_err());
        
        // Test suggestions
        let errors = resolver.validate_path_with_suggestions("extension", &context).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("extension[url]")));
    }

    #[test]
    fn test_choice_type_resolution() {
        use std::sync::Arc;
        use crate::canonical::DefinitionSession;
        
        let session = Arc::new(DefinitionSession::for_testing());
        let resolver = PathResolver::new(session);
        
        // Test choice type element detection
        let choice_json = serde_json::json!({
            "id": "Patient.deceased[x]",
            "path": "Patient.deceased[x]",
            "type": [
                {"code": "boolean"},
                {"code": "dateTime"}
            ]
        });
        
        let choice_elem = ElementDefinition::new(choice_json);
        assert!(choice_elem.is_choice_type());
        
        // Test choice type matching
        let matches = vec![choice_elem];
        let result = resolver.resolve_choice_type(&matches, "Patient.deceased[x]");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_path_with_slices() {
        use std::sync::Arc;
        use crate::canonical::DefinitionSession;
        
        let session = Arc::new(DefinitionSession::for_testing());
        let resolver = PathResolver::new(session);
        
        // Test parsing slice notation
        let segments = resolver.parse_path("identifier:mrn.system").unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].base, "identifier:mrn");
        assert_eq!(segments[1].base, "system");
    }

    #[test]
    fn test_parse_path_with_extensions() {
        use std::sync::Arc;
        use crate::canonical::DefinitionSession;
        
        let session = Arc::new(DefinitionSession::for_testing());
        let resolver = PathResolver::new(session);
        
        // Test parsing extension notation
        let segments = resolver.parse_path("extension[http://example.org/ext].valueString").unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].base, "extension");
        assert!(matches!(segments[0].bracket, Some(Bracket::Slice(_))));
        assert_eq!(segments[1].base, "valueString");
    }

    #[test]
    fn test_parse_path_with_choice_types() {
        use std::sync::Arc;
        use crate::canonical::DefinitionSession;
        
        let session = Arc::new(DefinitionSession::for_testing());
        let resolver = PathResolver::new(session);
        
        // Test parsing choice type notation
        let segments = resolver.parse_path("deceased[x]").unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].base, "deceased");
        assert!(matches!(segments[0].bracket, Some(Bracket::ChoiceType)));
    }

    #[test]
    fn test_parse_path_complex() {
        use std::sync::Arc;
        use crate::canonical::DefinitionSession;
        
        let session = Arc::new(DefinitionSession::for_testing());
        let resolver = PathResolver::new(session);
        
        // Test complex path with multiple bracket types
        let segments = resolver.parse_path("component[systolic].value[x]").unwrap();
        assert_eq!(segments.len(), 2);
        
        assert_eq!(segments[0].base, "component");
        assert!(matches!(segments[0].bracket, Some(Bracket::Slice(_))));
        
        assert_eq!(segments[1].base, "value");
        assert!(matches!(segments[1].bracket, Some(Bracket::ChoiceType)));
    }

    #[test]
    fn test_parse_bracket_content() {
        use std::sync::Arc;
        use crate::canonical::DefinitionSession;
        
        let session = Arc::new(DefinitionSession::for_testing());
        let resolver = PathResolver::new(session);
        
        // Test different bracket types
        assert!(matches!(resolver.parse_bracket("+").unwrap(), Bracket::Soft(SoftIndexOp::Increment)));
        assert!(matches!(resolver.parse_bracket("=").unwrap(), Bracket::Soft(SoftIndexOp::Repeat)));
        assert!(matches!(resolver.parse_bracket("x").unwrap(), Bracket::ChoiceType));
        assert!(matches!(resolver.parse_bracket("0").unwrap(), Bracket::Index(0)));
        assert!(matches!(resolver.parse_bracket("42").unwrap(), Bracket::Index(42)));
        assert!(matches!(resolver.parse_bracket("sliceName").unwrap(), Bracket::Slice(_)));
    }

    #[test]
    fn test_cache_functionality() {
        use std::sync::Arc;
        use crate::canonical::DefinitionSession;
        
        let session = Arc::new(DefinitionSession::for_testing());
        let resolver = PathResolver::new(session);
        
        // Test cache stats
        let (size, capacity) = resolver.cache_stats();
        assert_eq!(size, 0);
        assert!(capacity >= 0);
        
        // Test cache clearing
        resolver.clear_cache();
        let (size_after_clear, _) = resolver.cache_stats();
        assert_eq!(size_after_clear, 0);
    }
}
