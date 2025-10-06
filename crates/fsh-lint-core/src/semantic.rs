//! Semantic analysis layer for FSH CST
//!
//! This module provides semantic analysis over the CST using the typed AST layer.
//! It extracts FHIR resources, builds symbol tables, and tracks references.

use crate::cst::{FshSyntaxNode, ast::*};
use crate::{Diagnostic, FshLintError, Location, Result};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

pub type Span = Range<usize>;

/// Semantic model representing the FHIR-aware structure of FSH content
#[derive(Debug, Clone)]
pub struct SemanticModel {
    /// CST root node
    pub cst: FshSyntaxNode,
    /// FHIR resources defined in the FSH content
    pub resources: Vec<FhirResource>,
    /// Symbol table for cross-reference tracking
    pub symbols: SymbolTable,
    /// References between resources and elements
    pub references: Vec<Reference>,
    /// Source file path
    pub source_file: PathBuf,
    /// Source map for efficient offset-to-line/column conversion
    pub source_map: crate::SourceMap,
    /// Original source text (needed for SourceMap lookups)
    pub source: String,
}

impl SemanticModel {
    /// Create a new empty semantic model (STUB)
    pub fn new(source_file: PathBuf) -> Self {
        let source = String::new();
        let source_map = crate::SourceMap::new(&source);
        // Create empty CST
        let (cst, _) = crate::cst::parse_fsh(&source);
        Self {
            cst,
            resources: Vec::new(),
            symbols: SymbolTable::default(),
            references: Vec::new(),
            source_file,
            source_map,
            source,
        }
    }

    /// Create a new semantic model from CST
    pub fn from_cst(cst: FshSyntaxNode, source: String, source_file: PathBuf) -> Self {
        let source_map = crate::SourceMap::new(&source);
        Self {
            cst,
            resources: Vec::new(),
            symbols: SymbolTable::default(),
            references: Vec::new(),
            source_file,
            source_map,
            source,
        }
    }

    /// Add a resource to the model
    pub fn add_resource(&mut self, resource: FhirResource) {
        let symbol = Symbol {
            name: resource.id.clone(),
            symbol_type: match resource.resource_type {
                ResourceType::Profile => SymbolType::Profile,
                ResourceType::Extension => SymbolType::Extension,
                ResourceType::ValueSet => SymbolType::ValueSet,
                ResourceType::CodeSystem => SymbolType::CodeSystem,
                ResourceType::Instance => SymbolType::Instance,
                ResourceType::Invariant => SymbolType::Invariant,
                ResourceType::RuleSet => SymbolType::RuleSet,
                ResourceType::Mapping => SymbolType::Mapping,
                ResourceType::Logical => SymbolType::Logical,
            },
            definition_location: resource.location.clone(),
            references: Vec::new(),
        };

        self.symbols.add_symbol(symbol);
        self.resources.push(resource);
    }

    /// Add a reference to the model
    pub fn add_reference(&mut self, reference: Reference) {
        self.references.push(reference);
    }

    /// Get resource by ID
    pub fn get_resource(&self, id: &str) -> Option<&FhirResource> {
        self.resources.iter().find(|r| r.id == id)
    }

    /// Get all resources of a specific type
    pub fn get_resources_by_type(&self, resource_type: ResourceType) -> Vec<&FhirResource> {
        self.resources
            .iter()
            .filter(|r| r.resource_type == resource_type)
            .collect()
    }

    /// Get symbol table
    pub fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }

    /// Get all unresolved references
    pub fn unresolved_references(&self) -> Vec<&Reference> {
        self.references.iter().filter(|r| !r.is_resolved).collect()
    }
}

/// Represents a FHIR resource defined in FSH
#[derive(Debug, Clone)]
pub struct FhirResource {
    pub resource_type: ResourceType,
    pub id: String,
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub parent: Option<String>,
    pub elements: Vec<Element>,
    pub location: Location,
    pub metadata: ResourceMetadata,
}

