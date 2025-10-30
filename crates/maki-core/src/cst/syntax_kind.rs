//! Syntax kind enumeration for FSH CST
//!
//! This module defines all possible node and token types in the FSH syntax tree.

use std::fmt;

/// Syntax kind for FSH language elements
///
/// This enum represents all possible types of nodes and tokens in the CSH CST.
/// It includes:
/// - Trivia (whitespace, comments)
/// - Keywords (Profile, Parent, Extension, etc.)
/// - Punctuation and operators
/// - Structural nodes (profiles, rules, etc.)
/// - Literals and identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum FshSyntaxKind {
    // ==================
    // Trivia (0-9)
    // ==================
    /// Whitespace (spaces, tabs)
    Whitespace = 0,
    /// Line comment starting with //
    CommentLine = 1,
    /// Block comment /* ... */
    CommentBlock = 2,
    /// Newline character
    Newline = 3,

    // ==================
    // Keywords (10-99)
    // ==================

    // Definition keywords
    /// "Profile" keyword
    ProfileKw = 10,
    /// "Extension" keyword
    ExtensionKw = 11,
    /// "ValueSet" keyword
    ValuesetKw = 12,
    /// "CodeSystem" keyword
    CodesystemKw = 13,
    /// "Instance" keyword
    InstanceKw = 14,
    /// "Invariant" keyword
    InvariantKw = 15,
    /// "Mapping" keyword
    MappingKw = 16,
    /// "Logical" keyword
    LogicalKw = 17,
    /// "Resource" keyword
    ResourceKw = 18,
    /// "Alias" keyword
    AliasKw = 19,
    /// "RuleSet" keyword
    RulesetKw = 20,

    // Metadata keywords
    /// "Parent" keyword
    ParentKw = 21,
    /// "Id" keyword
    IdKw = 22,
    /// "Title" keyword
    TitleKw = 23,
    /// "Description" keyword
    DescriptionKw = 24,
    /// "Expression" keyword (for invariants)
    ExpressionKw = 25,
    /// "XPath" keyword (for invariants)
    XpathKw = 26,
    /// "Severity" keyword (for invariants)
    SeverityKw = 27,
    /// "InstanceOf" keyword
    InstanceofKw = 28,
    /// "Usage" keyword
    UsageKw = 29,
    /// "Source" keyword (for mappings)
    SourceKw = 30,
    /// "Target" keyword (for mappings)
    TargetKw = 31,
    /// "Context" keyword (for extensions)
    ContextKw = 32,
    /// "Characteristics" keyword (for logical models)
    CharacteristicsKw = 33,

    // Rule keywords
    /// "from" keyword (binding)
    FromKw = 40,
    /// "only" keyword (type constraint)
    OnlyKw = 41,
    /// "obeys" keyword (invariant reference)
    ObeysKw = 42,
    /// "contains" keyword (slicing)
    ContainsKw = 43,
    /// "named" keyword (in contains)
    NamedKw = 44,
    /// "and" keyword (in contains/obeys)
    AndKw = 45,
    /// "or" keyword (in only)
    OrKw = 46,
    /// "insert" keyword (rule set)
    InsertKw = 47,
    /// "include" keyword (ValueSet)
    IncludeKw = 48,
    /// "exclude" keyword (ValueSet)
    ExcludeKw = 49,
    /// "codes" keyword (ValueSet filter)
    CodesKw = 50,
    /// "where" keyword (ValueSet filter)
    WhereKw = 51,
    /// "system" keyword (ValueSet)
    SystemKw = 52,
    /// "valueset" reference keyword (ValueSet)
    ValuesetRefKw = 53,
    /// "contentreference" keyword (AddCRElementRule)
    ContentreferenceKw = 54,

    // Binding strength
    /// "required" binding strength
    RequiredKw = 60,
    /// "extensible" binding strength
    ExtensibleKw = 61,
    /// "preferred" binding strength
    PreferredKw = 62,
    /// "example" binding strength
    ExampleKw = 63,

    // ==================
    // Flags (70-79)
    // ==================
    /// "MS" (Must Support) flag
    MsFlag = 70,
    /// "SU" (Summary) flag
    SuFlag = 71,
    /// "TU" (Trial Use) flag
    TuFlag = 72,
    /// "N" (Normative) flag
    NFlag = 73,
    /// "D" (Draft) flag
    DFlag = 74,
    /// "?!" (Modifier) flag
    ModifierFlag = 75,

    // ==================
    // Punctuation & Operators (100-149)
    // ==================
    /// Colon ":"
    Colon = 100,
    /// Asterisk "*" (rule prefix)
    Asterisk = 101,
    /// Equals "="
    Equals = 102,
    /// Plus "+" (soft indexing)
    Plus = 1020,
    /// Plus-equals "+=" (additive assignment)
    PlusEquals = 1021,
    /// Caret "^" (metadata)
    Caret = 103,
    /// Dot "." (path separator)
    Dot = 104,
    /// Hash "#" (code prefix)
    Hash = 105,
    /// Minus/hyphen "-"
    Minus = 114,
    /// Left parenthesis "("
    LParen = 106,
    /// Right parenthesis ")"
    RParen = 107,
    /// Left bracket "["
    LBracket = 108,
    /// Right bracket "]"
    RBracket = 109,
    /// Left brace "{"
    LBrace = 110,
    /// Right brace "}"
    RBrace = 111,
    /// Range separator ".."
    Range = 112,
    /// Comma ","
    Comma = 113,
    /// Greater than ">"
    Gt = 115,
    /// Less than "<"
    Lt = 116,
    /// Question mark "?"
    Question = 117,
    /// Exclamation "!"
    Exclamation = 118,
    /// Percent "%"
    Percent = 119,
    /// Single quote "'"
    SingleQuote = 120,
    /// Backslash "\"
    Backslash = 121,
    /// Forward slash "/"
    Slash = 122,
    /// Arrow "->" (for mapping)
    Arrow = 123,

    // ==================
    // Literals & Identifiers (150-199)
    // ==================
    /// Identifier (unquoted name)
    Ident = 150,
    /// String literal "..."
    String = 151,
    /// Integer literal
    Integer = 152,
    /// Decimal literal
    Decimal = 153,
    /// Boolean true
    True = 154,
    /// Boolean false
    False = 155,
    /// Code (system#code or #code)
    Code = 156,
    /// URL/canonical reference
    Url = 157,
    /// Regex pattern /pattern/
    Regex = 158,
    /// UCUM unit 'unit'
    Unit = 159,

    // ==================
    // Structure Nodes (200-399)
    // ==================

    // Root and document
    /// Root node of the syntax tree
    Root = 200,
    /// Complete FSH document
    Document = 201,

    // Definitions
    /// Alias definition
    Alias = 210,
    /// Profile definition
    Profile = 211,
    /// Extension definition
    Extension = 212,
    /// ValueSet definition
    ValueSet = 213,
    /// CodeSystem definition
    CodeSystem = 214,
    /// Instance definition
    Instance = 215,
    /// Invariant definition
    Invariant = 216,
    /// Mapping definition
    Mapping = 217,
    /// Logical model definition
    Logical = 218,
    /// Resource definition
    Resource = 219,
    /// RuleSet definition
    RuleSet = 220,

    // Metadata clauses
    /// Parent clause
    ParentClause = 230,
    /// Id clause
    IdClause = 231,
    /// Title clause
    TitleClause = 232,
    /// Description clause
    DescriptionClause = 233,
    /// Expression clause
    ExpressionClause = 234,
    /// XPath clause
    XpathClause = 235,
    /// Severity clause
    SeverityClause = 236,
    /// InstanceOf clause
    InstanceofClause = 237,
    /// Usage clause
    UsageClause = 238,
    /// Source clause
    SourceClause = 239,
    /// Target clause
    TargetClause = 240,

    // Rules (250-299)
    /// Cardinality rule: * path 0..1
    CardRule = 250,
    /// Flag rule: * path MS SU
    FlagRule = 251,
    /// ValueSet binding rule: * path from ValueSet
    ValuesetRule = 252,
    /// Fixed value rule: * path = value
    FixedValueRule = 253,
    /// Contains rule: * path contains Item
    ContainsRule = 254,
    /// Only rule: * path only Type
    OnlyRule = 255,
    /// Obeys rule: * path obeys Invariant
    ObeysRule = 256,
    /// Caret value rule: * ^property = value
    CaretValueRule = 257,
    /// Insert rule: * insert RuleSet
    InsertRule = 258,
    /// Path rule: * path
    PathRule = 259,
    /// Add element rule (for Logical/Resource)
    AddElementRule = 260,
    /// Add content reference element rule (for Logical/Resource)
    AddCRElementRule = 262,
    /// Mapping rule: * path -> "target"
    MappingRule = 261,

    // ValueSet components (300-319)
    /// ValueSet include/exclude component
    VsComponent = 300,
    /// ValueSet concept component
    VsConceptComponent = 301,
    /// ValueSet filter component
    VsFilterComponent = 302,
    /// ValueSet filter
    VsFilter = 303,
    /// Code caret value rule
    CodeCaretValueRule = 304,
    /// Code insert rule
    CodeInsertRule = 305,
    /// ValueSet component "from" clause
    VsComponentFrom = 306,
    /// "from system X" part
    VsFromSystem = 307,
    /// "from valueset Y" part
    VsFromValueset = 308,
    /// "where" filter list
    VsFilterList = 309,
    /// Single filter definition
    VsFilterDefinition = 310,
    /// Filter operator (is-a, =, etc.)
    VsFilterOperator = 311,
    /// Filter value
    VsFilterValue = 312,

    // CodeSystem (320-329)
    /// CodeSystem concept
    Concept = 320,

    // Other constructs (330-349)
    /// Contains item (in contains rule)
    ContainsItem = 330,
    /// Cardinality specification (min..max)
    Cardinality = 331,
    /// Path expression (foo.bar[x].baz)
    Path = 332,
    /// Code reference (#code or system#code)
    CodeRef = 333,
    /// Type reference
    TypeRef = 334,
    /// Quantity value
    Quantity = 335,
    /// Ratio value (numerator:denominator)
    Ratio = 339,
    /// Parameter list in parameterized RuleSet
    ParameterList = 336,
    /// Individual parameter in RuleSet
    Parameter = 337,
    /// Insert rule arguments
    InsertRuleArgs = 338,

    // ==================
    // Special tokens (400+)
    // ==================
    /// Error token (for recovery)
    Error = 400,
    /// End of file
    Eof = 401,
    /// Unknown/invalid token
    Unknown = 402,

    // ==================
    // Compound expressions (500+)
    // ==================
    /// List of flags (MS SU TU)
    FlagList = 500,
    /// List of types (Type1 or Type2)
    TypeList = 501,
    /// List of invariants (inv1 and inv2)
    InvariantList = 502,
    /// List of contains items
    ContainsItemList = 503,

    // Tombstone marker (for syntax tree editing)
    /// Tombstone marker for deleted nodes
    Tombstone = 999,
}

