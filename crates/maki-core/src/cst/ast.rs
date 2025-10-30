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
//! let (cst, _) = parse_fsh("Profile: MyPatient\nParent: Patient");
//! let profile = Profile::cast(cst.first_child().unwrap()).unwrap();
//!
//! assert_eq!(profile.name().unwrap(), "MyPatient");
//! assert_eq!(profile.parent().unwrap().value(), "Patient");
//! ```

use super::{FshSyntaxKind, FshSyntaxNode, FshSyntaxToken};

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
}

impl Rule {
    pub fn cast(node: FshSyntaxNode) -> Option<Self> {
        match node.kind() {
            FshSyntaxKind::CardRule => CardRule::cast(node).map(Rule::Card),
            FshSyntaxKind::FlagRule => FlagRule::cast(node).map(Rule::Flag),
            FshSyntaxKind::ValuesetRule => ValueSetRule::cast(node).map(Rule::ValueSet),
            FshSyntaxKind::FixedValueRule => FixedValueRule::cast(node).map(Rule::FixedValue),
            FshSyntaxKind::PathRule => PathRule::cast(node).map(Rule::Path),
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

    pub fn cardinality(&self) -> Option<String> {
        // Find NUMBER..NUMBER or NUMBER..* pattern
        let text = self.syntax.text().to_string();
        // Simple extraction: look for pattern like "0..1" or "1..*"
        if let Some(pos) = text.find("..") {
            let start = text[..pos].split_whitespace().last()?;
            let end = text[pos + 2..].split_whitespace().next()?;
            Some(format!("{start}..{end}"))
        } else {
            None
        }
    }

    pub fn flags(&self) -> Vec<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| matches!(t.kind(), FshSyntaxKind::MsFlag | FshSyntaxKind::SuFlag))
            .map(|t| t.text().to_string())
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

    pub fn flags(&self) -> Vec<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| matches!(t.kind(), FshSyntaxKind::MsFlag | FshSyntaxKind::SuFlag))
            .map(|t| t.text().to_string())
            .collect()
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
        // Value can be string, number, identifier, or boolean
        get_string_text(&self.syntax)
            .or_else(|| get_ident_text(&self.syntax))
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

    /// Get path segments (split by '.')
    pub fn segments(&self) -> Vec<String> {
        self.as_string()
            .split('.')
            .map(|s| s.trim().to_string())
            .collect()
    }
}