/// Types of FHIR resources that can be defined in FSH
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceType {
    Profile,
    Extension,
    ValueSet,
    CodeSystem,
    Instance,
    Invariant,
    RuleSet,
    Mapping,
    Logical,
}

/// Element within a FHIR resource
#[derive(Debug, Clone)]
pub struct Element {
    pub path: String,
    pub cardinality: Option<Cardinality>,
    pub type_info: Option<TypeInfo>,
    pub constraints: Vec<Constraint>,
    pub location: Location,
    pub flags: Vec<ElementFlag>,
}

/// Cardinality constraint for an element
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cardinality {
    pub min: u32,
    pub max: Option<u32>,
}

/// Type information for an element
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_name: String,
    pub profile: Option<String>,
    pub target_types: Vec<String>,
}

/// Constraint applied to an element
#[derive(Debug, Clone)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub value: String,
    pub location: Location,
}

/// Types of constraints that can be applied to elements
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintType {
    FixedValue,
    Pattern,
    Binding,
    Slice,
    Contains,
    Only,
    Obeys,
}

/// Element flags (MS, SU, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementFlag {
    MustSupport,
    Summary,
    Modifier,
    Draft,
    Normative,
    TrialUse,
}

/// Resource metadata
#[derive(Debug, Clone, Default)]
pub struct ResourceMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub status: Option<String>,
    pub experimental: Option<bool>,
    pub date: Option<String>,
    pub publisher: Option<String>,
    pub contact: Vec<String>,
    pub use_context: Vec<String>,
    pub jurisdiction: Vec<String>,
    pub purpose: Option<String>,
    pub copyright: Option<String>,
}

/// Symbol table for tracking definitions and references
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    symbols: HashMap<String, Symbol>,
    file_symbols: HashMap<PathBuf, Vec<String>>,
}

