use chumsky::extra;
use chumsky::input::Stream;
use chumsky::prelude::*;
use chumsky::recovery::skip_then_retry_until;

use crate::ast::{
    Alias, BindingStrength, CardRule, Cardinality, CardinalityMax, CaretValueRule, Code,
    CodeCaretValueRule, CodeInsertRule, CodeSystem, Concept, ContainsItem, ContainsRule, CSRule,
    Extension, FixedValueRule, Flag, FlagRule, FSHDocument, InsertRule, Instance, InstanceRule,
    Invariant, LRRule, Logical, Mapping, ObeysRule, OnlyRule, PathRule, Profile, Resource, SDRule,
    Spanned, Value, ValueSet, ValueSetRule, VSComponent, VSComponentType, VSConceptComponent,
    VSFilter, VSFilterComponent, VSRule,
};
use crate::lexer::{LexSpan, Token};

use chumsky::input::ValueInput;

type ParserExtra<'tokens> = extra::Err<Rich<'tokens, Token>>;

#[derive(Debug, Clone)]
enum Entity {
    Alias(Alias),
    Profile(Profile),
    Extension(Extension),
    ValueSet(ValueSet),
    CodeSystem(CodeSystem),
    Instance(Instance),
    Invariant(Invariant),
    Mapping(Mapping),
    Logical(Logical),
    Resource(Resource),
}

#[derive(Debug, Clone)]
enum ValueSetLine {
    Component(VSComponent),
    Rule(VSRule),
}

fn entity<'src, I>() -> impl Parser<'src, I, Entity, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        alias_parser().map(Entity::Alias),
        profile_parser().map(Entity::Profile),
        extension_parser().map(Entity::Extension),
        value_set_parser().map(Entity::ValueSet),
        code_system_parser().map(Entity::CodeSystem),
        instance_parser().map(Entity::Instance),
        invariant_parser().map(Entity::Invariant),
        mapping_parser().map(Entity::Mapping),
        logical_parser().map(Entity::Logical),
        resource_parser().map(Entity::Resource),
    ))
    .recover_with(skip_then_retry_until(any().ignored(), end()))
}

pub fn fsh_document_parser<'src, I>() -> impl Parser<'src, I, FSHDocument, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    entity()
        .repeated()
        .collect::<Vec<_>>()
        .map_with(|entities, ext| {
            let span = to_range(ext.span());
            let mut document = FSHDocument::new(span.clone());

            for entity in entities {
                match entity {
                    Entity::Alias(a) => document.aliases.push(a),
                    Entity::Profile(p) => document.profiles.push(p),
                    Entity::Extension(e) => document.extensions.push(e),
                    Entity::ValueSet(vs) => document.value_sets.push(vs),
                    Entity::CodeSystem(cs) => document.code_systems.push(cs),
                    Entity::Instance(i) => document.instances.push(i),
                    Entity::Invariant(inv) => document.invariants.push(inv),
                    Entity::Mapping(m) => document.mappings.push(m),
                    Entity::Logical(l) => document.logicals.push(l),
                    Entity::Resource(r) => document.resources.push(r),
                }
            }

            document
        })
}

pub fn parse_tokens<'tokens>(
    tokens: &'tokens [(Token, LexSpan)],
    eof: LexSpan,
) -> (Option<FSHDocument>, Vec<Rich<'tokens, Token>>) {
    let parser = fsh_document_parser();
    // Create a stream and use .map() to split (Token, Span) tuples so parsers see just Token
    let stream = Stream::from_iter(tokens.iter().cloned())
        .map(eof, |(t, s)| (t, s));
    let (doc, errors) = parser.parse(stream).into_output_errors();
    (doc, errors)
}

fn value_set_parser<'src, I>() -> impl Parser<'src, I, ValueSet, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::ValueSet)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(metadata().repeated().collect::<Vec<_>>())
        .then(value_set_line().repeated().collect::<Vec<_>>())
        .map_with(|((name, metadata), lines), span| {
            let span = to_range(span.span());
            let mut value_set = ValueSet {
                name,
                parent: None,
                id: None,
                title: None,
                description: None,
                components: Vec::new(),
                rules: Vec::new(),
                span,
            };

            for meta in metadata {
                match meta {
                    Metadata::Parent(parent) => value_set.parent = Some(parent),
                    Metadata::Id(id) => value_set.id = Some(id),
                    Metadata::Title(title) => value_set.title = Some(title),
                    Metadata::Description(desc) => value_set.description = Some(desc),
                }
            }

            for line in lines {
                match line {
                    ValueSetLine::Component(component) => value_set.components.push(component),
                    ValueSetLine::Rule(rule) => value_set.rules.push(rule),
                }
            }

            value_set
        })
}

