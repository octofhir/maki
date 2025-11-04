//! Typed AST layer over CST
//!
//! This module provides ergonomic, type-safe wrappers over the raw CST nodes.
//! Each wrapper implements a `cast()` method to safely convert from CST nodes.
//!
//! # Example
//!
//! ```ignore
//! use maki_core::cst::{parse_fsh, ast::Profile};
//!
//! let (cst, _lexer_errors, _) = parse_fsh("Profile: MyPatient\nParent: Patient");
//! let profile = Profile::cast(cst.first_child().unwrap()).unwrap();
//!
//! assert_eq!(profile.name().unwrap(), "MyPatient");
//! assert_eq!(profile.parent().unwrap().value(), "Patient");
//! ```

use super::{FshSyntaxKind, FshSyntaxNode, FshSyntaxToken};

/// Flag values for FSH rules
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlagValue {
    /// Must Support (MS)
    MustSupport,
    /// Summary (SU)
    Summary,
    /// Trial Use (TU)
    TrialUse,
    /// Normative (N)
    Normative,
    /// Draft (D)
    Draft,
    /// Modifier (?!)
    Modifier,
}

impl FlagValue {
    /// Convert from syntax kind to flag value
    pub fn from_syntax_kind(kind: FshSyntaxKind) -> Option<Self> {
        match kind {
            FshSyntaxKind::MsFlag => Some(FlagValue::MustSupport),
            FshSyntaxKind::SuFlag => Some(FlagValue::Summary),
            FshSyntaxKind::TuFlag => Some(FlagValue::TrialUse),
            FshSyntaxKind::NFlag => Some(FlagValue::Normative),
            FshSyntaxKind::DFlag => Some(FlagValue::Draft),
            FshSyntaxKind::ModifierFlag => Some(FlagValue::Modifier),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagValue::MustSupport => "MS",
            FlagValue::Summary => "SU",
            FlagValue::TrialUse => "TU",
            FlagValue::Normative => "N",
            FlagValue::Draft => "D",
            FlagValue::Modifier => "?!",
        }
    }

    /// Check for flag conflicts (some flags are mutually exclusive)
    pub fn conflicts_with(&self, other: &FlagValue) -> bool {
        match (self, other) {
            // Normative, Trial Use, and Draft are mutually exclusive
            (FlagValue::Normative, FlagValue::TrialUse) => true,
            (FlagValue::Normative, FlagValue::Draft) => true,
            (FlagValue::TrialUse, FlagValue::Normative) => true,
            (FlagValue::TrialUse, FlagValue::Draft) => true,
            (FlagValue::Draft, FlagValue::Normative) => true,
            (FlagValue::Draft, FlagValue::TrialUse) => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for FlagValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Helper trait to join FlagValue vectors
pub trait FlagValueJoin {
    fn join(&self, separator: &str) -> String;
}

impl FlagValueJoin for Vec<FlagValue> {
    fn join(&self, separator: &str) -> String {
        self.iter()
            .map(|f| f.as_str())
            .collect::<Vec<_>>()
            .join(separator)
    }
}

/// Helper trait for casting CST nodes to typed wrappers
pub trait AstNode: Sized {
    fn can_cast(kind: FshSyntaxKind) -> bool;
    fn cast(node: FshSyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &FshSyntaxNode;
}

/// Helper function to find first child of a specific kind
fn child_of_kind(parent: &FshSyntaxNode, kind: FshSyntaxKind) -> Option<FshSyntaxNode> {
    parent.children().find(|n| n.kind() == kind)
}

/// Helper function to find first token of a specific kind
fn token_of_kind(parent: &FshSyntaxNode, kind: FshSyntaxKind) -> Option<FshSyntaxToken> {
    parent
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == kind)
}

/// Helper function to get identifier text (trim whitespace)
fn get_ident_text(node: &FshSyntaxNode) -> Option<String> {
    token_of_kind(node, FshSyntaxKind::Ident).map(|t| t.text().trim().to_string())
}

/// Helper function to get string literal text (without quotes)
fn get_string_text(node: &FshSyntaxNode) -> Option<String> {
    token_of_kind(node, FshSyntaxKind::String).map(|t| {
        let text = t.text();
        // Remove surrounding quotes
        if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
            text[1..text.len() - 1].to_string()
        } else {
            text.to_string()
        }
    })
}

// ============================================================================
// Document
// ============================================================================

/// Root document containing all FSH definitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    syntax: FshSyntaxNode,
}

impl AstNode for Document {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Document
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Document {
    pub fn profiles(&self) -> impl Iterator<Item = Profile> {
        self.syntax.children().filter_map(Profile::cast)
    }

    pub fn extensions(&self) -> impl Iterator<Item = Extension> {
        self.syntax.children().filter_map(Extension::cast)
    }

    pub fn value_sets(&self) -> impl Iterator<Item = ValueSet> {
        self.syntax.children().filter_map(ValueSet::cast)
    }

    pub fn code_systems(&self) -> impl Iterator<Item = CodeSystem> {
        self.syntax.children().filter_map(CodeSystem::cast)
    }

    pub fn logicals(&self) -> impl Iterator<Item = Logical> {
        self.syntax.children().filter_map(Logical::cast)
    }

    pub fn resources(&self) -> impl Iterator<Item = Resource> {
        self.syntax.children().filter_map(Resource::cast)
    }

    pub fn aliases(&self) -> impl Iterator<Item = Alias> {
        self.syntax.children().filter_map(Alias::cast)
    }

    pub fn instances(&self) -> impl Iterator<Item = Instance> {
        self.syntax.children().filter_map(Instance::cast)
    }

    pub fn mappings(&self) -> impl Iterator<Item = Mapping> {
        self.syntax.children().filter_map(Mapping::cast)
    }

    pub fn invariants(&self) -> impl Iterator<Item = Invariant> {
        self.syntax.children().filter_map(Invariant::cast)
    }
}

// ============================================================================
// Profile
// ============================================================================

/// Profile definition: Profile: Name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    syntax: FshSyntaxNode,
}

impl AstNode for Profile {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Profile
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Profile {
    /// Get the profile name
    pub fn name(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }

    /// Get the syntax node for the profile name (for precise location)
    pub fn name_token(&self) -> Option<FshSyntaxToken> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident)
    }

    /// Get the parent clause (Parent: ResourceType)
    pub fn parent(&self) -> Option<ParentClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::ParentClause).and_then(ParentClause::cast)
    }

    /// Get the id clause (Id: profile-id)
    pub fn id(&self) -> Option<IdClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::IdClause).and_then(IdClause::cast)
    }

    /// Get the title clause (Title: "Profile Title")
    pub fn title(&self) -> Option<TitleClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TitleClause).and_then(TitleClause::cast)
    }

    /// Get the description clause (Description: "Profile description")
    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    /// Get all rules in the profile
    pub fn rules(&self) -> impl Iterator<Item = Rule> {
        self.syntax.children().filter_map(Rule::cast)
    }

    pub fn components(&self) -> impl Iterator<Item = VsComponent> {
        self.syntax.children().filter_map(VsComponent::cast)
    }
}

// ============================================================================
// Extension
// ============================================================================

/// Extension definition: Extension: Name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    syntax: FshSyntaxNode,
}

impl AstNode for Extension {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Extension
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Extension {
    pub fn name(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }

    pub fn name_token(&self) -> Option<FshSyntaxToken> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident)
    }

    pub fn parent(&self) -> Option<ParentClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::ParentClause).and_then(ParentClause::cast)
    }

    pub fn id(&self) -> Option<IdClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::IdClause).and_then(IdClause::cast)
    }

    pub fn title(&self) -> Option<TitleClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TitleClause).and_then(TitleClause::cast)
    }

    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    pub fn components(&self) -> impl Iterator<Item = VsComponent> {
        self.syntax.children().filter_map(VsComponent::cast)
    }

    pub fn rules(&self) -> impl Iterator<Item = Rule> {
        self.syntax.children().filter_map(Rule::cast)
    }
}