impl SymbolTable {
    pub fn add_symbol(&mut self, symbol: Symbol) {
        let file_path = symbol.definition_location.file.clone();
        let symbol_name = symbol.name.clone();

        self.symbols.insert(symbol_name.clone(), symbol);
        self.file_symbols
            .entry(file_path)
            .or_default()
            .push(symbol_name);
    }

    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn get_symbols_in_file(&self, file_path: &PathBuf) -> Vec<&Symbol> {
        if let Some(symbol_names) = self.file_symbols.get(file_path) {
            symbol_names
                .iter()
                .filter_map(|name| self.symbols.get(name))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn add_reference(&mut self, symbol_name: &str, location: Location) {
        if let Some(symbol) = self.symbols.get_mut(symbol_name) {
            symbol.references.push(location);
        }
    }

    pub fn contains_symbol(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    pub fn symbol_names(&self) -> Vec<&String> {
        self.symbols.keys().collect()
    }
}

/// Symbol definition
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub definition_location: Location,
    pub references: Vec<Location>,
}

/// Types of symbols
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolType {
    Profile,
    Extension,
    ValueSet,
    CodeSystem,
    Instance,
    Invariant,
    RuleSet,
    Mapping,
    Logical,
}

/// Reference between semantic entities
#[derive(Debug, Clone)]
pub struct Reference {
    pub from: Location,
    pub target: String,
    pub reference_type: ReferenceType,
    pub is_resolved: bool,
}

/// Reference types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceType {
    Parent,
    Type,
    Profile,
    ValueSet,
    Invariant,
}

/// Semantic analyzer trait for extracting semantic information from CST
pub trait SemanticAnalyzer {
    fn analyze(
        &self,
        cst: &FshSyntaxNode,
        source: &str,
        file_path: PathBuf,
    ) -> Result<SemanticModel>;
    fn resolve_references(&self, model: &mut SemanticModel) -> Result<()>;
    fn validate_semantics(&self, model: &SemanticModel) -> Vec<Diagnostic>;
}

/// Configuration for the semantic analyzer
#[derive(Debug, Clone)]
pub struct SemanticAnalyzerConfig {
    pub strict_validation: bool,
    pub resolve_cross_file_references: bool,
    pub max_element_depth: usize,
}

impl Default for SemanticAnalyzerConfig {
    fn default() -> Self {
        Self {
            strict_validation: false,
            resolve_cross_file_references: false,
            max_element_depth: 10,
        }
    }
}

/// Default implementation of semantic analyzer
#[derive(Debug, Default)]
pub struct DefaultSemanticAnalyzer {
    _config: SemanticAnalyzerConfig,
}

impl DefaultSemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: SemanticAnalyzerConfig) -> Self {
        Self { _config: config }
    }

    /// Async version of analyze
    ///
    /// Zero-overhead wrapper for async contexts.
    pub async fn analyze_async(
        &self,
        cst: &FshSyntaxNode,
        source: &str,
        file_path: PathBuf,
    ) -> Result<SemanticModel> {
        self.analyze(cst, source, file_path)
    }

    fn build_profile_resource(
        &self,
        profile: &Profile,
        source: &str,
        file_path: &Path,
    ) -> FhirResource {
        let name = profile.name().unwrap_or_else(|| "Unknown".to_string());
        let id = profile
            .id()
            .and_then(|c| c.value())
            .unwrap_or_else(|| name.clone());
        let title = profile.title().and_then(|c| c.value());
        let description = profile.description().and_then(|c| c.value());
        let parent = profile.parent().and_then(|c| c.value());

        let location = node_to_location(file_path, profile.syntax(), source);

        FhirResource {
            resource_type: ResourceType::Profile,
            id,
            name: Some(name),
            title,
            description,
            parent,
            elements: Vec::new(), // TODO: Extract elements from rules
            location,
            metadata: ResourceMetadata::default(),
        }
    }

    fn build_extension_resource(
        &self,
        extension: &Extension,
        source: &str,
        file_path: &Path,
    ) -> FhirResource {
        let name = extension.name().unwrap_or_else(|| "Unknown".to_string());
        let id = extension
            .id()
            .and_then(|c| c.value())
            .unwrap_or_else(|| name.clone());
        let title = extension.title().and_then(|c| c.value());
        let description = extension.description().and_then(|c| c.value());
        let parent = extension.parent().and_then(|c| c.value());

        let location = node_to_location(file_path, extension.syntax(), source);

        FhirResource {
            resource_type: ResourceType::Extension,
            id,
            name: Some(name),
            title,
            description,
            parent,
            elements: Vec::new(),
            location,
            metadata: ResourceMetadata::default(),
        }
    }

    fn build_value_set_resource(
        &self,
        value_set: &ValueSet,
        source: &str,
        file_path: &Path,
    ) -> FhirResource {
        let name = value_set.name().unwrap_or_else(|| "Unknown".to_string());
        let id = value_set
            .id()
            .and_then(|c| c.value())
            .unwrap_or_else(|| name.clone());
        let title = value_set.title().and_then(|c| c.value());
        let description = value_set.description().and_then(|c| c.value());

        let location = node_to_location(file_path, value_set.syntax(), source);

        FhirResource {
            resource_type: ResourceType::ValueSet,
            id,
            name: Some(name),
            title,
            description,
            parent: None,
            elements: Vec::new(),
            location,
            metadata: ResourceMetadata::default(),
        }
    }

    fn build_code_system_resource(
        &self,
        code_system: &CodeSystem,
        source: &str,
        file_path: &Path,
    ) -> FhirResource {
        let name = code_system.name().unwrap_or_else(|| "Unknown".to_string());
        let id = code_system
            .id()
            .and_then(|c| c.value())
            .unwrap_or_else(|| name.clone());
        let title = code_system.title().and_then(|c| c.value());
        let description = code_system.description().and_then(|c| c.value());

        let location = node_to_location(file_path, code_system.syntax(), source);

        FhirResource {
            resource_type: ResourceType::CodeSystem,
            id,
            name: Some(name),
            title,
            description,
            parent: None,
            elements: Vec::new(),
            location,
            metadata: ResourceMetadata::default(),
        }
    }

    /// Parse a cardinality string like "1..1" or "0..*"
    pub fn parse_cardinality(&self, text: &str) -> Result<Option<Cardinality>> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let parts: Vec<_> = trimmed.split("..").collect();
        if parts.len() != 2 {
            return Err(FshLintError::semantic_error(format!(
                "Invalid cardinality expression: {text}"
            )));
        }

        let min = parts[0].parse::<u32>().map_err(|_| {
            FshLintError::semantic_error(format!("Invalid minimum cardinality in {text}"))
        })?;

        let max = if parts[1] == "*" {
            None
        } else {
            Some(parts[1].parse::<u32>().map_err(|_| {
                FshLintError::semantic_error(format!("Invalid maximum cardinality in {text}"))
            })?)
        };

        Ok(Some(Cardinality { min, max }))
    }

    /// Extract reference targets from a type expression like `Reference(Patient | Practitioner)`
    pub fn extract_reference_targets(&self, type_name: &str) -> Result<Vec<String>> {
        let trimmed = type_name.trim();
        if !trimmed.starts_with("Reference(") || !trimmed.ends_with(')') {
            return Ok(Vec::new());
        }

        let inner = &trimmed[10..trimmed.len() - 1];
        let targets = inner
            .split('|')
            .map(|part| part.trim().trim_matches(['"', '\'']).to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(targets)
    }

    pub fn is_valid_resource_id(&self, id: &str) -> bool {
        !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }

    pub fn is_valid_element_path(&self, path: &str) -> bool {
        !path.is_empty() && !path.contains(' ')
    }

    pub fn is_valid_fhir_type(&self, type_name: &str) -> bool {
        !type_name.trim().is_empty()
    }

    fn extract_references(&self, model: &mut SemanticModel) -> Result<()> {
        let mut references = Vec::new();

        for resource in &model.resources {
            if let Some(parent) = &resource.parent {
                references.push(Reference {
                    from: resource.location.clone(),
                    target: parent.clone(),
                    reference_type: ReferenceType::Parent,
                    is_resolved: false,
                });
            }

            for element in &resource.elements {
                if let Some(type_info) = &element.type_info {
                    references.push(Reference {
                        from: element.location.clone(),
                        target: type_info.type_name.clone(),
                        reference_type: ReferenceType::Type,
                        is_resolved: false,
                    });

                    if let Some(profile) = &type_info.profile {
                        references.push(Reference {
                            from: element.location.clone(),
                            target: profile.clone(),
                            reference_type: ReferenceType::Profile,
                            is_resolved: false,
                        });
                    }

                    for target_type in &type_info.target_types {
                        references.push(Reference {
                            from: element.location.clone(),
                            target: target_type.clone(),
                            reference_type: ReferenceType::Type,
                            is_resolved: false,
                        });
                    }
                }

                for constraint in &element.constraints {
                    match constraint.constraint_type {
                        ConstraintType::Binding => references.push(Reference {
                            from: constraint.location.clone(),
                            target: constraint.value.clone(),
                            reference_type: ReferenceType::ValueSet,
                            is_resolved: false,
                        }),
                        ConstraintType::Obeys => references.push(Reference {
                            from: constraint.location.clone(),
                            target: constraint.value.clone(),
                            reference_type: ReferenceType::Invariant,
                            is_resolved: false,
                        }),
                        _ => {}
                    }
                }
            }
        }

        for reference in references {
            model.add_reference(reference);
        }

        Ok(())
    }
}