fn value_set_line<'src, I>() -> impl Parser<'src, I, ValueSetLine, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        vs_component().map(ValueSetLine::Component),
        vs_rule_parser().map(ValueSetLine::Rule),
    ))
}

fn vs_rule_parser<'src, I>() -> impl Parser<'src, I, VSRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        caret_value_rule().map(VSRule::CaretValue),
        code_caret_value_rule().map(VSRule::CodeCaretValue),
        insert_rule().map(VSRule::Insert),
        code_insert_rule().map(VSRule::CodeInsert),
    ))
}

// Parse caret path with optional array operators [+] or [=] and continuation
// Supports: context, context[+], context[+].type, context[+].type.system
fn caret_path<'src, I>() -> impl Parser<'src, I, Spanned<String>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    string_like()
        .then(
            just(Token::LBracket)
                .ignore_then(choice((
                    just(Token::Plus).to("+"),
                    just(Token::Equal).to("="),
                )))
                .then_ignore(just(Token::RBracket))
                .or_not()
        )
        .then(
            // Optional continuation with .field after array operator
            just(Token::Dot)
                .ignore_then(string_like())
                .repeated()
                .collect::<Vec<_>>()
        )
        .map_with(|((base, array_op), continuations), ext| {
            let mut path = base.value.clone();
            if let Some(op) = array_op {
                path.push('[');
                path.push_str(op);
                path.push(']');
            }
            for cont in continuations {
                path.push('.');
                path.push_str(&cont.value);
            }
            Spanned {
                value: path,
                span: to_range(ext.span()),
            }
        })
}

fn caret_value_rule<'src, I>() -> impl Parser<'src, I, CaretValueRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    let path = string_like().map(Some).then_ignore(just(Token::Caret));
    let no_path = just(Token::Caret).to(None::<Spanned<String>>);

    just(Token::Star)
        .ignore_then(path.or(no_path))
        .then(caret_path())
        .then_ignore(just(Token::Equal))
        .then(value_literal())
        .map_with(|((path, caret_path), value), span| CaretValueRule {
            path,
            caret_path,
            value,
            span: to_range(span.span()),
        })
}

fn code_caret_value_rule<'src, I>() -> impl Parser<'src, I, CodeCaretValueRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(code_literal().repeated().at_least(1).collect::<Vec<_>>())
        .then_ignore(just(Token::Caret))
        .then(caret_path())
        .then_ignore(just(Token::Equal))
        .then(value_literal())
        .map_with(|((codes, caret_path), value), span| CodeCaretValueRule {
            codes,
            caret_path,
            value,
            span: to_range(span.span()),
        })
}

fn insert_rule<'src, I>() -> impl Parser<'src, I, InsertRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    let path = string_like().map(Some).then_ignore(just(Token::Insert));
    let no_path = just(Token::Insert).to(None::<Spanned<String>>);

    just(Token::Star)
        .ignore_then(path.or(no_path))
        .then(string_like())
        .map_with(|(path, rule_set), span| InsertRule {
            path,
            rule_set,
            span: to_range(span.span()),
        })
}

fn code_insert_rule<'src, I>() -> impl Parser<'src, I, CodeInsertRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(code_literal().repeated().at_least(1).collect::<Vec<_>>())
        .then_ignore(just(Token::Insert))
        .then(string_like())
        .map_with(|(codes, rule_set), span| CodeInsertRule {
            codes,
            rule_set,
            span: to_range(span.span()),
        })
}

#[derive(Debug, Clone)]
enum Metadata {
    Parent(Spanned<String>),
    Id(Spanned<String>),
    Title(Spanned<String>),
    Description(Spanned<String>),
}

fn metadata<'src, I>() -> impl Parser<'src, I, Metadata, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        just(Token::Parent)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(Metadata::Parent),
        just(Token::Id)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(Metadata::Id),
        just(Token::Title)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(Metadata::Title),
        just(Token::Description)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(Metadata::Description),
    ))
}

fn vs_component<'src, I>() -> impl Parser<'src, I, VSComponent, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(
            choice((
                just(Token::Include).to(true),
                just(Token::Exclude).to(false),
            ))
            .or_not(),
        )
        .then(choice((vs_codes_component(), vs_code_component())))
        .map_with(|(include, component_type), span| VSComponent {
            include: include.unwrap_or(true),
            component_type,
            span: to_range(span.span()),
        })
}