impl FshSyntaxKind {
    /// Check if this is a trivia kind (whitespace, comments, newlines)
    pub const fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Whitespace | Self::CommentLine | Self::CommentBlock | Self::Newline
        )
    }

    /// Check if this is a keyword
    pub const fn is_keyword(self) -> bool {
        (self as u16) >= 10 && (self as u16) < 100
    }

    /// Check if this is punctuation
    pub const fn is_punct(self) -> bool {
        (self as u16) >= 100 && (self as u16) < 150
    }

    /// Check if this is a literal
    pub const fn is_literal(self) -> bool {
        matches!(
            self,
            Self::String
                | Self::Integer
                | Self::Decimal
                | Self::True
                | Self::False
                | Self::Code
                | Self::Url
        )
    }

    /// Check if this is a structural node
    pub const fn is_node(self) -> bool {
        (self as u16) >= 200 && (self as u16) < 400
    }

    /// Get the text representation of keyword tokens
    pub const fn keyword_text(self) -> Option<&'static str> {
        match self {
            Self::ProfileKw => Some("Profile"),
            Self::ExtensionKw => Some("Extension"),
            Self::ValuesetKw => Some("ValueSet"),
            Self::CodesystemKw => Some("CodeSystem"),
            Self::InstanceKw => Some("Instance"),
            Self::InvariantKw => Some("Invariant"),
            Self::MappingKw => Some("Mapping"),
            Self::LogicalKw => Some("Logical"),
            Self::ResourceKw => Some("Resource"),
            Self::AliasKw => Some("Alias"),
            Self::RulesetKw => Some("RuleSet"),
            Self::ParentKw => Some("Parent"),
            Self::IdKw => Some("Id"),
            Self::TitleKw => Some("Title"),
            Self::DescriptionKw => Some("Description"),
            Self::ExpressionKw => Some("Expression"),
            Self::XpathKw => Some("XPath"),
            Self::SeverityKw => Some("Severity"),
            Self::InstanceofKw => Some("InstanceOf"),
            Self::UsageKw => Some("Usage"),
            Self::SourceKw => Some("Source"),
            Self::TargetKw => Some("Target"),
            Self::FromKw => Some("from"),
            Self::OnlyKw => Some("only"),
            Self::ObeysKw => Some("obeys"),
            Self::ContainsKw => Some("contains"),
            Self::NamedKw => Some("named"),
            Self::AndKw => Some("and"),
            Self::OrKw => Some("or"),
            Self::InsertKw => Some("insert"),
            Self::IncludeKw => Some("include"),
            Self::ExcludeKw => Some("exclude"),
            Self::CodesKw => Some("codes"),
            Self::WhereKw => Some("where"),
            Self::RequiredKw => Some("required"),
            Self::ExtensibleKw => Some("extensible"),
            Self::PreferredKw => Some("preferred"),
            Self::ExampleKw => Some("example"),
            Self::True => Some("true"),
            Self::False => Some("false"),
            _ => None,
        }
    }
}

