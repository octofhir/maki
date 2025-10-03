//! Semantic analysis and FHIR-aware modeling

use crate::{Diagnostic, FshLintError, Location, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tree_sitter::{Node, Tree};

/// Semantic model representing the FHIR-aware structure of FSH content
#[derive(Debug, Clone)]
pub struct SemanticModel {
    /// FHIR resources defined in the FSH content
    pub resources: Vec<FhirResource>,
    /// Symbol table for cross-reference tracking
    pub symbols: SymbolTable,
    /// References between resources and elements
    pub references: Vec<Reference>,
    /// Source file path
    pub source_file: PathBuf,
}

/// Represents a FHIR resource defined in FSH
#[derive(Debug, Clone)]
pub struct FhirResource {
    /// Type of FHIR resource (Profile, Extension, ValueSet, etc.)
    pub resource_type: ResourceType,
    /// Unique identifier for the resource
    pub id: String,
    /// Human-readable name
    pub name: Option<String>,
    /// Title of the resource
    pub title: Option<String>,
    /// Description of the resource
    pub description: Option<String>,
    /// Parent resource (for profiles)
    pub parent: Option<String>,
    /// Elements defined in this resource
    pub elements: Vec<Element>,
    /// Location in source file
    pub location: Location,
    /// Metadata fields
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
    /// Element path (e.g., "Patient.name", "name.given")
    pub path: String,
    /// Cardinality constraints
    pub cardinality: Option<Cardinality>,
    /// Type information
    pub type_info: Option<TypeInfo>,
    /// Constraints applied to this element
    pub constraints: Vec<Constraint>,
    /// Location in source file
    pub location: Location,
    /// Flags (MS, SU, etc.)
    pub flags: Vec<ElementFlag>,
}

/// Cardinality constraint for an element
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cardinality {
    /// Minimum occurrences
    pub min: u32,
    /// Maximum occurrences (None for unbounded)
    pub max: Option<u32>,
}

/// Type information for an element
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Type name (e.g., "string", "Patient", "Reference(Patient)")
    pub type_name: String,
    /// Profile URL if specified
    pub profile: Option<String>,
    /// Target types for references
    pub target_types: Vec<String>,
}

/// Constraint applied to an element
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Type of constraint
    pub constraint_type: ConstraintType,
    /// Value of the constraint
    pub value: String,
    /// Location in source file
    pub location: Location,
}

/// Types of constraints that can be applied to elements
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintType {
    /// Fixed value constraint
    FixedValue,
    /// Pattern constraint
    Pattern,
    /// Binding constraint
    Binding,
    /// Slice constraint
    Slice,
    /// Contains constraint
    Contains,
    /// Only constraint (type restriction)
    Only,
    /// Obeys constraint (invariant)
    Obeys,
}

/// Element flags (MS, SU, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementFlag {
    /// Must Support
    MustSupport,
    /// Summary
    Summary,
    /// Modifier
    Modifier,
    /// Draft
    Draft,
    /// Normative
    Normative,
    /// Trial Use
    TrialUse,
}

/// Resource metadata
#[derive(Debug, Clone, Default)]
pub struct ResourceMetadata {
    /// Title
    pub title: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Version
    pub version: Option<String>,
    /// Status (draft, active, retired, etc.)
    pub status: Option<String>,
    /// Experimental flag
    pub experimental: Option<bool>,
    /// Date
    pub date: Option<String>,
    /// Publisher
    pub publisher: Option<String>,
    /// Contact information
    pub contact: Vec<String>,
    /// Use context
    pub use_context: Vec<String>,
    /// Jurisdiction
    pub jurisdiction: Vec<String>,
    /// Purpose
    pub purpose: Option<String>,
    /// Copyright
    pub copyright: Option<String>,
}

/// Symbol table for tracking definitions and references
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    /// Map of symbol names to their definitions
    symbols: HashMap<String, Symbol>,
    /// Map of file paths to symbols defined in that file
    file_symbols: HashMap<PathBuf, Vec<String>>,
}