fn vs_codes_component<'src, I>() -> impl Parser<'src, I, VSComponentType, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Codes)
        .ignore_then(just(Token::From))
        .ignore_then(vs_component_source())
        .then(vs_filters().or_not())
        .map(|((from_system, from_valueset), filters)| {
            VSComponentType::Filter(VSFilterComponent {
                from_system,
                from_valueset,
                filters: filters.unwrap_or_default(),
            })
        })
}

fn vs_code_component<'src, I>() -> impl Parser<'src, I, VSComponentType, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    code_literal()
        .then_ignore(
            // Optional display string after code (e.g., #12345 "Display text")
            string_like().or_not()
        )
        .map(|code| {
            VSComponentType::Concept(VSConceptComponent {
                code,
                from_system: None,
                from_valueset: Vec::new(),
            })
        })
}

fn vs_component_source<'src, I>() -> impl Parser<
    'src,
    I,
    (Option<Spanned<String>>, Vec<Spanned<String>>),
    ParserExtra<'src>,
> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    let system = just(Token::System).ignore_then(string_like()).or_not();

    let first_valueset = just(Token::ValueSetRef).ignore_then(string_like()).or_not();

    let additional_valuesets = just(Token::And)
        .ignore_then(just(Token::ValueSetRef).ignore_then(string_like()))
        .repeated()
        .collect::<Vec<_>>();

    system
        .then(first_valueset)
        .then(additional_valuesets)
        .map(|((system, first), mut rest)| {
            if let Some(first_vs) = first {
                rest.insert(0, first_vs);
            }
            (system, rest)
        })
}

fn vs_filters<'src, I>() -> impl Parser<'src, I, Vec<VSFilter>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Where).ignore_then(
        vs_filter()
            .separated_by(just(Token::And))
            .collect::<Vec<_>>(),
    )
}

fn vs_filter<'src, I>() -> impl Parser<'src, I, VSFilter, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    string_like()
        .then(string_like())
        .then(value_literal().or_not())
        .then_ignore(
            // Optional display string after the value (e.g., #8310-5 "Body temperature")
            string_like().or_not()
        )
        .map_with(|((property, operator), value), span| VSFilter {
            property,
            operator,
            value,
            span: to_range(span.span()),
        })
}

fn code_literal<'src, I>() -> impl Parser<'src, I, Spanned<Code>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    select! { Token::Code(system, code) => (system, code) }.map_with(|(system, code), span| {
        Spanned {
            value: Code {
                system: if system.is_empty() {
                    None
                } else {
                    Some(system)
                },
                code,
                display: None,
            },
            span: to_range(span.span()),
        }
    })
}

fn value_literal<'src, I>() -> impl Parser<'src, I, Spanned<Value>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    let string_value = select! { Token::String(value) => value }
        .or(select! { Token::MultilineString(value) => value })
        .map(Value::String);

    let boolean = select! {
        Token::True => Value::Boolean(true),
        Token::False => Value::Boolean(false),
    };

    let number = select! { Token::Number(value) => value }
        .map(|text: String| text.parse::<f64>().unwrap_or_default())
        .map(Value::Number);

    let code = select! { Token::Code(system, code) => (system, code) }.map(|(system, code)| {
        Value::Code(Code {
            system: if system.is_empty() {
                None
            } else {
                Some(system)
            },
            code,
            display: None,
        })
    });

    let identifier = select! { Token::Ident(value) => Value::Identifier(value) };

    choice((string_value, boolean, number, code, identifier)).map_with(|value, ext| Spanned {
        value,
        span: to_range(ext.span()),
    })
}

fn string_like<'src, I>() -> impl Parser<'src, I, Spanned<String>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        select! { Token::Ident(value) => value },
        select! { Token::String(value) => value },
        select! { Token::MultilineString(value) => value },
    ))
    .map_with(|value, ext| Spanned {
        value,
        span: to_range(ext.span()),
    })
}

fn name<'src, I>() -> impl Parser<'src, I, Spanned<String>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    string_like()
}

fn to_range(span: LexSpan) -> std::ops::Range<usize> {
    span.start()..span.end()
}

// ============================================================================
// Alias Parser
// ============================================================================

fn alias_parser<'src, I>() -> impl Parser<'src, I, Alias, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Alias)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then_ignore(just(Token::Equal))
        .then(string_like())
        .map_with(|(name, value), span| Alias {
            name,
            value,
            span: to_range(span.span()),
        })
}

// ============================================================================
// Profile Parser
// ============================================================================