// ============================================================================
// ValueSet
// ============================================================================

/// ValueSet definition: ValueSet: Name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSet {
    syntax: FshSyntaxNode,
}

impl AstNode for ValueSet {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::ValueSet
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl ValueSet {
    pub fn name(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }

    pub fn name_token(&self) -> Option<FshSyntaxToken> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident)
    }

    pub fn id(&self) -> Option<IdClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::IdClause).and_then(IdClause::cast)
    }

    pub fn title(&self) -> Option<TitleClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TitleClause).and_then(TitleClause::cast)
    }

    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    pub fn rules(&self) -> impl Iterator<Item = Rule> {
        self.syntax.children().filter_map(Rule::cast)
    }
}

// ============================================================================
// ValueSet components
// ============================================================================

/// ValueSet component wrapper: * include/exclude (concept|filter)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsComponent {
    syntax: FshSyntaxNode,
}

impl AstNode for VsComponent {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsComponent
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsComponent {
    pub fn is_exclude(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .any(|t| t.kind() == FshSyntaxKind::ExcludeKw)
    }

    pub fn is_include(&self) -> bool {
        !self.is_exclude()
    }

    pub fn concept(&self) -> Option<VsConceptComponent> {
        child_of_kind(&self.syntax, FshSyntaxKind::VsConceptComponent)
            .and_then(VsConceptComponent::cast)
    }

    pub fn filter(&self) -> Option<VsFilterComponent> {
        child_of_kind(&self.syntax, FshSyntaxKind::VsFilterComponent)
            .and_then(VsFilterComponent::cast)
    }
}

/// ValueSet concept component: code ["display"] [from ...]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsConceptComponent {
    syntax: FshSyntaxNode,
}

impl AstNode for VsConceptComponent {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsConceptComponent
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsConceptComponent {
    pub fn code(&self) -> Option<CodeRef> {
        child_of_kind(&self.syntax, FshSyntaxKind::CodeRef).and_then(CodeRef::cast)
    }

    pub fn display(&self) -> Option<String> {
        get_string_text(&self.syntax)
    }

    pub fn from_clause(&self) -> Option<VsComponentFrom> {
        child_of_kind(&self.syntax, FshSyntaxKind::VsComponentFrom).and_then(VsComponentFrom::cast)
    }
}

/// ValueSet filter component: codes from ... where ...
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsFilterComponent {
    syntax: FshSyntaxNode,
}

impl AstNode for VsFilterComponent {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsFilterComponent
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsFilterComponent {
    pub fn from_clause(&self) -> Option<VsComponentFrom> {
        child_of_kind(&self.syntax, FshSyntaxKind::VsComponentFrom).and_then(VsComponentFrom::cast)
    }

    pub fn filters(&self) -> Vec<VsFilterDefinition> {
        child_of_kind(&self.syntax, FshSyntaxKind::VsFilterList)
            .and_then(VsFilterList::cast)
            .map(|list| list.definitions())
            .unwrap_or_default()
    }
}

/// ValueSet "from" clause (system/valueset)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsComponentFrom {
    syntax: FshSyntaxNode,
}

impl AstNode for VsComponentFrom {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsComponentFrom
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsComponentFrom {
    pub fn systems(&self) -> Vec<String> {
        self.syntax
            .children()
            .filter(|child| child.kind() == FshSyntaxKind::VsFromSystem)
            .filter_map(VsFromSystem::cast)
            .map(|system| system.system())
            .collect()
    }

    pub fn value_sets(&self) -> Vec<String> {
        self.syntax
            .children()
            .filter(|child| child.kind() == FshSyntaxKind::VsFromValueset)
            .filter_map(VsFromValueset::cast)
            .flat_map(|vs| vs.names())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsFromSystem {
    syntax: FshSyntaxNode,
}

impl AstNode for VsFromSystem {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsFromSystem
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsFromSystem {
    pub fn system(&self) -> String {
        self.syntax
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .skip_while(|t| t.kind() == FshSyntaxKind::SystemKw)
            .map(|t| t.text().to_string())
            .collect::<String>()
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsFromValueset {
    syntax: FshSyntaxNode,
}

impl AstNode for VsFromValueset {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsFromValueset
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsFromValueset {
    pub fn names(&self) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();

        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.into_token() {
                match token.kind() {
                    FshSyntaxKind::ValuesetRefKw => {
                        if !current.is_empty() {
                            parts.push(current.trim().to_string());
                            current.clear();
                        }
                    }
                    FshSyntaxKind::AndKw => {
                        if !current.trim().is_empty() {
                            parts.push(current.trim().to_string());
                            current.clear();
                        }
                    }
                    _ => current.push_str(&token.text().to_string()),
                }
            }
        }

        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }

        parts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsFilterList {
    syntax: FshSyntaxNode,
}

impl AstNode for VsFilterList {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsFilterList
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsFilterList {
    pub fn definitions(&self) -> Vec<VsFilterDefinition> {
        self.syntax
            .children()
            .filter(|child| child.kind() == FshSyntaxKind::VsFilterDefinition)
            .filter_map(VsFilterDefinition::cast)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsFilterDefinition {
    syntax: FshSyntaxNode,
}

impl AstNode for VsFilterDefinition {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsFilterDefinition
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsFilterDefinition {
    /// Get the filter property name
    pub fn property(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .find(|t| t.kind() == FshSyntaxKind::Ident)
            .map(|t| t.text().to_string())
    }

    /// Get the filter operator as a structured node
    pub fn operator(&self) -> Option<VsFilterOperator> {
        child_of_kind(&self.syntax, FshSyntaxKind::VsFilterOperator)
            .and_then(VsFilterOperator::cast)
    }

    /// Get the filter operator as a string
    pub fn operator_string(&self) -> Option<String> {
        self.operator().map(|op| op.text())
    }

    /// Get the filter value as a structured node
    pub fn value(&self) -> Option<VsFilterValue> {
        child_of_kind(&self.syntax, FshSyntaxKind::VsFilterValue).and_then(VsFilterValue::cast)
    }

    /// Get the filter value as a string
    pub fn value_string(&self) -> Option<String> {
        self.value().map(|val| val.text())
    }

    /// Get chained filters (connected by "and")
    pub fn chained_filters(&self) -> Vec<VsFilterDefinition> {
        // Look for sibling VsFilterDefinition nodes
        let mut filters = Vec::new();
        let mut current = self.syntax.next_sibling();
        
        while let Some(node) = current {
            if let Some(filter) = VsFilterDefinition::cast(node.clone()) {
                filters.push(filter);
            }
            current = node.next_sibling();
        }
        
        filters
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsFilterOperator {
    syntax: FshSyntaxNode,
}

impl AstNode for VsFilterOperator {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsFilterOperator
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsFilterOperator {
    pub fn text(&self) -> String {
        let text = self.syntax.text().to_string();
        text.trim().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsFilterValue {
    syntax: FshSyntaxNode,
}

impl AstNode for VsFilterValue {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::VsFilterValue
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl VsFilterValue {
    pub fn text(&self) -> String {
        let text = self.syntax.text().to_string();
        text.trim().to_string()
    }
}

/// Code reference node representing "system#code"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRef {
    syntax: FshSyntaxNode,
}

impl AstNode for CodeRef {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CodeRef
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CodeRef {
    pub fn system(&self) -> Option<String> {
        let mut text = String::new();
        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.into_token() {
                if token.kind() == FshSyntaxKind::Code {
                    break;
                }
                text.push_str(&token.text().to_string());
            }
        }

        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    pub fn code(&self) -> Option<String> {
        token_of_kind(&self.syntax, FshSyntaxKind::Code).map(|token| {
            let text = token.text().to_string();
            text.trim_start_matches('#').trim().to_string()
        })
    }

    pub fn raw(&self) -> String {
        let text = self.syntax.text().to_string();
        text.trim().to_string()
    }
}

// ============================================================================
// CodeSystem
// ============================================================================

/// CodeSystem definition: CodeSystem: Name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSystem {
    syntax: FshSyntaxNode,
}

impl AstNode for CodeSystem {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CodeSystem
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CodeSystem {
    pub fn name(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }

    pub fn name_token(&self) -> Option<FshSyntaxToken> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident)
    }

    pub fn id(&self) -> Option<IdClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::IdClause).and_then(IdClause::cast)
    }

    pub fn title(&self) -> Option<TitleClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TitleClause).and_then(TitleClause::cast)
    }

    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    pub fn rules(&self) -> impl Iterator<Item = Rule> {
        self.syntax.children().filter_map(Rule::cast)
    }
}

// ============================================================================
// Logical
// ============================================================================

/// Logical model definition: Logical: Name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logical {
    syntax: FshSyntaxNode,
}

impl AstNode for Logical {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Logical
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Logical {
    /// Get the logical model name
    pub fn name(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }

    /// Get the syntax node for the logical model name (for precise location)
    pub fn name_token(&self) -> Option<FshSyntaxToken> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident)
    }

    /// Get the parent clause (Parent: BaseType)
    pub fn parent(&self) -> Option<ParentClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::ParentClause).and_then(ParentClause::cast)
    }

    /// Get the id clause
    pub fn id(&self) -> Option<IdClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::IdClause).and_then(IdClause::cast)
    }

    /// Get the title clause
    pub fn title(&self) -> Option<TitleClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TitleClause).and_then(TitleClause::cast)
    }

    /// Get the description clause
    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    /// Get the characteristic codes
    ///
    /// Parses "Characteristics: #code1, #code2" and returns ["code1", "code2"]
    pub fn characteristics(&self) -> Vec<String> {
        let mut codes = Vec::new();
        let mut found_characteristics_kw = false;

        for child in self.syntax.children_with_tokens() {
            if let Some(token) = child.as_token() {
                // Look for Characteristics keyword
                if token.kind() == FshSyntaxKind::CharacteristicsKw {
                    found_characteristics_kw = true;
                    continue;
                }

                // After Characteristics keyword, collect codes
                if found_characteristics_kw {
                    let text = token.text();
                    if text.starts_with('#') {
                        codes.push(text[1..].to_string());
                    }
                }
            }
        }

        codes
    }

    /// Get all rules
    pub fn rules(&self) -> impl Iterator<Item = Rule> {
        self.syntax.children().filter_map(Rule::cast)
    }
}

// ============================================================================
// Resource
// ============================================================================

/// Resource definition: Resource: Name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    syntax: FshSyntaxNode,
}

impl AstNode for Resource {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Resource
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Resource {
    /// Get the resource name
    pub fn name(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }

    /// Get the syntax node for the resource name (for precise location)
    pub fn name_token(&self) -> Option<FshSyntaxToken> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident)
    }