/// Symbol definition
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Symbol type
    pub symbol_type: SymbolType,
    /// Location where symbol is defined
    pub definition_location: Location,
    /// References to this symbol
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
    Element,
}

/// Reference between resources or elements
#[derive(Debug, Clone)]
pub struct Reference {
    /// Source location of the reference
    pub from: Location,
    /// Target symbol name
    pub target: String,
    /// Type of reference
    pub reference_type: ReferenceType,
    /// Whether the reference is resolved
    pub is_resolved: bool,
}

/// Types of references
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceType {
    /// Parent reference (Profile extends Patient)
    Parent,
    /// Type reference (element type)
    Type,
    /// Profile reference (element profile)
    Profile,
    /// ValueSet binding
    ValueSet,
    /// CodeSystem reference
    CodeSystem,
    /// Instance reference
    Instance,
    /// Invariant reference
    Invariant,
    /// RuleSet reference
    RuleSet,
}

/// Semantic analyzer trait for extracting semantic information from AST
pub trait SemanticAnalyzer {
    /// Analyze a parsed FSH file and extract semantic model
    fn analyze(&self, tree: &Tree, source: &str, file_path: PathBuf) -> Result<SemanticModel>;

    /// Resolve references in the semantic model
    fn resolve_references(&self, model: &mut SemanticModel) -> Result<()>;

    /// Validate semantic constraints
    fn validate_semantics(&self, model: &SemanticModel) -> Vec<Diagnostic>;
}

/// Default implementation of semantic analyzer
#[derive(Debug, Default)]
pub struct DefaultSemanticAnalyzer {
    /// Configuration for semantic analysis
    config: SemanticAnalyzerConfig,
}

/// Configuration for semantic analyzer
#[derive(Debug, Clone)]
pub struct SemanticAnalyzerConfig {
    /// Enable strict validation
    pub strict_validation: bool,
    /// Enable cross-file reference resolution
    pub resolve_cross_file_references: bool,
    /// Maximum depth for element path resolution
    pub max_element_depth: usize,
}

impl Default for SemanticAnalyzerConfig {
    fn default() -> Self {
        Self {
            strict_validation: true,
            resolve_cross_file_references: true,
            max_element_depth: 10,
        }
    }
}

impl SemanticModel {
    /// Create a new empty semantic model
    pub fn new(source_file: PathBuf) -> Self {
        Self {
            resources: Vec::new(),
            symbols: SymbolTable::default(),
            references: Vec::new(),
            source_file,
        }
    }