fn profile_parser<'src, I>() -> impl Parser<'src, I, Profile, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Profile)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(profile_line().repeated().collect::<Vec<_>>())
        .map_with(|(name, lines), span| {
            let mut profile = Profile {
                name,
                parent: None,
                id: None,
                title: None,
                description: None,
                rules: Vec::new(),
                span: to_range(span.span()),
            };

            // Separate metadata from rules
            for line in lines {
                match line {
                    ProfileLine::Metadata(meta) => match meta {
                        ProfileMetadata::Parent(parent) => profile.parent = Some(parent),
                        ProfileMetadata::Id(id) => profile.id = Some(id),
                        ProfileMetadata::Title(title) => profile.title = Some(title),
                        ProfileMetadata::Description(desc) => profile.description = Some(desc),
                        ProfileMetadata::CaretValue(caret) => profile.rules.push(SDRule::CaretValue(caret)),
                    },
                    ProfileLine::Rule(rule) => profile.rules.push(rule),
                }
            }

            profile
        })
}

#[derive(Debug, Clone)]
enum ProfileLine {
    Metadata(ProfileMetadata),
    Rule(SDRule),
}

fn profile_line<'src, I>() -> impl Parser<'src, I, ProfileLine, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        profile_metadata().map(ProfileLine::Metadata),
        sd_rule().map(ProfileLine::Rule),
    ))
}

#[derive(Debug, Clone)]
enum ProfileMetadata {
    Parent(Spanned<String>),
    Id(Spanned<String>),
    Title(Spanned<String>),
    Description(Spanned<String>),
    CaretValue(CaretValueRule),
}

fn profile_metadata<'src, I>() -> impl Parser<'src, I, ProfileMetadata, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        just(Token::Parent)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ProfileMetadata::Parent),
        just(Token::Id)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ProfileMetadata::Id),
        just(Token::Title)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ProfileMetadata::Title),
        just(Token::Description)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ProfileMetadata::Description),
        // Top-level caret rules (without * prefix)
        top_level_caret_rule().map(ProfileMetadata::CaretValue),
    ))
}

// Caret rule at metadata level (no * prefix needed)
fn top_level_caret_rule<'src, I>() -> impl Parser<'src, I, CaretValueRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Caret)
        .ignore_then(caret_path())
        .then_ignore(just(Token::Equal))
        .then(value_literal())
        .map_with(|(caret_path, value), span| CaretValueRule {
            path: None,
            caret_path,
            value,
            span: to_range(span.span()),
        })
}

fn sd_rule<'src, I>() -> impl Parser<'src, I, SDRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        card_rule().map(SDRule::Card),
        flag_rule().map(SDRule::Flag),
        valueset_binding_rule().map(SDRule::ValueSet),
        fixed_value_rule().map(SDRule::FixedValue),
        contains_rule().map(SDRule::Contains),
        only_rule().map(SDRule::Only),
        obeys_rule().map(SDRule::Obeys),
        caret_value_rule().map(SDRule::CaretValue),
        insert_rule().map(SDRule::Insert),
        path_rule().map(SDRule::Path),
    ))
}

fn card_rule<'src, I>() -> impl Parser<'src, I, CardRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(path())
        .then(cardinality())
        .then(flags().or_not())
        .map_with(|((path, cardinality), flags), ext| CardRule {
            path,
            cardinality,
            flags: flags.unwrap_or_default(),
            span: to_range(ext.span()),
        })
}

fn flag_rule<'src, I>() -> impl Parser<'src, I, FlagRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(path().separated_by(just(Token::Comma)).at_least(1).collect::<Vec<_>>())
        .then(flags())
        .map_with(|(paths, flags), span| FlagRule {
            paths,
            flags,
            span: to_range(span.span()),
        })
}

fn valueset_binding_rule<'src, I>() -> impl Parser<'src, I, ValueSetRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(path())
        .then_ignore(just(Token::From))
        .then(string_like())
        .then(
            just(Token::LParen)
                .ignore_then(binding_strength())
                .then_ignore(just(Token::RParen))
                .or_not(),
        )
        .map_with(|((path, value_set), strength), span| ValueSetRule {
            path,
            value_set,
            strength,
            span: to_range(span.span()),
        })
}

fn fixed_value_rule<'src, I>() -> impl Parser<'src, I, FixedValueRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    let exactly = just(Token::Exactly).to(true).or_not();

    just(Token::Star)
        .ignore_then(path())
        .then_ignore(just(Token::Equal))
        .then(exactly)
        .then(value_literal())
        .map_with(|((path, exactly), value), span| FixedValueRule {
            path,
            value,
            exactly: exactly.unwrap_or(false),
            span: to_range(span.span()),
        })
}