    /// Get the parent clause (Parent: BaseType)
    pub fn parent(&self) -> Option<ParentClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::ParentClause).and_then(ParentClause::cast)
    }

    /// Get the id clause
    pub fn id(&self) -> Option<IdClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::IdClause).and_then(IdClause::cast)
    }

    /// Get the title clause
    pub fn title(&self) -> Option<TitleClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TitleClause).and_then(TitleClause::cast)
    }

    /// Get the description clause
    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    /// Get all rules
    pub fn rules(&self) -> impl Iterator<Item = Rule> {
        self.syntax.children().filter_map(Rule::cast)
    }
}

// ============================================================================
// Alias
// ============================================================================

/// Alias definition: Alias: Name = Value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    syntax: FshSyntaxNode,
}

impl AstNode for Alias {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Alias
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Alias {
    pub fn name(&self) -> Option<String> {
        // First IDENT after "Alias:"
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .find(|t| t.kind() == FshSyntaxKind::Ident)
            .map(|t| t.text().trim().to_string())
    }

    pub fn value(&self) -> Option<String> {
        // Collect all tokens after "=" until end of alias node
        // URLs are lexed as multiple tokens, so we need to concatenate them
        let mut found_equals = false;
        let mut value_parts = Vec::new();

        for child in self.syntax.children_with_tokens() {
            if let Some(token) = child.as_token() {
                if token.kind() == FshSyntaxKind::Equals {
                    found_equals = true;
                    continue;
                }

                if found_equals {
                    // Skip whitespace and newlines at the start
                    if value_parts.is_empty()
                        && (token.kind() == FshSyntaxKind::Whitespace
                            || token.kind() == FshSyntaxKind::Newline)
                    {
                        continue;
                    }

                    // Stop at newline
                    if token.kind() == FshSyntaxKind::Newline {
                        break;
                    }

                    // Handle string literals specially
                    if token.kind() == FshSyntaxKind::String {
                        let text = token.text();
                        if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
                            return Some(text[1..text.len() - 1].to_string());
                        } else {
                            return Some(text.to_string());
                        }
                    }

                    // Collect all other tokens
                    // CommentLine tokens in URLs need special handling - they contain "//" prefix
                    if token.kind() == FshSyntaxKind::CommentLine {
                        // This is actually part of a URL (http://...), not a real comment
                        value_parts.push(token.text().to_string());
                    } else if token.kind() != FshSyntaxKind::Whitespace {
                        value_parts.push(token.text().to_string());
                    }
                }
            }
        }

        if value_parts.is_empty() {
            None
        } else {
            Some(value_parts.join(""))
        }
    }
}

// ============================================================================
// Mapping
// ============================================================================

/// Mapping definition: Mapping: name
///
/// Mappings define ConceptMaps for translating between code systems.
///
/// # Example
///
/// ```fsh
/// Mapping: MyMapping
/// Id: my-mapping
/// Source: SourceValueSet
/// Target: "http://target-system.org"
/// * code1 -> code2 "Comment about mapping"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    syntax: FshSyntaxNode,
}

impl AstNode for Mapping {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Mapping
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Mapping {
    /// Get the mapping name
    pub fn name(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }

    /// Get the syntax node for the mapping name (for precise location)
    pub fn name_token(&self) -> Option<FshSyntaxToken> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident)
    }

    /// Get the id clause
    pub fn id(&self) -> Option<IdClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::IdClause).and_then(IdClause::cast)
    }

    /// Get the source clause
    pub fn source(&self) -> Option<SourceClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::SourceClause).and_then(SourceClause::cast)
    }

    /// Get the target clause
    pub fn target(&self) -> Option<TargetClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TargetClause).and_then(TargetClause::cast)
    }

    /// Get the title clause
    pub fn title(&self) -> Option<TitleClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TitleClause).and_then(TitleClause::cast)
    }

    /// Get the description clause
    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    /// Get all rules (mapping rules)
    pub fn rules(&self) -> impl Iterator<Item = Rule> {
        self.syntax.children().filter_map(Rule::cast)
    }
}

// ============================================================================
// Instance
// ============================================================================

/// Instance definition: Instance: name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    syntax: FshSyntaxNode,
}

impl AstNode for Instance {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Instance
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Instance {
    /// Get instance name
    pub fn name(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .find(|t| t.kind() == FshSyntaxKind::Ident)
            .map(|t| t.text().trim().to_string())
    }

    /// Get InstanceOf clause
    pub fn instance_of(&self) -> Option<InstanceOfClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::InstanceofClause)
            .and_then(InstanceOfClause::cast)
    }

    /// Get Usage clause
    pub fn usage(&self) -> Option<UsageClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::UsageClause).and_then(UsageClause::cast)
    }

    /// Get Id clause
    pub fn id(&self) -> Option<IdClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::IdClause).and_then(IdClause::cast)
    }

    /// Get Title clause
    pub fn title(&self) -> Option<TitleClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::TitleClause).and_then(TitleClause::cast)
    }

    /// Get Description clause
    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    /// Get all rules
    pub fn rules(&self) -> impl Iterator<Item = Rule> {
        self.syntax.children().filter_map(Rule::cast)
    }
}

