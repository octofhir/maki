/**
 * FSH Abstract Syntax Tree (AST) types
 *
 * Based on the official FSH ANTLR grammar and the Go reference implementation
 * from github.com/verily-src/fsh-lint
 */
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// Span represents a location in the source code
pub type Span = Range<usize>;

/// A value with its source location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

/// Root document containing all FSH entities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FSHDocument {
    pub aliases: Vec<Alias>,
    pub profiles: Vec<Profile>,
    pub extensions: Vec<Extension>,
    pub value_sets: Vec<ValueSet>,
    pub code_systems: Vec<CodeSystem>,
    pub instances: Vec<Instance>,
    pub invariants: Vec<Invariant>,
    pub mappings: Vec<Mapping>,
    pub logicals: Vec<Logical>,
    pub resources: Vec<Resource>,
    pub span: Span,
}

impl FSHDocument {
    pub fn new(span: Span) -> Self {
        Self {
            aliases: Vec::new(),
            profiles: Vec::new(),
            extensions: Vec::new(),
            value_sets: Vec::new(),
            code_systems: Vec::new(),
            instances: Vec::new(),
            invariants: Vec::new(),
            mappings: Vec::new(),
            logicals: Vec::new(),
            resources: Vec::new(),
            span,
        }
    }
}

/// Alias definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alias {
    pub name: Spanned<String>,
    pub value: Spanned<String>,
    pub span: Span,
}

/// Profile definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: Spanned<String>,
    pub parent: Option<Spanned<String>>,
    pub id: Option<Spanned<String>>,
    pub title: Option<Spanned<String>>,
    pub description: Option<Spanned<String>>,
    pub rules: Vec<SDRule>,
    pub span: Span,
}

/// Extension definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extension {
    pub name: Spanned<String>,
    pub parent: Option<Spanned<String>>,
    pub id: Option<Spanned<String>>,
    pub title: Option<Spanned<String>>,
    pub description: Option<Spanned<String>>,
    pub contexts: Vec<Spanned<String>>,
    pub rules: Vec<SDRule>,
    pub span: Span,
}

/// ValueSet definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueSet {
    pub name: Spanned<String>,
    pub parent: Option<Spanned<String>>,
    pub id: Option<Spanned<String>>,
    pub title: Option<Spanned<String>>,
    pub description: Option<Spanned<String>>,
    pub components: Vec<VSComponent>,
    pub rules: Vec<VSRule>,
    pub span: Span,
}

/// CodeSystem definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeSystem {
    pub name: Spanned<String>,
    pub id: Option<Spanned<String>>,
    pub title: Option<Spanned<String>>,
    pub description: Option<Spanned<String>>,
    pub concepts: Vec<Concept>,
    pub rules: Vec<CSRule>,
    pub span: Span,
}

/// Instance definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    pub name: Spanned<String>,
    pub instance_of: Option<Spanned<String>>,
    pub title: Option<Spanned<String>>,
    pub description: Option<Spanned<String>>,
    pub usage: Option<Spanned<String>>,
    pub rules: Vec<InstanceRule>,
    pub span: Span,
}

/// Invariant definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invariant {
    pub name: Spanned<String>,
    pub description: Option<Spanned<String>>,
    pub expression: Option<Spanned<String>>,
    pub xpath: Option<Spanned<String>>,
    pub severity: Option<Spanned<String>>,
    pub span: Span,
}

/// Mapping definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mapping {
    pub name: Spanned<String>,
    pub id: Option<Spanned<String>>,
    pub source: Option<Spanned<String>>,
    pub target: Option<Spanned<String>>,
    pub description: Option<Spanned<String>>,
    pub title: Option<Spanned<String>>,
    pub span: Span,
}

/// Logical model definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Logical {
    pub name: Spanned<String>,
    pub parent: Option<Spanned<String>>,
    pub id: Option<Spanned<String>>,
    pub title: Option<Spanned<String>>,
    pub description: Option<Spanned<String>>,
    pub characteristics: Vec<Spanned<String>>,
    pub rules: Vec<LRRule>,
    pub span: Span,
}

/// Resource definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub name: Spanned<String>,
    pub parent: Option<Spanned<String>>,
    pub id: Option<Spanned<String>>,
    pub title: Option<Spanned<String>>,
    pub description: Option<Spanned<String>>,
    pub rules: Vec<LRRule>,
    pub span: Span,
}

/// Structure Definition rules (for Profile and Extension)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SDRule {
    Card(CardRule),
    Flag(FlagRule),
    ValueSet(ValueSetRule),
    FixedValue(FixedValueRule),
    Contains(ContainsRule),
    Only(OnlyRule),
    Obeys(ObeysRule),
    CaretValue(CaretValueRule),
    Insert(InsertRule),
    Path(PathRule),
}

/// Logical/Resource rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LRRule {
    SD(SDRule),
    AddElement(AddElementRule),
}

/// Cardinality rule: * path 0..1
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardRule {
    pub path: Spanned<String>,
    pub cardinality: Spanned<Cardinality>,
    pub flags: Vec<Spanned<Flag>>,
    pub span: Span,
}

/// Flag rule: * path MS SU
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlagRule {
    pub paths: Vec<Spanned<String>>,
    pub flags: Vec<Spanned<Flag>>,
    pub span: Span,
}

/// ValueSet binding rule: * path from ValueSetName (required)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueSetRule {
    pub path: Spanned<String>,
    pub value_set: Spanned<String>,
    pub strength: Option<Spanned<BindingStrength>>,
    pub span: Span,
}