fn contains_rule<'src, I>() -> impl Parser<'src, I, ContainsRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(path())
        .then_ignore(just(Token::Contains))
        .then(
            contains_item()
                .separated_by(just(Token::And))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map_with(|(path, items), span| ContainsRule {
            path,
            items,
            span: to_range(span.span()),
        })
}

fn contains_item<'src, I>() -> impl Parser<'src, I, ContainsItem, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    string_like()
        .then(
            just(Token::Named)
                .ignore_then(string_like())
                .or_not(),
        )
        .then(cardinality().or_not())
        .then(flags().or_not())
        .map_with(|(((name, named_as), cardinality), flags), span| ContainsItem {
            name,
            named_as,
            cardinality,
            flags: flags.unwrap_or_default(),
            span: to_range(span.span()),
        })
}

fn only_rule<'src, I>() -> impl Parser<'src, I, OnlyRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(path())
        .then_ignore(just(Token::Only))
        .then(
            string_like()
                .separated_by(just(Token::Or))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map_with(|(path, types), span| OnlyRule {
            path,
            types,
            span: to_range(span.span()),
        })
}

fn obeys_rule<'src, I>() -> impl Parser<'src, I, ObeysRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    let path_then_obeys = path().map(Some).then_ignore(just(Token::Obeys));
    let just_obeys = just(Token::Obeys).to(None::<Spanned<String>>);

    just(Token::Star)
        .ignore_then(path_then_obeys.or(just_obeys))
        .then(
            string_like()
                .separated_by(just(Token::And))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map_with(|(path, invariants), span| ObeysRule {
            path,
            invariants,
            span: to_range(span.span()),
        })
}

fn path_rule<'src, I>() -> impl Parser<'src, I, PathRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(path())
        .map_with(|path, ext| PathRule {
            path,
            span: to_range(ext.span()),
        })
}

// ============================================================================
// Extension Parser
// ============================================================================

fn extension_parser<'src, I>() -> impl Parser<'src, I, Extension, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Extension)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(extension_line().repeated().collect::<Vec<_>>())
        .map_with(|(name, lines), span| {
            let mut extension = Extension {
                name,
                parent: None,
                id: None,
                title: None,
                description: None,
                contexts: Vec::new(),
                rules: Vec::new(),
                span: to_range(span.span()),
            };

            // Separate metadata from rules
            for line in lines {
                match line {
                    ExtensionLine::Metadata(meta) => match meta {
                        ExtensionMetadata::Parent(parent) => extension.parent = Some(parent),
                        ExtensionMetadata::Id(id) => extension.id = Some(id),
                        ExtensionMetadata::Title(title) => extension.title = Some(title),
                        ExtensionMetadata::Description(desc) => extension.description = Some(desc),
                        ExtensionMetadata::Context(ctx) => extension.contexts.push(ctx),
                        ExtensionMetadata::CaretValue(caret) => extension.rules.push(SDRule::CaretValue(caret)),
                    },
                    ExtensionLine::Rule(rule) => extension.rules.push(rule),
                }
            }

            extension
        })
}

#[derive(Debug, Clone)]
enum ExtensionLine {
    Metadata(ExtensionMetadata),
    Rule(SDRule),
}

fn extension_line<'src, I>() -> impl Parser<'src, I, ExtensionLine, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        extension_metadata().map(ExtensionLine::Metadata),
        sd_rule().map(ExtensionLine::Rule),
    ))
}

#[derive(Debug, Clone)]
enum ExtensionMetadata {
    Parent(Spanned<String>),
    Id(Spanned<String>),
    Title(Spanned<String>),
    Description(Spanned<String>),
    Context(Spanned<String>),
    CaretValue(CaretValueRule),
}

fn extension_metadata<'src, I>() -> impl Parser<'src, I, ExtensionMetadata, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        just(Token::Parent)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ExtensionMetadata::Parent),
        just(Token::Id)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ExtensionMetadata::Id),
        just(Token::Title)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ExtensionMetadata::Title),
        just(Token::Description)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ExtensionMetadata::Description),
        just(Token::Context)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(ExtensionMetadata::Context),
        // Top-level caret rules (without * prefix)
        top_level_caret_rule().map(ExtensionMetadata::CaretValue),
    ))
}

// ============================================================================
// Instance Parser
// ============================================================================