impl SemanticAnalyzer for DefaultSemanticAnalyzer {
    fn analyze(
        &self,
        cst: &FshSyntaxNode,
        source: &str,
        file_path: PathBuf,
    ) -> Result<SemanticModel> {
        let mut model = SemanticModel::from_cst(cst.clone(), source.to_string(), file_path.clone());

        // Cast CST root to typed Document
        let document = Document::cast(cst.clone()).ok_or_else(|| {
            FshLintError::semantic_error("Root node is not a Document".to_string())
        })?;

        // Extract Profiles
        for profile in document.profiles() {
            let resource = self.build_profile_resource(&profile, source, &file_path);
            model.add_resource(resource);
        }

        // Extract Extensions
        for extension in document.extensions() {
            let resource = self.build_extension_resource(&extension, source, &file_path);
            model.add_resource(resource);
        }

        // Extract ValueSets
        for value_set in document.value_sets() {
            let resource = self.build_value_set_resource(&value_set, source, &file_path);
            model.add_resource(resource);
        }

        // Extract CodeSystems
        for code_system in document.code_systems() {
            let resource = self.build_code_system_resource(&code_system, source, &file_path);
            model.add_resource(resource);
        }

        // Extract references between resources
        self.extract_references(&mut model)?;

        Ok(model)
    }