    /// Add a resource to the model
    pub fn add_resource(&mut self, resource: FhirResource) {
        // Add to symbol table
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

impl SymbolTable {
    /// Add a symbol to the table
    pub fn add_symbol(&mut self, symbol: Symbol) {
        let file_path = symbol.definition_location.file.clone();
        let symbol_name = symbol.name.clone();

        // Add to main symbol map
        self.symbols.insert(symbol_name.clone(), symbol);

        // Add to file symbols map
        self.file_symbols
            .entry(file_path)
            .or_insert_with(Vec::new)
            .push(symbol_name);
    }

    /// Get symbol by name
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Get all symbols defined in a file
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

    /// Add a reference to a symbol
    pub fn add_reference(&mut self, symbol_name: &str, location: Location) {
        if let Some(symbol) = self.symbols.get_mut(symbol_name) {
            symbol.references.push(location);
        }
    }

    /// Check if symbol exists
    pub fn contains_symbol(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// Get all symbol names
    pub fn symbol_names(&self) -> Vec<&String> {
        self.symbols.keys().collect()
    }
}

impl DefaultSemanticAnalyzer {
    /// Create a new semantic analyzer with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new semantic analyzer with custom configuration
    pub fn with_config(config: SemanticAnalyzerConfig) -> Self {
        Self { config }
    }

    /// Extract resources from the AST
    fn extract_resources(
        &self,
        root_node: Node,
        source: &str,
        file_path: &PathBuf,
    ) -> Result<Vec<FhirResource>> {
        let mut resources = Vec::new();
        let mut cursor = root_node.walk();

        // Walk through all top-level nodes
        if cursor.goto_first_child() {
            loop {
                let node = cursor.node();

                // Check if this is a resource definition
                if let Some(resource) = self.extract_resource_from_node(node, source, file_path)? {
                    resources.push(resource);
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(resources)
    }

    /// Extract a single resource from a node
    fn extract_resource_from_node(
        &self,
        node: Node,
        source: &str,
        file_path: &PathBuf,
    ) -> Result<Option<FhirResource>> {
        let node_type = node.kind();

        let resource_type = match node_type {
            "profile_definition" => ResourceType::Profile,
            "extension_definition" => ResourceType::Extension,
            "valueset_definition" => ResourceType::ValueSet,
            "codesystem_definition" => ResourceType::CodeSystem,
            "instance_definition" => ResourceType::Instance,
            "invariant_definition" => ResourceType::Invariant,
            "ruleset_definition" => ResourceType::RuleSet,
            "mapping_definition" => ResourceType::Mapping,
            "logical_definition" => ResourceType::Logical,
            _ => return Ok(None), // Not a resource definition
        };

        // Extract resource ID (first identifier after keyword)
        let id = self.extract_resource_id(node, source)?;

        // Create location from node
        let location = self.node_to_location(node, file_path);

        // Extract elements
        let elements = self.extract_elements(node, source, file_path)?;

        // Extract metadata
        let metadata = self.extract_metadata(node, source)?;

        // Extract parent (for profiles)
        let parent = if resource_type == ResourceType::Profile {
            self.extract_parent(node, source)?
        } else {
            None
        };

        let resource = FhirResource {
            resource_type,
            id,
            name: None, // Will be extracted from metadata if present
            title: metadata.title.clone(),
            description: metadata.description.clone(),
            parent,
            elements,
            location,
            metadata,
        };

        Ok(Some(resource))
    }

    /// Extract resource ID from node
    fn extract_resource_id(&self, node: Node, source: &str) -> Result<String> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "identifier" {
                    return Ok(child
                        .utf8_text(source.as_bytes())
                        .map_err(|e| {
                            FshLintError::semantic_error(format!(
                                "Failed to extract resource ID: {}",
                                e
                            ))
                        })?
                        .to_string());
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Err(FshLintError::semantic_error(
            "Resource ID not found".to_string(),
        ))
    }

    /// Extract parent resource for profiles
    fn extract_parent(&self, node: Node, source: &str) -> Result<Option<String>> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "parent_statement" {
                    // Look for identifier in parent statement
                    let mut parent_cursor = child.walk();
                    if parent_cursor.goto_first_child() {
                        loop {
                            let parent_child = parent_cursor.node();
                            if parent_child.kind() == "identifier" {
                                return Ok(Some(
                                    parent_child
                                        .utf8_text(source.as_bytes())
                                        .map_err(|e| {
                                            FshLintError::semantic_error(format!(
                                                "Failed to extract parent: {}",
                                                e
                                            ))
                                        })?
                                        .to_string(),
                                ));
                            }

                            if !parent_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Extract elements from resource node
    fn extract_elements(
        &self,
        node: Node,
        source: &str,
        file_path: &PathBuf,
    ) -> Result<Vec<Element>> {
        let mut elements = Vec::new();
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();

                // Look for element rules (caret rules, assignment rules, etc.)
                if let Some(element) = self.extract_element_from_node(child, source, file_path)? {
                    elements.push(element);
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(elements)
    }

    /// Extract element from a rule node
    fn extract_element_from_node(
        &self,
        node: Node,
        source: &str,
        file_path: &PathBuf,
    ) -> Result<Option<Element>> {
        let node_type = node.kind();

        match node_type {
            "caret_rule" | "assignment_rule" | "binding_rule" => {
                // Extract element path
                if let Some(path) = self.extract_element_path(node, source)? {
                    let location = self.node_to_location(node, file_path);

                    // Extract cardinality if present
                    let cardinality = self.extract_cardinality(node, source)?;

                    // Extract type info if present
                    let type_info = self.extract_type_info(node, source)?;

                    // Extract constraints
                    let constraints = self.extract_constraints(node, source, file_path)?;

                    // Extract flags
                    let flags = self.extract_element_flags(node, source)?;

                    let element = Element {
                        path,
                        cardinality,
                        type_info,
                        constraints,
                        location,
                        flags,
                    };

                    return Ok(Some(element));
                }
            }
            _ => {}
        }

        Ok(None)
    }

    /// Extract element path from rule node
    fn extract_element_path(&self, node: Node, source: &str) -> Result<Option<String>> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "path" || child.kind() == "element_path" {
                    return Ok(Some(
                        child
                            .utf8_text(source.as_bytes())
                            .map_err(|e| {
                                FshLintError::semantic_error(format!(
                                    "Failed to extract element path: {}",
                                    e
                                ))
                            })?
                            .to_string(),
                    ));
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Extract cardinality from node
    fn extract_cardinality(&self, node: Node, source: &str) -> Result<Option<Cardinality>> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "cardinality" {
                    // Parse cardinality (e.g., "1..1", "0..*", "1..5")
                    let cardinality_text = child.utf8_text(source.as_bytes()).map_err(|e| {
                        FshLintError::semantic_error(format!(
                            "Failed to extract cardinality: {}",
                            e
                        ))
                    })?;

                    return Ok(self.parse_cardinality(cardinality_text)?);
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Parse cardinality string (e.g., "1..1", "0..*")
    pub fn parse_cardinality(&self, cardinality_str: &str) -> Result<Option<Cardinality>> {
        if let Some((min_str, max_str)) = cardinality_str.split_once("..") {
            let min = min_str.parse::<u32>().map_err(|e| {
                FshLintError::semantic_error(format!("Invalid cardinality min: {}", e))
            })?;

            let max = if max_str == "*" {
                None
            } else {
                Some(max_str.parse::<u32>().map_err(|e| {
                    FshLintError::semantic_error(format!("Invalid cardinality max: {}", e))
                })?)
            };

            Ok(Some(Cardinality { min, max }))
        } else {
            Ok(None)
        }
    }

    /// Extract type information from node
    fn extract_type_info(&self, node: Node, source: &str) -> Result<Option<TypeInfo>> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();

                // Look for type assignment (e.g., "* name only string")
                if child.kind() == "type_assignment" || child.kind() == "only_rule" {
                    if let Some(type_name) = self.extract_type_name(child, source)? {
                        let type_info = TypeInfo {
                            type_name: type_name.clone(),
                            profile: None, // TODO: Extract profile if specified
                            target_types: if type_name.starts_with("Reference(") {
                                self.extract_reference_targets(&type_name)?
                            } else {
                                Vec::new()
                            },
                        };
                        return Ok(Some(type_info));
                    }
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Extract type name from type assignment node
    fn extract_type_name(&self, node: Node, source: &str) -> Result<Option<String>> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "type" || child.kind() == "identifier" {
                    return Ok(Some(
                        child
                            .utf8_text(source.as_bytes())
                            .map_err(|e| {
                                FshLintError::semantic_error(format!(
                                    "Failed to extract type name: {}",
                                    e
                                ))
                            })?
                            .to_string(),
                    ));
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Extract reference target types from Reference(Type1 | Type2) syntax
    pub fn extract_reference_targets(&self, type_name: &str) -> Result<Vec<String>> {
        if let Some(targets_str) = type_name
            .strip_prefix("Reference(")
            .and_then(|s| s.strip_suffix(")"))
        {
            let targets: Vec<String> = targets_str
                .split('|')
                .map(|s| s.trim().to_string())
                .collect();
            Ok(targets)
        } else {
            Ok(Vec::new())
        }
    }

    /// Extract constraints from node
    fn extract_constraints(
        &self,
        node: Node,
        source: &str,
        file_path: &PathBuf,
    ) -> Result<Vec<Constraint>> {
        let mut constraints = Vec::new();
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();

                match child.kind() {
                    "fixed_value_rule" => {
                        if let Some(value) = self.extract_constraint_value(child, source)? {
                            constraints.push(Constraint {
                                constraint_type: ConstraintType::FixedValue,
                                value,
                                location: self.node_to_location(child, file_path),
                            });
                        }
                    }
                    "pattern_rule" => {
                        if let Some(value) = self.extract_constraint_value(child, source)? {
                            constraints.push(Constraint {
                                constraint_type: ConstraintType::Pattern,
                                value,
                                location: self.node_to_location(child, file_path),
                            });
                        }
                    }
                    "binding_rule" => {
                        if let Some(value) = self.extract_constraint_value(child, source)? {
                            constraints.push(Constraint {
                                constraint_type: ConstraintType::Binding,
                                value,
                                location: self.node_to_location(child, file_path),
                            });
                        }
                    }
                    "contains_rule" => {
                        if let Some(value) = self.extract_constraint_value(child, source)? {
                            constraints.push(Constraint {
                                constraint_type: ConstraintType::Contains,
                                value,
                                location: self.node_to_location(child, file_path),
                            });
                        }
                    }
                    "obeys_rule" => {
                        if let Some(value) = self.extract_constraint_value(child, source)? {
                            constraints.push(Constraint {
                                constraint_type: ConstraintType::Obeys,
                                value,
                                location: self.node_to_location(child, file_path),
                            });
                        }
                    }
                    _ => {}
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(constraints)
    }

    /// Extract constraint value from constraint node
    fn extract_constraint_value(&self, node: Node, source: &str) -> Result<Option<String>> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();

                // Look for value nodes
                if child.kind() == "value"
                    || child.kind() == "string"
                    || child.kind() == "identifier"
                {
                    return Ok(Some(
                        child
                            .utf8_text(source.as_bytes())
                            .map_err(|e| {
                                FshLintError::semantic_error(format!(
                                    "Failed to extract constraint value: {}",
                                    e
                                ))
                            })?
                            .to_string(),
                    ));
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Extract element flags from node
    fn extract_element_flags(&self, node: Node, source: &str) -> Result<Vec<ElementFlag>> {
        let mut flags = Vec::new();
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();

                if child.kind() == "flag" {
                    if let Ok(flag_text) = child.utf8_text(source.as_bytes()) {
                        match flag_text {
                            "MS" => flags.push(ElementFlag::MustSupport),
                            "SU" => flags.push(ElementFlag::Summary),
                            "?!" => flags.push(ElementFlag::Modifier),
                            "D" => flags.push(ElementFlag::Draft),
                            "N" => flags.push(ElementFlag::Normative),
                            "TU" => flags.push(ElementFlag::TrialUse),
                            _ => {} // Unknown flag, ignore
                        }
                    }
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(flags)
    }

    /// Extract metadata from resource node
    fn extract_metadata(&self, node: Node, source: &str) -> Result<ResourceMetadata> {
        let mut metadata = ResourceMetadata::default();
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();

                match child.kind() {
                    "title_statement" => {
                        metadata.title = self.extract_metadata_value(child, source)?;
                    }
                    "description_statement" => {
                        metadata.description = self.extract_metadata_value(child, source)?;
                    }
                    "version_statement" => {
                        metadata.version = self.extract_metadata_value(child, source)?;
                    }
                    "status_statement" => {
                        metadata.status = self.extract_metadata_value(child, source)?;
                    }
                    "experimental_statement" => {
                        if let Some(value) = self.extract_metadata_value(child, source)? {
                            metadata.experimental = Some(value.to_lowercase() == "true");
                        }
                    }
                    "date_statement" => {
                        metadata.date = self.extract_metadata_value(child, source)?;
                    }
                    "publisher_statement" => {
                        metadata.publisher = self.extract_metadata_value(child, source)?;
                    }
                    "purpose_statement" => {
                        metadata.purpose = self.extract_metadata_value(child, source)?;
                    }
                    "copyright_statement" => {
                        metadata.copyright = self.extract_metadata_value(child, source)?;
                    }
                    _ => {}
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(metadata)
    }

    /// Extract metadata value from metadata statement node
    fn extract_metadata_value(&self, node: Node, source: &str) -> Result<Option<String>> {
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();

                if child.kind() == "string" || child.kind() == "value" {
                    let value = child.utf8_text(source.as_bytes()).map_err(|e| {
                        FshLintError::semantic_error(format!(
                            "Failed to extract metadata value: {}",
                            e
                        ))
                    })?;

                    // Remove quotes if present
                    let cleaned_value = if value.starts_with('"') && value.ends_with('"') {
                        &value[1..value.len() - 1]
                    } else {
                        value
                    };

                    return Ok(Some(cleaned_value.to_string()));
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Extract references from the semantic model
    fn extract_references(&self, model: &mut SemanticModel) -> Result<()> {
        let mut references = Vec::new();

        for resource in &model.resources {
            // Add parent references for profiles
            if let Some(parent) = &resource.parent {
                let reference = Reference {
                    from: resource.location.clone(),
                    target: parent.clone(),
                    reference_type: ReferenceType::Parent,
                    is_resolved: false,
                };
                references.push(reference);
            }

            // Add type references from elements
            for element in &resource.elements {
                if let Some(type_info) = &element.type_info {
                    // Add type reference
                    let reference = Reference {
                        from: element.location.clone(),
                        target: type_info.type_name.clone(),
                        reference_type: ReferenceType::Type,
                        is_resolved: false,
                    };
                    references.push(reference);

                    // Add profile reference if present
                    if let Some(profile) = &type_info.profile {
                        let reference = Reference {
                            from: element.location.clone(),
                            target: profile.clone(),
                            reference_type: ReferenceType::Profile,
                            is_resolved: false,
                        };
                        references.push(reference);
                    }

                    // Add target type references for Reference types
                    for target_type in &type_info.target_types {
                        let reference = Reference {
                            from: element.location.clone(),
                            target: target_type.clone(),
                            reference_type: ReferenceType::Type,
                            is_resolved: false,
                        };
                        references.push(reference);
                    }
                }

                // Add constraint references
                for constraint in &element.constraints {
                    match constraint.constraint_type {
                        ConstraintType::Binding => {
                            let reference = Reference {
                                from: constraint.location.clone(),
                                target: constraint.value.clone(),
                                reference_type: ReferenceType::ValueSet,
                                is_resolved: false,
                            };
                            references.push(reference);
                        }
                        ConstraintType::Obeys => {
                            let reference = Reference {
                                from: constraint.location.clone(),
                                target: constraint.value.clone(),
                                reference_type: ReferenceType::Invariant,
                                is_resolved: false,
                            };
                            references.push(reference);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Add all references to the model
        for reference in references {
            model.add_reference(reference);
        }

        Ok(())
    }

    /// Convert tree-sitter node to location
    fn node_to_location(&self, node: Node, file_path: &PathBuf) -> Location {
        let start_point = node.start_position();
        let end_point = node.end_position();

        Location {
            file: file_path.clone(),
            line: start_point.row + 1,      // Convert to 1-based
            column: start_point.column + 1, // Convert to 1-based
            end_line: Some(end_point.row + 1),
            end_column: Some(end_point.column + 1),
            offset: node.start_byte(),
            length: node.end_byte() - node.start_byte(),
            span: Some((node.start_byte(), node.end_byte())),
        }
    }
}

impl SemanticAnalyzer for DefaultSemanticAnalyzer {
    fn analyze(&self, tree: &Tree, source: &str, file_path: PathBuf) -> Result<SemanticModel> {
        let mut model = SemanticModel::new(file_path.clone());

        // Extract resources from the AST
        let resources = self.extract_resources(tree.root_node(), source, &file_path)?;

        // Add resources to the model
        for resource in resources {
            model.add_resource(resource);
        }

        // Extract references between resources and elements
        self.extract_references(&mut model)?;

        Ok(model)
    }

    fn resolve_references(&self, model: &mut SemanticModel) -> Result<()> {
        // Mark references as resolved if target exists in symbol table
        for reference in &mut model.references {
            if model.symbols.contains_symbol(&reference.target) {
                reference.is_resolved = true;
            }
        }

        Ok(())
    }

    fn validate_semantics(&self, model: &SemanticModel) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for unresolved references
        for reference in model.unresolved_references() {
            let diagnostic = Diagnostic::new(
                "unresolved-reference",
                crate::Severity::Error,
                format!("Unresolved reference to '{}'", reference.target),
                reference.from.clone(),
            );
            diagnostics.push(diagnostic);
        }

        // Validate resource-specific rules
        for resource in &model.resources {
            diagnostics.extend(self.validate_resource(resource));
        }

        // Check for duplicate resource IDs
        let mut resource_ids = HashMap::new();
        for resource in &model.resources {
            if let Some(existing_location) = resource_ids.insert(&resource.id, &resource.location) {
                let diagnostic = Diagnostic::new(
                    "duplicate-resource-id",
                    crate::Severity::Error,
                    format!(
                        "Duplicate resource ID '{}' (first defined at {})",
                        resource.id, existing_location
                    ),
                    resource.location.clone(),
                );
                diagnostics.push(diagnostic);
            }
        }

        // Validate element paths and cardinalities
        for resource in &model.resources {
            for element in &resource.elements {
                diagnostics.extend(self.validate_element(element, resource));
            }
        }

        diagnostics
    }
}

impl DefaultSemanticAnalyzer {
    /// Validate a single resource
    fn validate_resource(&self, resource: &FhirResource) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for missing required metadata
        match resource.resource_type {
            ResourceType::Profile | ResourceType::Extension => {
                if resource.title.is_none() && resource.metadata.title.is_none() {
                    let diagnostic = Diagnostic::new(
                        "missing-title",
                        crate::Severity::Warning,
                        format!(
                            "{} '{}' should have a title",
                            resource.resource_type, resource.id
                        ),
                        resource.location.clone(),
                    );
                    diagnostics.push(diagnostic);
                }

                if resource.description.is_none() && resource.metadata.description.is_none() {
                    let diagnostic = Diagnostic::new(
                        "missing-description",
                        crate::Severity::Warning,
                        format!(
                            "{} '{}' should have a description",
                            resource.resource_type, resource.id
                        ),
                        resource.location.clone(),
                    );
                    diagnostics.push(diagnostic);
                }
            }
            _ => {}
        }

        // Validate resource ID format
        if !self.is_valid_resource_id(&resource.id) {
            let diagnostic = Diagnostic::new(
                "invalid-resource-id",
                crate::Severity::Error,
                format!(
                    "Invalid resource ID '{}'. IDs must start with a letter and contain only letters, numbers, and hyphens",
                    resource.id
                ),
                resource.location.clone(),
            );
            diagnostics.push(diagnostic);
        }

        // Check for profiles without parent
        if resource.resource_type == ResourceType::Profile && resource.parent.is_none() {
            let diagnostic = Diagnostic::new(
                "profile-missing-parent",
                crate::Severity::Error,
                format!("Profile '{}' must specify a parent resource", resource.id),
                resource.location.clone(),
            );
            diagnostics.push(diagnostic);
        }

        diagnostics
    }

    /// Validate a single element
    fn validate_element(&self, element: &Element, resource: &FhirResource) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Validate cardinality
        if let Some(cardinality) = &element.cardinality {
            if let Some(max) = cardinality.max {
                if cardinality.min > max {
                    let diagnostic = Diagnostic::new(
                        "invalid-cardinality",
                        crate::Severity::Error,
                        format!(
                            "Invalid cardinality {}: minimum ({}) cannot be greater than maximum ({})",
                            cardinality, cardinality.min, max
                        ),
                        element.location.clone(),
                    );
                    diagnostics.push(diagnostic);
                }
            }
        }

        // Validate element path format
        if !self.is_valid_element_path(&element.path) {
            let diagnostic = Diagnostic::new(
                "invalid-element-path",
                crate::Severity::Error,
                format!(
                    "Invalid element path '{}'. Paths should use dot notation (e.g., 'name.given')",
                    element.path
                ),
                element.location.clone(),
            );
            diagnostics.push(diagnostic);
        }

        // Check for conflicting constraints
        let fixed_values: Vec<_> = element
            .constraints
            .iter()
            .filter(|c| c.constraint_type == ConstraintType::FixedValue)
            .collect();

        if fixed_values.len() > 1 {
            let diagnostic = Diagnostic::new(
                "multiple-fixed-values",
                crate::Severity::Error,
                format!(
                    "Element '{}' has multiple fixed values, only one is allowed",
                    element.path
                ),
                element.location.clone(),
            );
            diagnostics.push(diagnostic);
        }

        // Validate type constraints for specific resource types
        if resource.resource_type == ResourceType::Profile {
            if let Some(type_info) = &element.type_info {
                // Check for valid FHIR types
                if !self.is_valid_fhir_type(&type_info.type_name) {
                    let diagnostic = Diagnostic::new(
                        "invalid-fhir-type",
                        crate::Severity::Warning,
                        format!("'{}' may not be a valid FHIR type", type_info.type_name),
                        element.location.clone(),
                    );
                    diagnostics.push(diagnostic);
                }
            }
        }

        diagnostics
    }

    /// Check if resource ID is valid
    pub fn is_valid_resource_id(&self, id: &str) -> bool {
        if id.is_empty() {
            return false;
        }

        // Must start with a letter
        if !id.chars().next().unwrap().is_ascii_alphabetic() {
            return false;
        }

        // Can only contain letters, numbers, and hyphens
        id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    }

    /// Check if element path is valid
    pub fn is_valid_element_path(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        // Basic validation: should not start or end with dot, no double dots
        !path.starts_with('.') && !path.ends_with('.') && !path.contains("..")
    }

    /// Check if type name is a valid FHIR type (basic validation)
    pub fn is_valid_fhir_type(&self, type_name: &str) -> bool {
        // Basic FHIR primitive types
        const PRIMITIVE_TYPES: &[&str] = &[
            "boolean",
            "integer",
            "string",
            "decimal",
            "uri",
            "url",
            "canonical",
            "base64Binary",
            "instant",
            "date",
            "dateTime",
            "time",
            "code",
            "oid",
            "id",
            "markdown",
            "unsignedInt",
            "positiveInt",
            "uuid",
        ];

        // Common FHIR resource types
        const RESOURCE_TYPES: &[&str] = &[
            "Patient",
            "Practitioner",
            "Organization",
            "Location",
            "Encounter",
            "Observation",
            "Condition",
            "Procedure",
            "MedicationRequest",
            "DiagnosticReport",
            "DocumentReference",
            "Bundle",
            "OperationOutcome",
        ];

        // Check if it's a primitive type
        if PRIMITIVE_TYPES.contains(&type_name) {
            return true;
        }

        // Check if it's a resource type
        if RESOURCE_TYPES.contains(&type_name) {
            return true;
        }

        // Check if it's a Reference type
        if type_name.starts_with("Reference(") && type_name.ends_with(')') {
            return true;
        }

        // For now, assume other types might be valid (could be custom types or extensions)
        true
    }
}

// Helper implementations
impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::Profile => write!(f, "Profile"),
            ResourceType::Extension => write!(f, "Extension"),
            ResourceType::ValueSet => write!(f, "ValueSet"),
            ResourceType::CodeSystem => write!(f, "CodeSystem"),
            ResourceType::Instance => write!(f, "Instance"),
            ResourceType::Invariant => write!(f, "Invariant"),
            ResourceType::RuleSet => write!(f, "RuleSet"),
            ResourceType::Mapping => write!(f, "Mapping"),
            ResourceType::Logical => write!(f, "Logical"),
        }
    }
}

impl std::fmt::Display for Cardinality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.max {
            Some(max) => write!(f, "{}..{}", self.min, max),
            None => write!(f, "{}..*", self.min),
        }
    }
}