fn instance_parser<'src, I>() -> impl Parser<'src, I, Instance, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Instance)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(instance_metadata().repeated().collect::<Vec<_>>())
        .then(instance_rule().repeated().collect::<Vec<_>>())
        .map_with(|((name, metadata), rules), span| {
            let mut instance = Instance {
                name,
                instance_of: None,
                title: None,
                description: None,
                usage: None,
                rules,
                span: to_range(span.span()),
            };

            for meta in metadata {
                match meta {
                    InstanceMetadata::InstanceOf(instance_of) => instance.instance_of = Some(instance_of),
                    InstanceMetadata::Title(title) => instance.title = Some(title),
                    InstanceMetadata::Description(desc) => instance.description = Some(desc),
                    InstanceMetadata::Usage(usage) => instance.usage = Some(usage),
                }
            }

            instance
        })
}

#[derive(Debug, Clone)]
enum InstanceMetadata {
    InstanceOf(Spanned<String>),
    Title(Spanned<String>),
    Description(Spanned<String>),
    Usage(Spanned<String>),
}

fn instance_metadata<'src, I>() -> impl Parser<'src, I, InstanceMetadata, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        just(Token::InstanceOf)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(InstanceMetadata::InstanceOf),
        just(Token::Title)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(InstanceMetadata::Title),
        just(Token::Description)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(InstanceMetadata::Description),
        just(Token::Usage)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(InstanceMetadata::Usage),
    ))
}

fn instance_rule<'src, I>() -> impl Parser<'src, I, InstanceRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        fixed_value_rule().map(InstanceRule::FixedValue),
        insert_rule().map(InstanceRule::Insert),
        path_rule().map(InstanceRule::Path),
    ))
}

// ============================================================================
// CodeSystem Parser
// ============================================================================

fn code_system_parser<'src, I>() -> impl Parser<'src, I, CodeSystem, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::CodeSystem)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(metadata().repeated().collect::<Vec<_>>())
        .then(cs_rule().repeated().collect::<Vec<_>>())
        .map_with(|((name, metadata), rules), span| {
            let mut code_system = CodeSystem {
                name,
                id: None,
                title: None,
                description: None,
                concepts: Vec::new(),
                rules: Vec::new(),
                span: to_range(span.span()),
            };

            for meta in metadata {
                match meta {
                    Metadata::Parent(_parent) => {
                        // code_system doesn't have parent field, ignore
                    },
                    Metadata::Id(id) => code_system.id = Some(id),
                    Metadata::Title(title) => code_system.title = Some(title),
                    Metadata::Description(desc) => code_system.description = Some(desc),
                }
            }

            code_system.rules = rules;
            code_system
        })
}

fn cs_rule<'src, I>() -> impl Parser<'src, I, CSRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        concept_rule().map(CSRule::Concept),
        code_caret_value_rule().map(CSRule::CodeCaretValue),
        code_insert_rule().map(CSRule::CodeInsert),
    ))
}

fn concept_rule<'src, I>() -> impl Parser<'src, I, Concept, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Star)
        .ignore_then(code_literal().repeated().at_least(1).collect::<Vec<_>>())
        .then(
            string_like()
                .map(Some)
                .or_not()
                .map(|opt| opt.flatten()),
        )
        .map_with(|(codes, display), span| Concept {
            codes,
            display,
            definition: None,
            span: to_range(span.span()),
        })
}

// ============================================================================
// Invariant Parser
// ============================================================================

fn invariant_parser<'src, I>() -> impl Parser<'src, I, Invariant, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Invariant)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(invariant_metadata().repeated().collect::<Vec<_>>())
        .map_with(|(name, metadata), span| {
            let mut invariant = Invariant {
                name,
                description: None,
                expression: None,
                xpath: None,
                severity: None,
                span: to_range(span.span()),
            };

            for meta in metadata {
                match meta {
                    InvariantMetadata::Description(desc) => invariant.description = Some(desc),
                    InvariantMetadata::Expression(expr) => invariant.expression = Some(expr),
                    InvariantMetadata::XPath(xpath) => invariant.xpath = Some(xpath),
                    InvariantMetadata::Severity(severity) => invariant.severity = Some(severity),
                }
            }

            invariant
        })
}

#[derive(Debug, Clone)]
enum InvariantMetadata {
    Description(Spanned<String>),
    Expression(Spanned<String>),
    XPath(Spanned<String>),
    Severity(Spanned<String>),
}

fn invariant_metadata<'src, I>() -> impl Parser<'src, I, InvariantMetadata, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        just(Token::Description)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(InvariantMetadata::Description),
        just(Token::Expression)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(InvariantMetadata::Expression),
        just(Token::XPath)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(InvariantMetadata::XPath),
        just(Token::Severity)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(InvariantMetadata::Severity),
    ))
}

// ============================================================================
// Mapping Parser
// ============================================================================