impl fmt::Display for FshSyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<FshSyntaxKind> for rowan::SyntaxKind {
    fn from(kind: FshSyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trivia_classification() {
        assert!(FshSyntaxKind::Whitespace.is_trivia());
        assert!(FshSyntaxKind::CommentLine.is_trivia());
        assert!(FshSyntaxKind::CommentBlock.is_trivia());
        assert!(!FshSyntaxKind::ProfileKw.is_trivia());
    }

    #[test]
    fn test_keyword_classification() {
        assert!(FshSyntaxKind::ProfileKw.is_keyword());
        assert!(FshSyntaxKind::ParentKw.is_keyword());
        assert!(!FshSyntaxKind::Ident.is_keyword());
        assert!(!FshSyntaxKind::Whitespace.is_keyword());
    }

    #[test]
    fn test_keyword_text() {
        assert_eq!(FshSyntaxKind::ProfileKw.keyword_text(), Some("Profile"));
        assert_eq!(FshSyntaxKind::FromKw.keyword_text(), Some("from"));
        assert_eq!(FshSyntaxKind::Ident.keyword_text(), None);
    }

    #[test]
    fn test_node_classification() {
        assert!(FshSyntaxKind::Profile.is_node());
        assert!(FshSyntaxKind::CardRule.is_node());
        assert!(!FshSyntaxKind::Ident.is_node());
    }
}