/// Fixed value assignment: * path = value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixedValueRule {
    pub path: Spanned<String>,
    pub value: Spanned<Value>,
    pub exactly: bool,
    pub span: Span,
}

/// Contains rule: * path contains item1 and item2
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainsRule {
    pub path: Spanned<String>,
    pub items: Vec<ContainsItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainsItem {
    pub name: Spanned<String>,
    pub named_as: Option<Spanned<String>>,
    pub cardinality: Option<Spanned<Cardinality>>,
    pub flags: Vec<Spanned<Flag>>,
    pub span: Span,
}

/// Only rule: * path only Type1 or Type2
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlyRule {
    pub path: Spanned<String>,
    pub types: Vec<Spanned<String>>,
    pub span: Span,
}

/// Obeys rule: * path obeys inv-1 and inv-2
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObeysRule {
    pub path: Option<Spanned<String>>,
    pub invariants: Vec<Spanned<String>>,
    pub span: Span,
}

/// Caret value rule: * path ^property = value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaretValueRule {
    pub path: Option<Spanned<String>>,
    pub caret_path: Spanned<String>,
    pub value: Spanned<Value>,
    pub span: Span,
}

/// Insert rule: * path insert RuleSetName
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertRule {
    pub path: Option<Spanned<String>>,
    pub rule_set: Spanned<String>,
    pub span: Span,
}

/// Path rule: * path
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathRule {
    pub path: Spanned<String>,
    pub span: Span,
}

/// Add element rule (for Logical/Resource)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddElementRule {
    pub path: Spanned<String>,
    pub cardinality: Spanned<Cardinality>,
    pub flags: Vec<Spanned<Flag>>,
    pub types: Vec<Spanned<String>>,
    pub short: Option<Spanned<String>>,
    pub definition: Option<Spanned<String>>,
    pub span: Span,
}

/// ValueSet component: * include/exclude ...
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VSComponent {
    pub include: bool, // true for include, false for exclude
    pub component_type: VSComponentType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VSComponentType {
    Concept(VSConceptComponent),
    Filter(VSFilterComponent),
}

/// ValueSet concept component: code from system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VSConceptComponent {
    pub code: Spanned<Code>,
    pub from_system: Option<Spanned<String>>,
    pub from_valueset: Vec<Spanned<String>>,
}

/// ValueSet filter component: codes from system where ...
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VSFilterComponent {
    pub from_system: Option<Spanned<String>>,
    pub from_valueset: Vec<Spanned<String>>,
    pub filters: Vec<VSFilter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VSFilter {
    pub property: Spanned<String>,
    pub operator: Spanned<String>,
    pub value: Option<Spanned<Value>>,
    pub span: Span,
}

/// ValueSet rules (other than components)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VSRule {
    CaretValue(CaretValueRule),
    CodeCaretValue(CodeCaretValueRule),
    Insert(InsertRule),
    CodeInsert(CodeInsertRule),
}

/// Code caret value rule: * code ^property = value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeCaretValueRule {
    pub codes: Vec<Spanned<Code>>,
    pub caret_path: Spanned<String>,
    pub value: Spanned<Value>,
    pub span: Span,
}

/// Code insert rule: * code insert RuleSet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeInsertRule {
    pub codes: Vec<Spanned<Code>>,
    pub rule_set: Spanned<String>,
    pub span: Span,
}

/// CodeSystem rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CSRule {
    Concept(Concept),
    CodeCaretValue(CodeCaretValueRule),
    CodeInsert(CodeInsertRule),
}

/// CodeSystem concept
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Concept {
    pub codes: Vec<Spanned<Code>>,
    pub display: Option<Spanned<String>>,
    pub definition: Option<Spanned<String>>,
    pub span: Span,
}

/// Instance rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstanceRule {
    FixedValue(FixedValueRule),
    Insert(InsertRule),
    Path(PathRule),
}

/// Cardinality: min..max
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cardinality {
    pub min: Option<u32>,
    pub max: CardinalityMax,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardinalityMax {
    Number(u32),
    Star, // *
}

/// Flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flag {
    MS,       // Must Support
    SU,       // Summary
    TU,       // Trial Use
    N,        // Normative
    D,        // Draft
    Modifier, // ?!
}

/// Binding strength
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingStrength {
    Required,
    Extensible,
    Preferred,
    Example,
    /// Unknown/invalid binding strength (for semantic validation)
    Unknown(String),
}

/// Code with optional system: system#code or #code
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Code {
    pub system: Option<String>,
    pub code: String,
    pub display: Option<String>,
}

/// FSH value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Code(Code),
    Quantity(Quantity),
    Reference(String),
    Canonical(String),
    DateTime(String),
    Time(String),
    Identifier(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub value: Option<f64>,
    pub unit: String,
    pub display: Option<String>,
}

impl std::fmt::Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Flag::MS => write!(f, "MS"),
            Flag::SU => write!(f, "SU"),
            Flag::TU => write!(f, "TU"),
            Flag::N => write!(f, "N"),
            Flag::D => write!(f, "D"),
            Flag::Modifier => write!(f, "?!"),
        }
    }
}

impl std::fmt::Display for BindingStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindingStrength::Required => write!(f, "required"),
            BindingStrength::Extensible => write!(f, "extensible"),
            BindingStrength::Preferred => write!(f, "preferred"),
            BindingStrength::Example => write!(f, "example"),
            BindingStrength::Unknown(s) => write!(f, "{}", s),
        }
    }
}