// ============================================================================
// Invariant
// ============================================================================

/// Invariant definition: Invariant: name
///
/// Defines a constraint that can be referenced by ObeysRule.
///
/// # Example
///
/// ```fsh
/// Invariant: inv-1
/// Description: "SHALL have a contact party or an organization or both"
/// Expression: "telecom or name"
/// Severity: #error
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invariant {
    syntax: FshSyntaxNode,
}

impl AstNode for Invariant {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Invariant
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Invariant {
    /// Get the invariant name/key
    pub fn name(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }

    /// Get the invariant name token (for precise location)
    pub fn name_token(&self) -> Option<FshSyntaxToken> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident)
    }

    /// Get the description clause
    pub fn description(&self) -> Option<DescriptionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::DescriptionClause)
            .and_then(DescriptionClause::cast)
    }

    /// Get the severity clause
    pub fn severity(&self) -> Option<SeverityClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::SeverityClause).and_then(SeverityClause::cast)
    }

    /// Get the expression clause (FHIRPath)
    pub fn expression(&self) -> Option<ExpressionClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::ExpressionClause)
            .and_then(ExpressionClause::cast)
    }

    /// Get the xpath clause (optional XPath 1.0 expression)
    pub fn xpath(&self) -> Option<XPathClause> {
        child_of_kind(&self.syntax, FshSyntaxKind::XpathClause).and_then(XPathClause::cast)
    }
}

// ============================================================================
// Metadata Clauses
// ============================================================================

/// Parent clause: Parent: ResourceType
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentClause {
    syntax: FshSyntaxNode,
}

impl AstNode for ParentClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::ParentClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl ParentClause {
    pub fn value(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }
}

/// Id clause: Id: resource-id
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdClause {
    syntax: FshSyntaxNode,
}

impl AstNode for IdClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::IdClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl IdClause {
    pub fn value(&self) -> Option<String> {
        get_ident_text(&self.syntax)
    }
}

/// Title clause: Title: "Resource Title"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleClause {
    syntax: FshSyntaxNode,
}

impl AstNode for TitleClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::TitleClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl TitleClause {
    pub fn value(&self) -> Option<String> {
        get_string_text(&self.syntax)
    }
}

/// Description clause: Description: "Resource description"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptionClause {
    syntax: FshSyntaxNode,
}

impl AstNode for DescriptionClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::DescriptionClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl DescriptionClause {
    pub fn value(&self) -> Option<String> {
        get_string_text(&self.syntax)
    }
}

/// InstanceOf clause: InstanceOf: ProfileName
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceOfClause {
    syntax: FshSyntaxNode,
}

impl AstNode for InstanceOfClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::InstanceofClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl InstanceOfClause {
    pub fn value(&self) -> Option<String> {
        // Get the identifier after "InstanceOf:"
        get_ident_text(&self.syntax)
    }
}

/// Usage clause: Usage: #example
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageClause {
    syntax: FshSyntaxNode,
}

impl AstNode for UsageClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::UsageClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl UsageClause {
    pub fn value(&self) -> Option<String> {
        // Get the code after "Usage:" (starts with #)
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .find(|t| t.kind() == FshSyntaxKind::Code)
            .map(|t| {
                let text = t.text().trim();
                // Remove # prefix if present
                if text.starts_with('#') {
                    text[1..].to_string()
                } else {
                    text.to_string()
                }
            })
    }
}

/// Source clause: Source: SourceSystem
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceClause {
    syntax: FshSyntaxNode,
}

impl AstNode for SourceClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::SourceClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl SourceClause {
    pub fn value(&self) -> Option<String> {
        // Get the identifier after "Source:"
        get_ident_text(&self.syntax)
    }
}

/// Target clause: Target: "http://target-system.org"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetClause {
    syntax: FshSyntaxNode,
}

impl AstNode for TargetClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::TargetClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl TargetClause {
    pub fn value(&self) -> Option<String> {
        // Get the string after "Target:"
        get_string_text(&self.syntax)
    }
}

/// Severity clause: Severity: #error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeverityClause {
    syntax: FshSyntaxNode,
}

impl AstNode for SeverityClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::SeverityClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl SeverityClause {
    pub fn value(&self) -> Option<String> {
        // Get the identifier after "Severity:" and optional #
        // Parser stores # and identifier as separate tokens
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .find(|t| t.kind() == FshSyntaxKind::Ident)
            .map(|t| t.text().trim().to_string())
    }
}

/// Expression clause: Expression: "FHIRPath expression"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionClause {
    syntax: FshSyntaxNode,
}

impl AstNode for ExpressionClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::ExpressionClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl ExpressionClause {
    pub fn value(&self) -> Option<String> {
        // Get the string after "Expression:"
        get_string_text(&self.syntax)
    }
}

/// XPath clause: XPath: "XPath 1.0 expression"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathClause {
    syntax: FshSyntaxNode,
}

impl AstNode for XPathClause {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::XpathClause
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl XPathClause {
    pub fn value(&self) -> Option<String> {
        // Get the string after "XPath:"
        get_string_text(&self.syntax)
    }
}

// ============================================================================
// Rules
// ============================================================================

/// Unified rule type (enum over all rule types)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    Card(CardRule),
    Flag(FlagRule),
    ValueSet(ValueSetRule),
    FixedValue(FixedValueRule),
    Path(PathRule),
    Contains(ContainsRule),
    Only(OnlyRule),
    Obeys(ObeysRule),
    AddElement(AddElementRule),
    Mapping(MappingRule),
    CaretValue(CaretValueRule),
    CodeCaretValue(CodeCaretValueRule),
    CodeInsert(CodeInsertRule),
}

impl Rule {
    pub fn cast(node: FshSyntaxNode) -> Option<Self> {
        match node.kind() {
            FshSyntaxKind::CardRule => CardRule::cast(node).map(Rule::Card),
            FshSyntaxKind::FlagRule => FlagRule::cast(node).map(Rule::Flag),
            FshSyntaxKind::ValuesetRule => ValueSetRule::cast(node).map(Rule::ValueSet),
            FshSyntaxKind::FixedValueRule => FixedValueRule::cast(node).map(Rule::FixedValue),
            FshSyntaxKind::PathRule => PathRule::cast(node).map(Rule::Path),
            FshSyntaxKind::ContainsRule => ContainsRule::cast(node).map(Rule::Contains),
            FshSyntaxKind::OnlyRule => OnlyRule::cast(node).map(Rule::Only),
            FshSyntaxKind::ObeysRule => ObeysRule::cast(node).map(Rule::Obeys),
            FshSyntaxKind::AddElementRule => AddElementRule::cast(node).map(Rule::AddElement),
            FshSyntaxKind::MappingRule => MappingRule::cast(node).map(Rule::Mapping),
            FshSyntaxKind::CaretValueRule => CaretValueRule::cast(node).map(Rule::CaretValue),
            FshSyntaxKind::CodeCaretValueRule => {
                CodeCaretValueRule::cast(node).map(Rule::CodeCaretValue)
            }
            FshSyntaxKind::CodeInsertRule => CodeInsertRule::cast(node).map(Rule::CodeInsert),
            _ => None,
        }
    }