    fn resolve_references(&self, model: &mut SemanticModel) -> Result<()> {
        for reference in &mut model.references {
            if model.symbols.contains_symbol(&reference.target) {
                reference.is_resolved = true;
            }
        }
        Ok(())
    }

    fn validate_semantics(&self, model: &SemanticModel) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for reference in model.unresolved_references() {
            diagnostics.push(Diagnostic::new(
                "unresolved-reference",
                crate::Severity::Error,
                format!("Unresolved reference to '{}'", reference.target),
                reference.from.clone(),
            ));
        }

        let mut resource_ids = HashMap::new();
        for resource in &model.resources {
            if let Some(existing_location) = resource_ids.insert(&resource.id, &resource.location) {
                diagnostics.push(Diagnostic::new(
                    "duplicate-resource-id",
                    crate::Severity::Error,
                    format!(
                        "Duplicate resource ID '{}' (first defined at {})",
                        resource.id, existing_location
                    ),
                    resource.location.clone(),
                ));
            }

            diagnostics.extend(self.validate_resource(resource));

            for element in &resource.elements {
                diagnostics.extend(self.validate_element(element, resource));
            }
        }

        diagnostics
    }
}

impl DefaultSemanticAnalyzer {
    fn validate_resource(&self, resource: &FhirResource) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !self.is_valid_resource_id(&resource.id) {
            diagnostics.push(Diagnostic::new(
                "invalid-resource-id",
                crate::Severity::Error,
                format!("Invalid resource ID '{}'.", resource.id),
                resource.location.clone(),
            ));
        }

        diagnostics
    }

    fn validate_element(&self, element: &Element, resource: &FhirResource) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !self.is_valid_element_path(&element.path) {
            diagnostics.push(Diagnostic::new(
                "invalid-element-path",
                crate::Severity::Error,
                format!(
                    "Invalid element path '{}' in resource '{}'.",
                    element.path, resource.id
                ),
                element.location.clone(),
            ));
        }

        if let Some(type_info) = &element.type_info {
            if !self.is_valid_fhir_type(&type_info.type_name) {
                diagnostics.push(Diagnostic::new(
                    "invalid-fhir-type",
                    crate::Severity::Error,
                    format!(
                        "Invalid FHIR type '{}' in resource '{}'.",
                        type_info.type_name, resource.id
                    ),
                    element.location.clone(),
                ));
            }
        }

        diagnostics
    }
}

fn node_to_location(file_path: &Path, node: &FshSyntaxNode, source: &str) -> Location {
    let range = node.text_range();
    let span = usize::from(range.start())..usize::from(range.end());
    span_to_location(file_path, &span, source)
}

fn span_to_location(file_path: &Path, span: &Span, source: &str) -> Location {
    let (start_line, start_col) = offset_to_line_col(source, span.start);
    let (end_line, end_col) = offset_to_line_col(source, span.end);

    Location {
        file: file_path.to_path_buf(),
        line: start_line + 1,
        column: start_col + 1,
        end_line: Some(end_line + 1),
        end_column: Some(end_col + 1),
        offset: span.start,
        length: span.end.saturating_sub(span.start),
        span: Some((span.start, span.end)),
    }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0usize;
    let mut column = 0usize;
    let mut current = 0usize;

    for ch in source.chars() {
        if current >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }

        current += ch.len_utf8();
    }

    (line, column)
}