fn mapping_parser<'src, I>() -> impl Parser<'src, I, Mapping, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Mapping)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(mapping_metadata().repeated().collect::<Vec<_>>())
        .map_with(|(name, metadata), span| {
            let mut mapping = Mapping {
                name,
                id: None,
                source: None,
                target: None,
                description: None,
                title: None,
                span: to_range(span.span()),
            };

            for meta in metadata {
                match meta {
                    MappingMetadata::Id(id) => mapping.id = Some(id),
                    MappingMetadata::Source(source) => mapping.source = Some(source),
                    MappingMetadata::Target(target) => mapping.target = Some(target),
                    MappingMetadata::Description(desc) => mapping.description = Some(desc),
                    MappingMetadata::Title(title) => mapping.title = Some(title),
                }
            }

            mapping
        })
}

#[derive(Debug, Clone)]
enum MappingMetadata {
    Id(Spanned<String>),
    Source(Spanned<String>),
    Target(Spanned<String>),
    Description(Spanned<String>),
    Title(Spanned<String>),
}

fn mapping_metadata<'src, I>() -> impl Parser<'src, I, MappingMetadata, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        just(Token::Id)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(MappingMetadata::Id),
        just(Token::Source)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(MappingMetadata::Source),
        just(Token::Target)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(MappingMetadata::Target),
        just(Token::Description)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(MappingMetadata::Description),
        just(Token::Title)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(MappingMetadata::Title),
    ))
}

// ============================================================================
// Logical Parser
// ============================================================================

fn logical_parser<'src, I>() -> impl Parser<'src, I, Logical, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Logical)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(logical_metadata().repeated().collect::<Vec<_>>())
        .then(lr_rule().repeated().collect::<Vec<_>>())
        .map_with(|((name, metadata), rules), span| {
            let mut logical = Logical {
                name,
                parent: None,
                id: None,
                title: None,
                description: None,
                characteristics: Vec::new(),
                rules,
                span: to_range(span.span()),
            };

            for meta in metadata {
                match meta {
                    LogicalMetadata::Parent(parent) => logical.parent = Some(parent),
                    LogicalMetadata::Id(id) => logical.id = Some(id),
                    LogicalMetadata::Title(title) => logical.title = Some(title),
                    LogicalMetadata::Description(desc) => logical.description = Some(desc),
                    LogicalMetadata::Characteristics(chars) => logical.characteristics.push(chars),
                }
            }

            logical
        })
}

#[derive(Debug, Clone)]
enum LogicalMetadata {
    Parent(Spanned<String>),
    Id(Spanned<String>),
    Title(Spanned<String>),
    Description(Spanned<String>),
    Characteristics(Spanned<String>),
}

fn logical_metadata<'src, I>() -> impl Parser<'src, I, LogicalMetadata, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        just(Token::Parent)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(LogicalMetadata::Parent),
        just(Token::Id)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(LogicalMetadata::Id),
        just(Token::Title)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(LogicalMetadata::Title),
        just(Token::Description)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(LogicalMetadata::Description),
        just(Token::Characteristics)
            .ignore_then(just(Token::Colon))
            .ignore_then(string_like())
            .map(LogicalMetadata::Characteristics),
    ))
}

// ============================================================================
// Resource Parser
// ============================================================================

fn resource_parser<'src, I>() -> impl Parser<'src, I, Resource, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    just(Token::Resource)
        .ignore_then(just(Token::Colon))
        .ignore_then(name())
        .then(profile_metadata().repeated().collect::<Vec<_>>())
        .then(lr_rule().repeated().collect::<Vec<_>>())
        .map_with(|((name, metadata), rules), span| {
            let mut resource = Resource {
                name,
                parent: None,
                id: None,
                title: None,
                description: None,
                rules,
                span: to_range(span.span()),
            };

            for meta in metadata {
                match meta {
                    ProfileMetadata::Parent(parent) => resource.parent = Some(parent),
                    ProfileMetadata::Id(id) => resource.id = Some(id),
                    ProfileMetadata::Title(title) => resource.title = Some(title),
                    ProfileMetadata::Description(desc) => resource.description = Some(desc),
                    ProfileMetadata::CaretValue(caret) => resource.rules.push(LRRule::SD(SDRule::CaretValue(caret))),
                }
            }

            resource
        })
}

fn lr_rule<'src, I>() -> impl Parser<'src, I, LRRule, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        sd_rule().map(LRRule::SD),
        // add_element_rule().map(LRRule::AddElement), // TODO: Implement AddElement rule
    ))
}

// ============================================================================
// Helper Parsers
// ============================================================================