    pub fn syntax(&self) -> &FshSyntaxNode {
        match self {
            Rule::Card(r) => r.syntax(),
            Rule::Flag(r) => r.syntax(),
            Rule::ValueSet(r) => r.syntax(),
            Rule::FixedValue(r) => r.syntax(),
            Rule::Path(r) => r.syntax(),
            Rule::Contains(r) => r.syntax(),
            Rule::Only(r) => r.syntax(),
            Rule::Obeys(r) => r.syntax(),
            Rule::AddElement(r) => r.syntax(),
            Rule::Mapping(r) => r.syntax(),
            Rule::CaretValue(r) => r.syntax(),
            Rule::CodeCaretValue(r) => r.syntax(),
            Rule::CodeInsert(r) => r.syntax(),
        }
    }
}

/// Cardinality rule: * path 0..1 MS
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardRule {
    syntax: FshSyntaxNode,
}

impl AstNode for CardRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CardRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CardRule {
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node, not a child
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    /// Get the cardinality as a structured CardinalityNode
    pub fn cardinality(&self) -> Option<CardinalityNode> {
        self.syntax
            .children()
            .find_map(CardinalityNode::cast)
    }

    /// Get the cardinality as a string (for backward compatibility)
    pub fn cardinality_string(&self) -> Option<String> {
        self.cardinality().map(|c| c.as_string())
    }

    /// Get all flag values as structured FlagValue enum
    pub fn flags(&self) -> Vec<FlagValue> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter_map(|t| FlagValue::from_syntax_kind(t.kind()))
            .collect()
    }

    /// Get flag values as strings (for backward compatibility)
    pub fn flags_as_strings(&self) -> Vec<String> {
        self.flags()
            .into_iter()
            .map(|f| f.as_str().to_string())
            .collect()
    }
}

/// Flag rule: * path MS SU
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagRule {
    syntax: FshSyntaxNode,
}

impl AstNode for FlagRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::FlagRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl FlagRule {
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node, not a child
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    /// Get all flag values as structured FlagValue enum
    pub fn flags(&self) -> Vec<FlagValue> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter_map(|t| FlagValue::from_syntax_kind(t.kind()))
            .collect()
    }

    /// Get flag values as strings (for backward compatibility)
    pub fn flags_as_strings(&self) -> Vec<String> {
        self.flags()
            .into_iter()
            .map(|f| f.as_str().to_string())
            .collect()
    }

    /// Check if flags have any conflicts
    pub fn has_flag_conflicts(&self) -> bool {
        let flags = self.flags();
        for (i, flag1) in flags.iter().enumerate() {
            for flag2 in flags.iter().skip(i + 1) {
                if flag1.conflicts_with(flag2) {
                    return true;
                }
            }
        }
        false
    }

    /// Get conflicting flag pairs
    pub fn flag_conflicts(&self) -> Vec<(FlagValue, FlagValue)> {
        let flags = self.flags();
        let mut conflicts = Vec::new();
        
        for (i, flag1) in flags.iter().enumerate() {
            for flag2 in flags.iter().skip(i + 1) {
                if flag1.conflicts_with(flag2) {
                    conflicts.push((flag1.clone(), flag2.clone()));
                }
            }
        }
        
        conflicts
    }
}

/// ValueSet binding rule: * path from ValueSetName (required)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSetRule {
    syntax: FshSyntaxNode,
}

impl AstNode for ValueSetRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::ValuesetRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl ValueSetRule {
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node, not a child
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    /// Get ValueSet components (structured access)
    pub fn value_set_components(&self) -> impl Iterator<Item = VsComponent> + '_ {
        self.syntax.children().filter_map(VsComponent::cast)
    }

    /// Get ValueSet filter definitions (structured access)
    pub fn filter_definitions(&self) -> impl Iterator<Item = VsFilterDefinition> + '_ {
        self.syntax.children().filter_map(VsFilterDefinition::cast)
    }

    /// Get the ValueSet name/URL (for backward compatibility)
    pub fn value_set(&self) -> Option<String> {
        // ValueSet can be:
        // 1. A simple identifier: "MyValueSet"
        // 2. A URL: "http://hl7.org/fhir/ValueSet/marital-status"
        //
        // URLs are problematic because the lexer treats "//" as a comment.
        // We need to collect all tokens after "from" until we hit "(" or newline

        let text = self.syntax.text().to_string();

        // Remove "from" keyword and any leading/trailing whitespace
        let after_from = text.trim_start().strip_prefix("from")?;
        let after_from = after_from.trim_start();

        // Find the binding strength part (if exists) and extract everything before it
        if let Some(paren_pos) = after_from.find('(') {
            Some(after_from[..paren_pos].trim().to_string())
        } else {
            // No binding strength, take everything until newline
            Some(after_from.trim().to_string())
        }
    }

    pub fn strength(&self) -> Option<String> {
        // Extract binding strength from parentheses: (required), (extensible), etc.
        let text = self.syntax.text().to_string();
        if let Some(start) = text.find('(')
            && let Some(end) = text.find(')')
        {
            return Some(text[start + 1..end].trim().to_string());
        }
        None
    }
}

/// Fixed value rule: * path = value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedValueRule {
    syntax: FshSyntaxNode,
}

impl AstNode for FixedValueRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::FixedValueRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl FixedValueRule {
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node, not a child
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    pub fn value(&self) -> Option<String> {
        // Value can be code (with optional display), string, number, identifier, or boolean
        // Priority: Code > Identifier > String > Number > Boolean
        // This ensures `#collection "Collection"` returns `#collection` not `"Collection"`
        token_of_kind(&self.syntax, FshSyntaxKind::Code)
            .map(|t| t.text().to_string())
            .or_else(|| get_ident_text(&self.syntax))
            .or_else(|| get_string_text(&self.syntax))
            .or_else(|| {
                token_of_kind(&self.syntax, FshSyntaxKind::Integer).map(|t| t.text().to_string())
            })
            .or_else(|| {
                token_of_kind(&self.syntax, FshSyntaxKind::True).map(|t| t.text().to_string())
            })
            .or_else(|| {
                token_of_kind(&self.syntax, FshSyntaxKind::False).map(|t| t.text().to_string())
            })
    }
}

/// Path rule: * path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRule {
    syntax: FshSyntaxNode,
}

impl AstNode for PathRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::PathRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl PathRule {
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node, not a child
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }
}

/// Contains rule: * path contains Item1 and Item2
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainsRule {
    syntax: FshSyntaxNode,
}

