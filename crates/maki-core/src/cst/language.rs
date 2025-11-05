//! Rowan language implementation for FSH
//!
//! This module implements the `rowan::Language` trait for FSH, which connects
//! our FshSyntaxKind enum to Rowan's generic CST infrastructure.

use rowan::Language;

use super::FshSyntaxKind;

/// Language implementation for FHIR Shorthand
///
/// This is a zero-sized type that implements `rowan::Language` to provide
/// the connection between our syntax kinds and Rowan's generic tree types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FshLanguage;

impl Language for FshLanguage {
    type Kind = FshSyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        // Safety: We control the SyntaxKind values and ensure they map to valid FshSyntaxKind
        // The match below validates all expected values
        match raw.0 {
            // Trivia
            0 => FshSyntaxKind::Whitespace,
            1 => FshSyntaxKind::CommentLine,
            2 => FshSyntaxKind::CommentBlock,
            3 => FshSyntaxKind::Newline,

            // Keywords (10-99)
            10 => FshSyntaxKind::ProfileKw,
            11 => FshSyntaxKind::ExtensionKw,
            12 => FshSyntaxKind::ValuesetKw,
            13 => FshSyntaxKind::CodesystemKw,
            14 => FshSyntaxKind::InstanceKw,
            15 => FshSyntaxKind::InvariantKw,
            16 => FshSyntaxKind::MappingKw,
            17 => FshSyntaxKind::LogicalKw,
            18 => FshSyntaxKind::ResourceKw,
            19 => FshSyntaxKind::AliasKw,
            20 => FshSyntaxKind::RulesetKw,
            21 => FshSyntaxKind::ParentKw,
            22 => FshSyntaxKind::IdKw,
            23 => FshSyntaxKind::TitleKw,
            24 => FshSyntaxKind::DescriptionKw,
            25 => FshSyntaxKind::ExpressionKw,
            26 => FshSyntaxKind::XpathKw,
            27 => FshSyntaxKind::SeverityKw,
            28 => FshSyntaxKind::InstanceofKw,
            29 => FshSyntaxKind::UsageKw,
            30 => FshSyntaxKind::SourceKw,
            31 => FshSyntaxKind::TargetKw,
            32 => FshSyntaxKind::ContextKw,
            33 => FshSyntaxKind::CharacteristicsKw,
            40 => FshSyntaxKind::FromKw,
            41 => FshSyntaxKind::OnlyKw,
            42 => FshSyntaxKind::ObeysKw,
            43 => FshSyntaxKind::ContainsKw,
            44 => FshSyntaxKind::NamedKw,
            45 => FshSyntaxKind::AndKw,
            46 => FshSyntaxKind::OrKw,
            47 => FshSyntaxKind::InsertKw,
            48 => FshSyntaxKind::IncludeKw,
            49 => FshSyntaxKind::ExcludeKw,
            50 => FshSyntaxKind::CodesKw,
            51 => FshSyntaxKind::WhereKw,
            52 => FshSyntaxKind::SystemKw,
            53 => FshSyntaxKind::ValuesetRefKw,
            54 => FshSyntaxKind::ContentreferenceKw,
            60 => FshSyntaxKind::RequiredKw,
            61 => FshSyntaxKind::ExtensibleKw,
            62 => FshSyntaxKind::PreferredKw,
            63 => FshSyntaxKind::ExampleKw,

            // Flags (70-79)
            70 => FshSyntaxKind::MsFlag,
            71 => FshSyntaxKind::SuFlag,
            72 => FshSyntaxKind::TuFlag,
            73 => FshSyntaxKind::NFlag,
            74 => FshSyntaxKind::DFlag,
            75 => FshSyntaxKind::ModifierFlag,

            // Special high numbers (1020+)
            1020 => FshSyntaxKind::Plus,
            1021 => FshSyntaxKind::PlusEquals,

            // Punctuation (100-149)
            100 => FshSyntaxKind::Colon,
            101 => FshSyntaxKind::Asterisk,
            102 => FshSyntaxKind::Equals,
            103 => FshSyntaxKind::Caret,
            104 => FshSyntaxKind::Dot,
            105 => FshSyntaxKind::Hash,
            106 => FshSyntaxKind::LParen,
            107 => FshSyntaxKind::RParen,
            108 => FshSyntaxKind::LBracket,
            109 => FshSyntaxKind::RBracket,
            110 => FshSyntaxKind::LBrace,
            111 => FshSyntaxKind::RBrace,
            112 => FshSyntaxKind::Range,
            113 => FshSyntaxKind::Comma,
            114 => FshSyntaxKind::Minus,
            115 => FshSyntaxKind::Gt,
            116 => FshSyntaxKind::Lt,
            117 => FshSyntaxKind::Question,
            118 => FshSyntaxKind::Exclamation,
            119 => FshSyntaxKind::Percent,
            120 => FshSyntaxKind::SingleQuote,
            121 => FshSyntaxKind::Backslash,
            122 => FshSyntaxKind::Slash,
            123 => FshSyntaxKind::Arrow,

            // Literals & Identifiers (150-199)
            150 => FshSyntaxKind::Ident,
            151 => FshSyntaxKind::String,
            152 => FshSyntaxKind::Integer,
            153 => FshSyntaxKind::Decimal,
            154 => FshSyntaxKind::True,
            155 => FshSyntaxKind::False,
            156 => FshSyntaxKind::Code,
            157 => FshSyntaxKind::Url,
            158 => FshSyntaxKind::Regex,
            159 => FshSyntaxKind::Unit,
            160 => FshSyntaxKind::Canonical,
            161 => FshSyntaxKind::Reference,
            162 => FshSyntaxKind::CodeableReference,
            163 => FshSyntaxKind::BracketedParamToken,
            164 => FshSyntaxKind::PlainParamToken,
            165 => FshSyntaxKind::DateTime,
            166 => FshSyntaxKind::Time,

            // Structure nodes (200-399)
            200 => FshSyntaxKind::Root,
            201 => FshSyntaxKind::Document,
            210 => FshSyntaxKind::Alias,
            211 => FshSyntaxKind::Profile,
            212 => FshSyntaxKind::Extension,
            213 => FshSyntaxKind::ValueSet,
            214 => FshSyntaxKind::CodeSystem,
            215 => FshSyntaxKind::Instance,
            216 => FshSyntaxKind::Invariant,
            217 => FshSyntaxKind::Mapping,
            218 => FshSyntaxKind::Logical,
            219 => FshSyntaxKind::Resource,
            220 => FshSyntaxKind::RuleSet,
            230 => FshSyntaxKind::ParentClause,
            231 => FshSyntaxKind::IdClause,
            232 => FshSyntaxKind::TitleClause,
            233 => FshSyntaxKind::DescriptionClause,
            234 => FshSyntaxKind::ExpressionClause,
            235 => FshSyntaxKind::XpathClause,
            236 => FshSyntaxKind::SeverityClause,
            237 => FshSyntaxKind::InstanceofClause,
            238 => FshSyntaxKind::UsageClause,
            239 => FshSyntaxKind::SourceClause,
            240 => FshSyntaxKind::TargetClause,
            250 => FshSyntaxKind::CardRule,
            251 => FshSyntaxKind::FlagRule,
            252 => FshSyntaxKind::ValuesetRule,
            253 => FshSyntaxKind::FixedValueRule,
            254 => FshSyntaxKind::ContainsRule,
            255 => FshSyntaxKind::OnlyRule,
            256 => FshSyntaxKind::ObeysRule,
            257 => FshSyntaxKind::CaretValueRule,
            258 => FshSyntaxKind::InsertRule,
            259 => FshSyntaxKind::PathRule,
            260 => FshSyntaxKind::AddElementRule,
            261 => FshSyntaxKind::MappingRule,
            262 => FshSyntaxKind::AddCRElementRule,
            300 => FshSyntaxKind::VsComponent,
            301 => FshSyntaxKind::VsConceptComponent,
            302 => FshSyntaxKind::VsFilterComponent,
            303 => FshSyntaxKind::VsFilter,
            304 => FshSyntaxKind::CodeCaretValueRule,
            305 => FshSyntaxKind::CodeInsertRule,
            306 => FshSyntaxKind::VsComponentFrom,
            307 => FshSyntaxKind::VsFromSystem,
            308 => FshSyntaxKind::VsFromValueset,
            309 => FshSyntaxKind::VsFilterList,
            310 => FshSyntaxKind::VsFilterDefinition,
            311 => FshSyntaxKind::VsFilterOperator,
            312 => FshSyntaxKind::VsFilterValue,
            320 => FshSyntaxKind::Concept,
            330 => FshSyntaxKind::ContainsItem,
            331 => FshSyntaxKind::Cardinality,
            332 => FshSyntaxKind::Path,
            333 => FshSyntaxKind::CodeRef,
            334 => FshSyntaxKind::TypeRef,
            335 => FshSyntaxKind::Quantity,
            336 => FshSyntaxKind::ParameterList,
            337 => FshSyntaxKind::Parameter,
            338 => FshSyntaxKind::InsertRuleArgs,
            339 => FshSyntaxKind::Ratio,
            340 => FshSyntaxKind::PathSegment,

            // Value nodes (380-389)
            380 => FshSyntaxKind::RegexValue,
            381 => FshSyntaxKind::CanonicalValue,
            382 => FshSyntaxKind::ReferenceValue,
            383 => FshSyntaxKind::CodeableReferenceValue,
            384 => FshSyntaxKind::NameValue,

            // Special tokens (400+)
            400 => FshSyntaxKind::Error,
            401 => FshSyntaxKind::Eof,
            402 => FshSyntaxKind::Unknown,

            // Compound expressions (500+)
            500 => FshSyntaxKind::FlagList,
            501 => FshSyntaxKind::TypeList,
            502 => FshSyntaxKind::InvariantList,
            503 => FshSyntaxKind::ContainsItemList,

            // Tombstone
            999 => FshSyntaxKind::Tombstone,

            // Unknown value - return ERROR for unknown kinds
            _ => {
                eprintln!("Warning: Unknown syntax kind: {}", raw.0);
                FshSyntaxKind::Unknown
            }
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_roundtrip() {
        // Test that we can convert back and forth without loss
        let kinds = [
            FshSyntaxKind::Whitespace,
            FshSyntaxKind::ProfileKw,
            FshSyntaxKind::Ident,
            FshSyntaxKind::Colon,
            FshSyntaxKind::Profile,
            FshSyntaxKind::CardRule,
        ];

        for &kind in &kinds {
            let raw = FshLanguage::kind_to_raw(kind);
            let back = FshLanguage::kind_from_raw(raw);
            assert_eq!(kind, back, "Roundtrip failed for {kind:?}");
        }
    }

    #[test]
    fn test_kind_values() {
        // Verify specific kind values match our expectations
        assert_eq!(FshLanguage::kind_to_raw(FshSyntaxKind::Whitespace).0, 0);
        assert_eq!(FshLanguage::kind_to_raw(FshSyntaxKind::ProfileKw).0, 10);
        assert_eq!(FshLanguage::kind_to_raw(FshSyntaxKind::Colon).0, 100);
        assert_eq!(FshLanguage::kind_to_raw(FshSyntaxKind::Root).0, 200);
    }
}