fn path<'src, I>() -> impl Parser<'src, I, Spanned<String>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    // Parse paths with optional slicing and dot notation
    // Examples: identifier, name.given, identifier[ssn], name.given[0]
    string_like()
        .then(
            // Optional slicing [name]
            just(Token::LBracket)
                .ignore_then(string_like())
                .then_ignore(just(Token::RBracket))
                .or_not()
                .boxed()
        )
        .then(
            // Optional dot-separated segments: .given.family etc.
            just(Token::Dot)
                .ignore_then(string_like())
                .then(
                    // Optional slicing after dot segment
                    just(Token::LBracket)
                        .ignore_then(string_like())
                        .then_ignore(just(Token::RBracket))
                        .or_not()
                        .boxed()
                )
                .repeated()
                .collect::<Vec<_>>()
                .boxed()
        )
        .map_with(|((base, slice), segments), ext| {
            let mut path = base.value.clone();

            // Add slice to base if present
            if let Some(slice_name) = slice {
                path.push('[');
                path.push_str(&slice_name.value);
                path.push(']');
            }

            // Add dot-separated segments
            for (segment, seg_slice) in segments {
                path.push('.');
                path.push_str(&segment.value);
                if let Some(slice_name) = seg_slice {
                    path.push('[');
                    path.push_str(&slice_name.value);
                    path.push(']');
                }
            }

            Spanned {
                value: path,
                span: to_range(ext.span()),
            }
        })
        .boxed()
}

fn cardinality<'src, I>() -> impl Parser<'src, I, Spanned<Cardinality>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    let number = select! { Token::Number(n) => n };
    let star = just(Token::Star).to("*".to_string());

    number
        .or(star.clone())
        .then_ignore(just(Token::Dot).then(just(Token::Dot)))
        .then(number.or(star))
        .map_with(|(min_str, max_str), span| {
            let min = if min_str == "*" {
                None
            } else {
                min_str.parse::<u32>().ok()
            };

            let max = if max_str == "*" {
                CardinalityMax::Star
            } else {
                CardinalityMax::Number(max_str.parse::<u32>().unwrap_or(0))
            };

            Spanned {
                value: Cardinality { min, max },
                span: to_range(span.span()),
            }
        })
}

fn flags<'src, I>() -> impl Parser<'src, I, Vec<Spanned<Flag>>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    choice((
        just(Token::MS).to(Flag::MS),
        just(Token::SU).to(Flag::SU),
        just(Token::TU).to(Flag::TU),
        just(Token::N).to(Flag::N),
        just(Token::D).to(Flag::D),
        just(Token::Mod).to(Flag::Modifier),
    ))
    .map_with(|flag, ext| Spanned {
        value: flag,
        span: to_range(ext.span()),
    })
    .repeated()
    .at_least(1)
    .collect::<Vec<_>>()
}

fn binding_strength<'src, I>() -> impl Parser<'src, I, Spanned<BindingStrength>, ParserExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = LexSpan>,
{
    // Parser accepts any identifier - semantic analyzer validates
    // This is proper compiler design: parse liberally, validate semantically
    choice((
        just(Token::Required).to(BindingStrength::Required),
        just(Token::Extensible).to(BindingStrength::Extensible),
        just(Token::Preferred).to(BindingStrength::Preferred),
        just(Token::Example).to(BindingStrength::Example),
        // Accept any other identifier as Unknown for semantic validation
        string_like().map(|s| BindingStrength::Unknown(s.value)),
    ))
    .map_with(|strength, ext| Spanned {
        value: strength,
        span: to_range(ext.span()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use chumsky::span::Span as _;

    #[test]
    fn parse_valueset_document() {
        let input = r#"
ValueSet: TestVS
Id: test-vs
Title: "Test ValueSet"
Description: "Example"
* include codes from system http://loinc.org where concept is-a #8310-5
* exclude http://loinc.org#94563-4
* ^compose.include[0].display = "Blood Pressure"
* #8310-5 insert CommonCodes
"#;

        let (tokens, lex_errors) = lex(input);
        assert!(lex_errors.is_empty(), "Lexer errors: {:?}", lex_errors);

        let eof = LexSpan::new(input.len(), input.len());
        let (document, parse_errors) = parse_tokens(&tokens, eof);
        assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);

        let document = document.expect("expected parsed document");
        assert_eq!(document.value_sets.len(), 1);
        let vs = &document.value_sets[0];
        assert_eq!(vs.name.value, "TestVS");
        assert_eq!(vs.id.as_ref().map(|id| id.value.as_str()), Some("test-vs"));
        assert_eq!(vs.components.len(), 2);
        assert_eq!(vs.rules.len(), 2);
    }
}