impl AstNode for ContainsRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::ContainsRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl ContainsRule {
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node, not a child
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    /// Get the contains items (slice names)
    /// Example: "Item1 and Item2" or "Item1 named slice1"
    pub fn items(&self) -> Vec<String> {
        let text = self.syntax.text().to_string();

        // Remove "contains" keyword and clean up whitespace/newlines
        let after_contains = text
            .trim_start()
            .strip_prefix("contains")
            .unwrap_or(&text)
            .trim();

        // Clean up the text - replace newlines with spaces and normalize whitespace
        let cleaned_text = after_contains.replace('\n', " ").replace('\r', " ");

        // Split by "and" keyword and extract item names
        cleaned_text
            .split("and")
            .map(|item| {
                let item = item.trim();

                // Handle "Item1 named slice1" format - extract the item name before "named"
                if let Some(named_pos) = item.find("named") {
                    item[..named_pos].trim().to_string()
                } else {
                    // Extract just the extension name (first word before cardinality/flags)
                    let words: Vec<&str> = item.split_whitespace().collect();
                    if let Some(first_word) = words.first() {
                        let first_word = first_word.trim();
                        // Skip cardinality patterns and flags
                        if !first_word.is_empty()
                            && !first_word.contains("..")  // Skip cardinality like "0..1"
                            && first_word != "MS"          // Skip MustSupport flag
                            && first_word != "SU"          // Skip Summary flag
                            && !first_word.starts_with("//")
                        // Skip comments
                        {
                            first_word.to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        item.to_string()
                    }
                }
            })
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Only rule: * path only Type1 or Type2
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlyRule {
    syntax: FshSyntaxNode,
}

impl AstNode for OnlyRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::OnlyRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl OnlyRule {
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node, not a child
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    /// Get the allowed types
    /// Example: "String" or "String or Integer"
    pub fn types(&self) -> Vec<String> {
        let text = self.syntax.text().to_string();

        // Remove "only" keyword
        let after_only = text
            .trim_start()
            .strip_prefix("only")
            .unwrap_or(&text)
            .trim();

        // Split by "or" keyword
        after_only
            .split("or")
            .map(|t| t.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Obeys rule: * path obeys Invariant1 and Invariant2
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObeysRule {
    syntax: FshSyntaxNode,
}

impl AstNode for ObeysRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::ObeysRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl ObeysRule {
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node, not a child
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    /// Get the invariant names
    /// Example: "inv-1" or "inv-1 and inv-2"
    pub fn invariants(&self) -> Vec<String> {
        let text = self.syntax.text().to_string();

        // Remove "obeys" keyword
        let after_obeys = text
            .trim_start()
            .strip_prefix("obeys")
            .unwrap_or(&text)
            .trim();

        // Split by "and" keyword
        after_obeys
            .split("and")
            .map(|inv| inv.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// AddElement rule: * elementName 0..1 MS Type "short" "definition"
///
/// Used in Logical models and Resources to define new elements with types.
///
/// Grammar: * path CARD flags* TYPE (or TYPE)* STRING STRING?
///
/// # Example
///
/// ```fsh
/// * name 0..* HumanName "Name(s) of the human" "The names by which the human is or has been known"
/// * status 1..1 MS code "Status of the record"
/// * value[x] 0..1 string or integer "The value"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddElementRule {
    syntax: FshSyntaxNode,
}

impl AstNode for AddElementRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::AddElementRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl AddElementRule {
    /// Get the element path
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    /// Get the cardinality as a string (e.g., "0..1", "1..*")
    pub fn cardinality(&self) -> Option<String> {
        let mut parts = Vec::new();

        // Find min (Integer)
        if let Some(min_token) = token_of_kind(&self.syntax, FshSyntaxKind::Integer) {
            parts.push(min_token.text().trim().to_string());
        }

        // Find max (Integer or Asterisk)
        for child in self.syntax.children_with_tokens() {
            if let Some(token) = child.as_token() {
                match token.kind() {
                    FshSyntaxKind::Integer => {
                        if parts.len() == 1 {
                            parts.push(token.text().trim().to_string());
                            break;
                        }
                    }
                    FshSyntaxKind::Asterisk => {
                        if parts.len() == 1 {
                            parts.push("*".to_string());
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }

        if parts.len() == 2 {
            Some(format!("{}..{}", parts[0], parts[1]))
        } else {
            None
        }
    }

    /// Get the cardinality as a string (for backward compatibility)
    pub fn cardinality_string(&self) -> Option<String> {
        self.cardinality()
    }

    /// Get all flag values as structured FlagValue enum
    pub fn flags(&self) -> Vec<FlagValue> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter_map(|t| FlagValue::from_syntax_kind(t.kind()))
            .collect()
    }

    /// Get flag values as strings (for backward compatibility)
    pub fn flags_as_strings(&self) -> Vec<String> {
        self.flags()
            .into_iter()
            .map(|f| f.as_str().to_string())
            .collect()
    }

    /// Get the element types (can be multiple with "or")
    /// Example: ["HumanName"] or ["string", "integer"]
    pub fn types(&self) -> Vec<String> {
        let mut types = Vec::new();
        let mut in_type_section = false;

        for child in self.syntax.children_with_tokens() {
            if let Some(token) = child.as_token() {
                match token.kind() {
                    FshSyntaxKind::Ident => {
                        // After cardinality and flags, identifiers are types
                        if in_type_section
                            || token.text().chars().next().unwrap_or(' ').is_uppercase()
                        {
                            in_type_section = true;
                            types.push(token.text().trim().to_string());
                        }
                    }
                    FshSyntaxKind::String => {
                        // Stop when we hit the short description
                        break;
                    }
                    _ => {}
                }
            }
        }

        types
    }

    /// Get the short description (first string literal)
    pub fn short(&self) -> Option<String> {
        get_string_text(&self.syntax)
    }

    /// Get the full definition (second string literal, if present)
    pub fn definition(&self) -> Option<String> {
        let mut string_count = 0;
        for child in self.syntax.children_with_tokens() {
            if let Some(token) = child.as_token() {
                if token.kind() == FshSyntaxKind::String {
                    string_count += 1;
                    if string_count == 2 {
                        let text = token.text();
                        // Remove surrounding quotes
                        if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
                            return Some(text[1..text.len() - 1].to_string());
                        } else {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

// ============================================================================
// MappingRule
// ============================================================================

/// Mapping rule: * path -> "target" "comment" #language
///
/// Used in Mapping definitions to map elements to external specifications.
///
/// Grammar: * path -> STRING STRING? CODE?
///
/// # Examples
///
/// ```fsh
/// * name -> "PID-5"
/// * status -> "OBX-11" "Observation result status"
/// * identifier -> "Patient.identifier" "Business identifier" #en
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingRule {
    syntax: FshSyntaxNode,
}

impl AstNode for MappingRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::MappingRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl MappingRule {
    /// Get the element path (left side of ->)
    pub fn path(&self) -> Option<Path> {
        // The path is the previous sibling of the rule node
        self.syntax
            .prev_sibling()
            .filter(|n| n.kind() == FshSyntaxKind::Path)
            .and_then(Path::cast)
    }

    /// Get the target mapping expression (first string after ->)
    pub fn map(&self) -> Option<String> {
        get_string_text(&self.syntax)
    }

    /// Get the comment (second string literal, if present)
    pub fn comment(&self) -> Option<String> {
        let mut string_count = 0;
        for child in self.syntax.children_with_tokens() {
            if let Some(token) = child.as_token() {
                if token.kind() == FshSyntaxKind::String {
                    string_count += 1;
                    if string_count == 2 {
                        let text = token.text();
                        // Remove surrounding quotes
                        if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
                            return Some(text[1..text.len() - 1].to_string());
                        } else {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Get the language code (e.g., #en, #en-US)
    pub fn language(&self) -> Option<String> {
        for child in self.syntax.children_with_tokens() {
            if let Some(token) = child.as_token() {
                if token.kind() == FshSyntaxKind::Code {
                    let text = token.text();
                    // Remove leading #
                    if text.starts_with('#') {
                        return Some(text[1..].to_string());
                    } else {
                        return Some(text.to_string());
                    }
                }
            }
        }
        None
    }
}

/// Caret value rule: * path ^field = value or * ^field = value
/// Used to set metadata on elements or the profile itself
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaretValueRule {
    syntax: FshSyntaxNode,
}

impl AstNode for CaretValueRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CaretValueRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CaretValueRule {
    /// Get the element path (if present, for element-level caret rules)
    /// For profile-level caret rules (like * ^version = "1.0"), this returns None
    pub fn element_path(&self) -> Option<Path> {
        // Look for a path before the caret path
        // The structure is: * [element_path] ^field = value
        // We need to find the first Path that doesn't start with a caret
        for sibling in self.syntax.siblings_with_tokens(rowan::Direction::Prev) {
            if let Some(node) = sibling.as_node() {
                if node.kind() == FshSyntaxKind::Path {
                    // Check if this path starts with a caret (^field path)
                    let text = node.text().to_string();
                    if !text.trim().starts_with('^') {
                        return Path::cast(FshSyntaxNode::from(node.clone()));
                    }
                }
            }
        }
        None
    }

    /// Get the caret path (the ^field part)
    pub fn caret_path(&self) -> Option<Path> {
        // Find the Path child that represents the caret field
        for child in self.syntax.children() {
            if child.kind() == FshSyntaxKind::Path {
                return Path::cast(child);
            }
        }
        None
    }

    /// Get the field name (without the caret)
    pub fn field(&self) -> Option<String> {
        self.caret_path().map(|p| {
            let text = p.syntax().text().to_string();
            // Remove leading ^ if present
            if text.starts_with('^') {
                text[1..].to_string()
            } else {
                text
            }
        })
    }

    /// Get the assigned value
    pub fn value(&self) -> Option<String> {
        // Value can be code (with optional display), string, number, identifier, or boolean
        // Priority: Code > Identifier > String > Number > Boolean
        token_of_kind(&self.syntax, FshSyntaxKind::Code)
            .map(|t| t.text().to_string())
            .or_else(|| get_ident_text(&self.syntax))
            .or_else(|| get_string_text(&self.syntax))
            .or_else(|| {
                token_of_kind(&self.syntax, FshSyntaxKind::Integer).map(|t| t.text().to_string())
            })
            .or_else(|| {
                token_of_kind(&self.syntax, FshSyntaxKind::Decimal).map(|t| t.text().to_string())
            })
            .or_else(|| {
                token_of_kind(&self.syntax, FshSyntaxKind::True).map(|t| t.text().to_string())
            })
            .or_else(|| {
                token_of_kind(&self.syntax, FshSyntaxKind::False).map(|t| t.text().to_string())
            })
    }
}

/// Code caret value rule: * #code ^property = value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCaretValueRule {
    syntax: FshSyntaxNode,
}

impl AstNode for CodeCaretValueRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CodeCaretValueRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CodeCaretValueRule {
    /// Get the code value (first code in the rule)
    pub fn code_value(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .find(|t| t.kind() == FshSyntaxKind::Code)
            .map(|t| t.text().trim_start_matches('#').to_string())
    }

    /// Get all codes in the rule
    pub fn codes(&self) -> Vec<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .filter(|t| t.kind() == FshSyntaxKind::Code)
            .map(|t| t.text().trim_start_matches('#').to_string())
            .collect()
    }

    /// Get the caret path (^property)
    pub fn caret_path(&self) -> Option<Path> {
        child_of_kind(&self.syntax, FshSyntaxKind::Path).and_then(Path::cast)
    }

    /// Get the assigned value
    pub fn assigned_value(&self) -> Option<String> {
        self.value()
    }

    /// Get the assigned value (alias for backward compatibility)
    pub fn value(&self) -> Option<String> {
        let mut after_equals = false;

        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.into_token() {
                match token.kind() {
                    FshSyntaxKind::Equals | FshSyntaxKind::PlusEquals => {
                        after_equals = true;
                        continue;
                    }
                    FshSyntaxKind::Whitespace | FshSyntaxKind::CommentLine => continue,
                    _ if !after_equals => continue,
                    FshSyntaxKind::String => {
                        let text = token.text().to_string();
                        let trimmed = text.trim();
                        return Some(trimmed.trim_matches('"').to_string());
                    }
                    FshSyntaxKind::Code
                    | FshSyntaxKind::Ident
                    | FshSyntaxKind::Integer
                    | FshSyntaxKind::Decimal
                    | FshSyntaxKind::True
                    | FshSyntaxKind::False
                    | FshSyntaxKind::Reference
                    | FshSyntaxKind::Canonical
                    | FshSyntaxKind::CodeableReference => {
                        return Some(token.text().to_string());
                    }
                    _ => {}
                }
            }
        }

        None
    }
}

/// Code insert rule: * #code insert RuleSet(...)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeInsertRule {
    syntax: FshSyntaxNode,
}

impl AstNode for CodeInsertRule {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CodeInsertRule
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CodeInsertRule {
    /// Get the code value (first code in the rule)
    pub fn code_value(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .find(|t| t.kind() == FshSyntaxKind::Code)
            .map(|t| t.text().trim_start_matches('#').to_string())
    }

    /// Get all codes in the rule
    pub fn codes(&self) -> Vec<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .filter(|t| t.kind() == FshSyntaxKind::Code)
            .map(|t| t.text().trim_start_matches('#').to_string())
            .collect()
    }

    /// Get the ruleset reference
    pub fn ruleset_reference(&self) -> Option<String> {
        self.rule_set()
    }

    /// Get the ruleset reference (alias for backward compatibility)
    pub fn rule_set(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|c| c.into_token())
            .find(|t| {
                matches!(
                    t.kind(),
                    FshSyntaxKind::Ident
                        | FshSyntaxKind::PlainParamToken
                        | FshSyntaxKind::BracketedParamToken
                )
            })
            .map(|t| t.text().to_string())
    }

    pub fn arguments(&self) -> Vec<String> {
        child_of_kind(&self.syntax, FshSyntaxKind::InsertRuleArgs)
            .and_then(InsertRuleArguments::cast)
            .map(|args| args.items())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertRuleArguments {
    syntax: FshSyntaxNode,
}

impl AstNode for InsertRuleArguments {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::InsertRuleArgs
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl InsertRuleArguments {
    pub fn items(&self) -> Vec<String> {
        let mut items = Vec::new();
        let mut current = String::new();

        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.into_token() {
                match token.kind() {
                    FshSyntaxKind::Comma => {
                        if !current.trim().is_empty() {
                            items.push(current.trim().to_string());
                            current.clear();
                        }
                    }
                    _ => current.push_str(&token.text().to_string()),
                }
            }
        }

        if !current.trim().is_empty() {
            items.push(current.trim().to_string());
        }

        items
    }
}

// ============================================================================
// Path
// ============================================================================

/// Path expression: name.given or identifier[0].value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    syntax: FshSyntaxNode,
}

impl AstNode for Path {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::Path
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl Path {
    /// Get the full path as a string
    pub fn as_string(&self) -> String {
        self.syntax.text().to_string().trim().to_string()
    }

    /// Get path segments as structured PathSegment nodes
    pub fn segments(&self) -> impl Iterator<Item = PathSegment> + '_ {
        self.syntax.children().filter_map(PathSegment::cast)
    }

    /// Get path segments as strings (for backward compatibility)
    pub fn segments_as_strings(&self) -> Vec<String> {
        let text = self.syntax.text().to_string().trim().to_string();
        if text == "." {
            return vec![".".to_string()];
        }

        let mut segments = Vec::new();
        let mut current = String::new();

        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.into_token() {
                match token.kind() {
                    FshSyntaxKind::Whitespace | FshSyntaxKind::Newline => continue,
                    FshSyntaxKind::Dot => {
                        let trimmed = current.trim();
                        if !trimmed.is_empty() {
                            segments.push(trimmed.to_string());
                            current.clear();
                        }
                    }
                    _ => {
                        current.push_str(token.text().as_ref());
                    }
                }
            }
        }

        let trimmed = current.trim();
        if !trimmed.is_empty() {
            segments.push(trimmed.to_string());
        }

        segments
    }
}

// ============================================================================
// CardinalityNode
// ============================================================================

/// Cardinality node representing min..max or single value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardinalityNode {
    syntax: FshSyntaxNode,
}

impl AstNode for CardinalityNode {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CardinalityNode
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CardinalityNode {
    /// Get the minimum cardinality value
    pub fn min(&self) -> Option<u32> {
        // Find the first number token
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .find(|token| token.kind() == FshSyntaxKind::Integer)
            .and_then(|token| token.text().parse().ok())
    }

    /// Get the maximum cardinality value as string (handles "*" for unbounded)
    pub fn max(&self) -> Option<String> {
        let mut found_range = false;
        
        for child in self.syntax.children_with_tokens() {
            if let Some(token) = child.into_token() {
                match token.kind() {
                    FshSyntaxKind::Range => {
                        found_range = true;
                    }
                    FshSyntaxKind::Integer if found_range => {
                        return Some(token.text().to_string());
                    }
                    FshSyntaxKind::Asterisk if found_range => {
                        return Some("*".to_string());
                    }
                    _ => {}
                }
            }
        }
        
        // If no range found, max equals min (single value cardinality)
        if !found_range {
            self.min().map(|m| m.to_string())
        } else {
            None
        }
    }

    /// Check if this is unbounded cardinality (max = "*")
    pub fn is_unbounded(&self) -> bool {
        self.max().map_or(false, |max| max == "*")
    }

    /// Get the full cardinality as a string (e.g., "0..1", "1..*", "5")
    pub fn as_string(&self) -> String {
        if let (Some(min), Some(max)) = (self.min(), self.max()) {
            if min.to_string() == max {
                // Single value cardinality
                min.to_string()
            } else {
                // Range cardinality
                format!("{}..{}", min, max)
            }
        } else {
            // Fallback to raw text
            self.syntax.text().to_string().trim().to_string()
        }
    }

    /// Check if cardinality contains a specific string (for backward compatibility)
    pub fn contains(&self, s: &str) -> bool {
        self.as_string().contains(s)
    }
}

impl std::fmt::Display for CardinalityNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl Default for CardinalityNode {
    fn default() -> Self {
        // Create a minimal CardinalityNode with empty syntax
        // This is a fallback for when no cardinality is specified
        use rowan::GreenNodeBuilder;
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(FshSyntaxKind::CardinalityNode.into());
        builder.token(FshSyntaxKind::Integer.into(), "1");
        builder.finish_node();
        
        let green = builder.finish();
        let syntax = FshSyntaxNode::new_root(green);
        CardinalityNode { syntax }
    }
}

// ============================================================================
// PathSegment
// ============================================================================

/// Individual path segment (identifier, number, keyword, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment {
    syntax: FshSyntaxNode,
}

impl AstNode for PathSegment {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::PathSegment
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl PathSegment {
    /// Get the identifier text of this path segment
    pub fn identifier(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .find(|token| {
                matches!(
                    token.kind(),
                    FshSyntaxKind::Ident
                        | FshSyntaxKind::Integer
                        | FshSyntaxKind::DateTime
                        | FshSyntaxKind::Time
                        // Add other alpha keywords as needed
                )
            })
            .map(|token| token.text().to_string())
    }

    /// Check if this path segment is numeric
    pub fn is_numeric(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .any(|token| token.kind() == FshSyntaxKind::Integer)
    }

    /// Check if this path segment is a datetime
    pub fn is_datetime(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .any(|token| token.kind() == FshSyntaxKind::DateTime)
    }

    /// Check if this path segment is a time
    pub fn is_time(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(|child| child.into_token())
            .any(|token| token.kind() == FshSyntaxKind::Time)
    }

    /// Get the raw text of this path segment
    pub fn text(&self) -> String {
        self.syntax.text().to_string().trim().to_string()
    }
}

// ============================================================================
// Value Expression Nodes
// ============================================================================

/// Regex value: /pattern/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexValue {
    syntax: FshSyntaxNode,
}

impl AstNode for RegexValue {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::RegexValue
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl RegexValue {
    /// Get the regex pattern (without the surrounding slashes)
    pub fn pattern(&self) -> Option<String> {
        token_of_kind(&self.syntax, FshSyntaxKind::Regex).map(|t| {
            let text = t.text();
            // Remove surrounding slashes if present
            if text.len() >= 2 && text.starts_with('/') && text.ends_with('/') {
                text[1..text.len() - 1].to_string()
            } else {
                text.to_string()
            }
        })
    }
}

/// Canonical value: canonical|version or Canonical(Type)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalValue {
    syntax: FshSyntaxNode,
}

impl AstNode for CanonicalValue {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CanonicalValue
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CanonicalValue {
    /// Get the canonical URL
    pub fn url(&self) -> Option<String> {
        // Look for Canonical token first
        if let Some(token) = token_of_kind(&self.syntax, FshSyntaxKind::Canonical) {
            let text = token.text();
            // If it contains |, split and take the first part
            if let Some(pipe_pos) = text.find('|') {
                Some(text[..pipe_pos].to_string())
            } else {
                Some(text.to_string())
            }
        } else {
            // Look for Ident token (Canonical keyword)
            token_of_kind(&self.syntax, FshSyntaxKind::Ident).map(|t| t.text().to_string())
        }
    }

    /// Get the version if present (after |)
    pub fn version(&self) -> Option<String> {
        // Look for version in Canonical token
        if let Some(token) = token_of_kind(&self.syntax, FshSyntaxKind::Canonical) {
            let text = token.text();
            if let Some(pipe_pos) = text.find('|') {
                Some(text[pipe_pos + 1..].to_string())
            } else {
                None
            }
        } else {
            // Look for separate version token (if lexer splits it)
            self.syntax
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.text().starts_with('|'))
                .map(|t| t.text()[1..].to_string())
        }
    }

    /// Get the type parameter if this is Canonical(Type) syntax
    pub fn type_param(&self) -> Option<String> {
        // Look for identifier between parentheses
        let mut in_parens = false;
        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.as_token() {
                match token.kind() {
                    FshSyntaxKind::LParen => in_parens = true,
                    FshSyntaxKind::RParen => in_parens = false,
                    FshSyntaxKind::Ident if in_parens => {
                        return Some(token.text().to_string());
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

/// Reference value: Reference(Type1 or Type2)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceValue {
    syntax: FshSyntaxNode,
}

impl AstNode for ReferenceValue {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::ReferenceValue
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl ReferenceValue {
    /// Get the reference types (split by 'or')
    pub fn types(&self) -> Vec<String> {
        let mut types = Vec::new();
        let mut in_parens = false;
        let mut current_type = String::new();

        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.as_token() {
                match token.kind() {
                    FshSyntaxKind::LParen => in_parens = true,
                    FshSyntaxKind::RParen => {
                        in_parens = false;
                        if !current_type.trim().is_empty() {
                            types.push(current_type.trim().to_string());
                            current_type.clear();
                        }
                    }
                    FshSyntaxKind::OrKw if in_parens => {
                        if !current_type.trim().is_empty() {
                            types.push(current_type.trim().to_string());
                            current_type.clear();
                        }
                    }
                    FshSyntaxKind::Ident if in_parens => {
                        if !current_type.is_empty() {
                            current_type.push(' ');
                        }
                        current_type.push_str(token.text());
                    }
                    _ => {}
                }
            }
        }

        types
    }
}

/// CodeableReference value: CodeableReference(Type)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeableReferenceValue {
    syntax: FshSyntaxNode,
}

impl AstNode for CodeableReferenceValue {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::CodeableReferenceValue
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl CodeableReferenceValue {
    /// Get the type parameter
    pub fn type_param(&self) -> Option<String> {
        // Look for identifier between parentheses
        let mut in_parens = false;
        for element in self.syntax.children_with_tokens() {
            if let Some(token) = element.as_token() {
                match token.kind() {
                    FshSyntaxKind::LParen => in_parens = true,
                    FshSyntaxKind::RParen => in_parens = false,
                    FshSyntaxKind::Ident if in_parens => {
                        return Some(token.text().to_string());
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

/// Name value: Name "Display String" or System#code "Display"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameValue {
    syntax: FshSyntaxNode,
}

impl AstNode for NameValue {
    fn can_cast(kind: FshSyntaxKind) -> bool {
        kind == FshSyntaxKind::NameValue
    }

    fn cast(node: FshSyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &FshSyntaxNode {
        &self.syntax
    }
}

impl NameValue {
    /// Get the name/identifier part
    pub fn name(&self) -> Option<String> {
        token_of_kind(&self.syntax, FshSyntaxKind::Ident).map(|t| t.text().to_string())
    }

    /// Get the code part if this is System#code format
    pub fn code(&self) -> Option<String> {
        token_of_kind(&self.syntax, FshSyntaxKind::Code).map(|t| t.text().to_string())
    }

    /// Get the display string if present
    pub fn display(&self) -> Option<String> {
        get_string_text(&self.syntax)
    }

    /// Check if this is a system#code format
    pub fn is_system_code(&self) -> bool {
        self.code().is_some()
    }
}