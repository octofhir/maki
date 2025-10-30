//! Hierarchical parser for FSH constructs
//!
//! This module builds a structured CST from tokens, creating proper nodes for
//! Profiles, Extensions, Rules, etc. Unlike the flat parser, this creates a
//! hierarchical tree that matches the FSH grammar.

use super::lexer::LexerError;
use super::{CstBuilder, CstToken, FshSyntaxKind, FshSyntaxNode};

/// Parse FSH source into a hierarchical CST
///
/// This creates proper structure with Profile, Extension, Rule nodes.
///
/// # Example
///
/// ```rust,ignore
/// use maki_core::cst::parse_fsh;
///
/// let source = r#"
/// Profile: MyPatient
/// Parent: Patient
/// * name 1..1 MS
/// "#;
///
/// let (cst, errors) = parse_fsh(source);
/// assert!(errors.is_empty());
/// assert_eq!(cst.text().to_string(), source);
/// ```
pub fn parse_fsh(source: &str) -> (FshSyntaxNode, Vec<LexerError>) {
    let (tokens, errors) = super::lex_with_trivia(source);
    let cst = parse_tokens(&tokens);
    (cst, errors)
}

/// Parse a token stream into a hierarchical CST
fn parse_tokens(tokens: &[CstToken]) -> FshSyntaxNode {
    let mut parser = Parser::new(tokens);
    parser.parse_document();
    parser.finish()
}

/// Token stream parser
struct Parser<'a> {
    tokens: &'a [CstToken],
    pos: usize,
    builder: CstBuilder,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [CstToken]) -> Self {
        Self {
            tokens,
            pos: 0,
            builder: CstBuilder::new(),
        }
    }

    fn finish(self) -> FshSyntaxNode {
        self.builder.finish()
    }

    /// Parse the top-level document
    fn parse_document(&mut self) {
        self.builder.start_node(FshSyntaxKind::Document);

        let mut doc_iteration = 0;
        while !self.at_end() {
            doc_iteration += 1;
            if doc_iteration > 10000 {
                break;
            }

            // Skip trivia at document level
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            // Parse top-level declarations
            match self.current_kind() {
                FshSyntaxKind::ProfileKw => self.parse_profile(),
                FshSyntaxKind::ExtensionKw => self.parse_extension(),
                FshSyntaxKind::ValuesetKw => self.parse_valueset(),
                FshSyntaxKind::CodesystemKw => self.parse_codesystem(),
                FshSyntaxKind::InstanceKw => self.parse_instance(),
                FshSyntaxKind::InvariantKw => self.parse_invariant(),
                FshSyntaxKind::MappingKw => self.parse_mapping(),
                FshSyntaxKind::LogicalKw => self.parse_logical(),
                FshSyntaxKind::ResourceKw => self.parse_resource(),
                FshSyntaxKind::AliasKw => self.parse_alias(),
                FshSyntaxKind::RulesetKw => {
                    let start_pos = self.pos;
                    self.parse_ruleset();
                    let end_pos = self.pos;
                    if start_pos == end_pos {
                        break; // Prevent infinite loop
                    }
                }
                FshSyntaxKind::CommentLine
                | FshSyntaxKind::CommentBlock
                | FshSyntaxKind::Newline => {
                    // Preserve comments and newlines at document level
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Eof => break,
                _ => {
                    // Unknown token - consume as error
                    self.error_and_recover();
                }
            }
        }

        self.builder.finish_node(); // DOCUMENT
    }

    /// Parse a Profile declaration
    ///
    /// Grammar: Profile: <name> <metadata>* <rule>*
    fn parse_profile(&mut self) {
        self.builder.start_node(FshSyntaxKind::Profile);

        // Profile keyword
        self.expect(FshSyntaxKind::ProfileKw);
        self.consume_trivia();

        // Colon
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();

        // Profile name
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Parse metadata clauses and rules
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                // Metadata keywords
                FshSyntaxKind::ParentKw => self.parse_parent_clause(),
                FshSyntaxKind::IdKw => self.parse_id_clause(),
                FshSyntaxKind::TitleKw => self.parse_title_clause(),
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),

                // Rules start with *
                FshSyntaxKind::Asterisk => self.parse_lr_rule(),

                // Comment or newline
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }

                _ => break,
            }
        }

        self.builder.finish_node(); // PROFILE
    }

    /// Parse Extension declaration
    fn parse_extension(&mut self) {
        self.builder.start_node(FshSyntaxKind::Extension);

        self.expect(FshSyntaxKind::ExtensionKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Parse metadata and rules (same as Profile)
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                FshSyntaxKind::ParentKw => self.parse_parent_clause(),
                FshSyntaxKind::IdKw => self.parse_id_clause(),
                FshSyntaxKind::TitleKw => self.parse_title_clause(),
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),
                FshSyntaxKind::ContextKw => self.parse_context_clause(),
                FshSyntaxKind::CharacteristicsKw => self.parse_characteristics_clause(),
                FshSyntaxKind::Asterisk => self.parse_lr_rule(),
                FshSyntaxKind::Caret => self.parse_rule(), // Caret rules like ^extension[FMM]
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }
                _ => break,
            }
        }

        self.builder.finish_node(); // EXTENSION
    }

    /// Parse ValueSet declaration
    fn parse_valueset(&mut self) {
        self.builder.start_node(FshSyntaxKind::ValueSet);
        self.expect(FshSyntaxKind::ValuesetKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Parse metadata clauses and rules
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                // Metadata keywords
                FshSyntaxKind::IdKw => self.parse_id_clause(),
                FshSyntaxKind::TitleKw => self.parse_title_clause(),
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),

                // ValueSet rules starting with *
                FshSyntaxKind::Asterisk => self.parse_vs_rule(),

                // Caret rules (for metadata like ^status)
                FshSyntaxKind::Caret => self.parse_rule(),

                // Comment or newline
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }

                _ => {
                    self.error_and_recover();
                }
            }
        }

        self.builder.finish_node();
    }

    /// Parse CodeSystem declaration
    fn parse_codesystem(&mut self) {
        self.builder.start_node(FshSyntaxKind::CodeSystem);
        self.expect(FshSyntaxKind::CodesystemKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Parse metadata clauses and concepts
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                // Metadata keywords
                FshSyntaxKind::IdKw => self.parse_id_clause(),
                FshSyntaxKind::TitleKw => self.parse_title_clause(),
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),

                // CodeSystem concepts starting with *
                FshSyntaxKind::Asterisk => self.parse_concept(),

                // Caret rules (for metadata like ^status)
                FshSyntaxKind::Caret => self.parse_rule(),

                // Comment or newline
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }

                _ => {
                    self.error_and_recover();
                }
            }
        }

        self.builder.finish_node();
    }

    /// Parse Instance declaration
    fn parse_instance(&mut self) {
        self.builder.start_node(FshSyntaxKind::Instance);

        // Instance keyword
        self.expect(FshSyntaxKind::InstanceKw);
        self.consume_trivia();

        // Colon
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();

        // Instance name
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Parse metadata clauses and rules
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                // Metadata keywords
                FshSyntaxKind::InstanceofKw => self.parse_instanceof_clause(),
                FshSyntaxKind::UsageKw => self.parse_usage_clause(),
                FshSyntaxKind::TitleKw => self.parse_title_clause(),
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),

                // Rules start with *
                FshSyntaxKind::Asterisk => self.parse_lr_rule(),

                // Comments and newlines
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }

                // Unknown - skip to next line
                _ => {
                    self.error_and_recover();
                }
            }
        }

        self.builder.finish_node();
    }

    /// Parse Invariant declaration
    fn parse_invariant(&mut self) {
        self.builder.start_node(FshSyntaxKind::Invariant);

        // Invariant keyword
        self.expect(FshSyntaxKind::InvariantKw);
        self.consume_trivia();

        // Colon
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();

        // Invariant name
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Parse metadata clauses (Severity, XPath, Expression, Description)
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                // Metadata keywords
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),
                FshSyntaxKind::SeverityKw => self.parse_severity_clause(),
                FshSyntaxKind::XpathKw => self.parse_xpath_clause(),
                FshSyntaxKind::ExpressionKw => self.parse_expression_clause(),

                // Newlines between clauses
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }

                // Unknown - skip to next line
                _ => {
                    self.error_and_recover();
                }
            }
        }

        self.builder.finish_node();
    }

    /// Parse Alias declaration
    fn parse_alias(&mut self) {
        self.builder.start_node(FshSyntaxKind::Alias);
        self.expect(FshSyntaxKind::AliasKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();

        // Alias name
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia();

        // Equals
        if self.at(FshSyntaxKind::Equals) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();

            // Alias value (URL or identifier) - consume all tokens until newline
            // URLs like "http://example.com/path" are lexed as multiple tokens (ident, colon, slashes, etc.)
            // We need to collect all of them into the alias value
            while !self.at_end()
                && !self.at(FshSyntaxKind::Newline)
                && self.current().map(|t| t.kind) != Some(FshSyntaxKind::Whitespace)
            {
                self.add_current_token();
                self.advance();
            }
        }

        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse RuleSet declaration
    /// Grammar: RuleSet: Name or RuleSet: Name(param1, param2, ...)
    fn parse_ruleset(&mut self) {
        self.builder.start_node(FshSyntaxKind::RuleSet);

        self.expect(FshSyntaxKind::RulesetKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();

        // RuleSet name
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia();

        // Optional parameters: (param1, param2)
        if self.at(FshSyntaxKind::LParen) {
            self.parse_parameter_list();
        }

        self.consume_trivia_and_newlines();

        let mut iteration_count = 0;
        let mut consecutive_newlines = 0;
        let mut non_rule_token_count = 0;

        // Parse rules in the RuleSet body
        while !self.at_end() {
            // Check for definition keyword FIRST
            if self.at_definition_keyword() {
                break;
            }
            iteration_count += 1;
            if iteration_count > 1000 {
                break;
            }

            // PRAGMATIC FIX: If we've seen 20+ non-rule tokens in a row, we're probably past the RuleSet
            if non_rule_token_count > 20 {
                break;
            }

            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                FshSyntaxKind::Asterisk => {
                    consecutive_newlines = 0; // Reset on rule
                    non_rule_token_count = 0; // Reset on rule
                    self.parse_lr_rule();
                }
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    consecutive_newlines = 0; // Reset on comment
                    non_rule_token_count = 0; // Reset on comment
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    consecutive_newlines += 1;
                    non_rule_token_count += 1;
                    self.add_current_token();
                    self.advance();
                    // CRITICAL FIX: Break on blank line (2+ newlines = end of RuleSet)
                    if consecutive_newlines >= 2 {
                        break;
                    }
                }
                _ => {
                    consecutive_newlines = 0; // Reset on other tokens
                    non_rule_token_count += 1;
                    // Don't break on unknown tokens (like LBrace from template params)
                    // Just skip them and continue parsing
                    self.add_current_token();
                    self.advance();
                }
            }
        }

        self.builder.finish_node(); // RULE_SET
    }

    /// Parse parameter list for parameterized RuleSet: (param1, param2, ...)
    fn parse_parameter_list(&mut self) {
        self.builder.start_node(FshSyntaxKind::ParameterList);

        self.expect(FshSyntaxKind::LParen);
        self.consume_trivia();

        // Parse parameters
        let mut param_count = 0;
        while !self.at_end() && !self.at(FshSyntaxKind::RParen) {
            // Safety check for infinite loop
            if param_count > 100 {
                break;
            }

            self.parse_parameter();
            param_count += 1;

            // Comma separator between parameters
            if self.at(FshSyntaxKind::Comma) {
                self.add_current_token();
                self.advance();
                self.consume_trivia();
            } else if !self.at(FshSyntaxKind::RParen) {
                // CRITICAL FIX: If no comma and not at ), we need to break
                // Otherwise we'll loop infinitely trying to parse parameters
                break;
            }
        }

        self.expect(FshSyntaxKind::RParen);
        self.builder.finish_node(); // PARAMETER_LIST
    }

    /// Parse single parameter in RuleSet parameter list
    fn parse_parameter(&mut self) {
        self.builder.start_node(FshSyntaxKind::Parameter);
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia();
        self.builder.finish_node(); // PARAMETER
    }

    /// Parse insert arguments in insert rule: (arg1, arg2, ...)
    fn parse_insert_arguments(&mut self) {
        self.builder.start_node(FshSyntaxKind::InsertRuleArgs);

        self.expect(FshSyntaxKind::LParen);
        self.consume_trivia();

        // Parse arguments
        while !self.at_end() && !self.at(FshSyntaxKind::RParen) {
            // Parse argument value - can be various types
            self.parse_insert_argument();

            // Comma separator between arguments
            if self.at(FshSyntaxKind::Comma) {
                self.add_current_token();
                self.advance();
                self.consume_trivia();
            }
        }

        self.expect(FshSyntaxKind::RParen);
        self.builder.finish_node(); // INSERT_RULE_ARGS
    }

    /// Parse single argument in insert rule
    fn parse_insert_argument(&mut self) {
        // Arguments can be: strings, numbers, booleans, codes, identifiers, etc.
        if self.at(FshSyntaxKind::String)
            || self.at(FshSyntaxKind::Integer)
            || self.at(FshSyntaxKind::Decimal)
            || self.at(FshSyntaxKind::True)
            || self.at(FshSyntaxKind::False)
            || self.at(FshSyntaxKind::Hash)
        {
            // Code value starts with #
            if self.at(FshSyntaxKind::Hash) {
                self.add_current_token();
                self.advance();
                if self.at(FshSyntaxKind::Ident) {
                    self.add_current_token();
                    self.advance();
                }
            } else {
                self.add_current_token();
                self.advance();
            }
        } else if self.at(FshSyntaxKind::Ident) {
            // Could be identifier or path
            self.add_current_token();
            self.advance();
            // Handle path continuation (dot notation)
            while self.at(FshSyntaxKind::Dot) {
                self.add_current_token();
                self.advance();
                if self.at(FshSyntaxKind::Ident) {
                    self.add_current_token();
                    self.advance();
                }
            }
        }
        self.consume_trivia();
    }

    /// Parse Mapping declaration
    fn parse_mapping(&mut self) {
        self.builder.start_node(FshSyntaxKind::Mapping);

        self.expect(FshSyntaxKind::MappingKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Parse metadata and rules (same structure as Profile)
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                FshSyntaxKind::IdKw => self.parse_id_clause(),
                FshSyntaxKind::SourceKw => self.parse_source_clause(),
                FshSyntaxKind::TargetKw => self.parse_target_clause(),
                FshSyntaxKind::TitleKw => self.parse_title_clause(),
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),
                FshSyntaxKind::Asterisk => self.parse_lr_rule(),
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }
                _ => break,
            }
        }

        self.builder.finish_node(); // MAPPING
    }

    /// Parse Logical declaration
    fn parse_logical(&mut self) {
        self.builder.start_node(FshSyntaxKind::Logical);

        self.expect(FshSyntaxKind::LogicalKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Same as Profile
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                FshSyntaxKind::ParentKw => self.parse_parent_clause(),
                FshSyntaxKind::IdKw => self.parse_id_clause(),
                FshSyntaxKind::TitleKw => self.parse_title_clause(),
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),
                FshSyntaxKind::Asterisk => self.parse_lr_rule(),
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }
                _ => break,
            }
        }

        self.builder.finish_node(); // LOGICAL
    }

    /// Parse Resource declaration
    fn parse_resource(&mut self) {
        self.builder.start_node(FshSyntaxKind::Resource);

        self.expect(FshSyntaxKind::ResourceKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();

        // Same as Logical
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                FshSyntaxKind::ParentKw => self.parse_parent_clause(),
                FshSyntaxKind::IdKw => self.parse_id_clause(),
                FshSyntaxKind::TitleKw => self.parse_title_clause(),
                FshSyntaxKind::DescriptionKw => self.parse_description_clause(),
                FshSyntaxKind::Asterisk => self.parse_lr_rule(),
                FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock => {
                    self.add_current_token();
                    self.advance();
                }
                FshSyntaxKind::Newline => {
                    self.add_current_token();
                    self.advance();
                }
                _ => break,
            }
        }

        self.builder.finish_node(); // RESOURCE
    }

    /// Parse Source clause (for Mappings)
    fn parse_source_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::SourceClause);
        self.expect(FshSyntaxKind::SourceKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Target clause (for Mappings)
    fn parse_target_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::TargetClause);
        self.expect(FshSyntaxKind::TargetKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::String);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Parent clause
    fn parse_parent_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::ParentClause);
        self.expect(FshSyntaxKind::ParentKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Id clause
    fn parse_id_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::IdClause);
        self.expect(FshSyntaxKind::IdKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Title clause
    fn parse_title_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::TitleClause);
        self.expect(FshSyntaxKind::TitleKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::String);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Description clause
    fn parse_description_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::DescriptionClause);
        self.expect(FshSyntaxKind::DescriptionKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::String);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse InstanceOf clause
    fn parse_instanceof_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::InstanceofClause);
        self.expect(FshSyntaxKind::InstanceofKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Usage clause
    fn parse_usage_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::UsageClause);
        self.expect(FshSyntaxKind::UsageKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        // Usage value can be #definition, #example, #inline
        if self.at(FshSyntaxKind::Hash) {
            self.add_current_token();
            self.advance();
        }
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Context clause (for Extensions)
    fn parse_context_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::PathRule); // Context is like a path rule
        self.expect(FshSyntaxKind::ContextKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();

        // Parse comma-separated list of contexts
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia();

        // Additional contexts with comma separator
        while self.at(FshSyntaxKind::Comma) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
            self.expect(FshSyntaxKind::Ident);
            self.consume_trivia();
        }

        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Characteristics clause (for Logical models)
    fn parse_characteristics_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::PathRule); // Characteristics is similar to context
        self.expect(FshSyntaxKind::CharacteristicsKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();

        // Parse comma-separated list of code values
        // Each characteristic is a code starting with #
        if self.at(FshSyntaxKind::Hash) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
            if self.at(FshSyntaxKind::Ident) {
                self.add_current_token();
                self.advance();
                self.consume_trivia();
            }
        }

        // Additional characteristics with comma separator
        while self.at(FshSyntaxKind::Comma) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();

            if self.at(FshSyntaxKind::Hash) {
                self.add_current_token();
                self.advance();
                self.consume_trivia();
                if self.at(FshSyntaxKind::Ident) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                }
            }
        }

        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Severity clause
    fn parse_severity_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::SeverityClause);
        self.expect(FshSyntaxKind::SeverityKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        // Severity value can be #error, #warning
        if self.at(FshSyntaxKind::Hash) {
            self.add_current_token();
            self.advance();
        }
        self.expect(FshSyntaxKind::Ident);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse XPath clause
    fn parse_xpath_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::XpathClause);
        self.expect(FshSyntaxKind::XpathKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::String);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse Expression clause
    fn parse_expression_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::ExpressionClause);
        self.expect(FshSyntaxKind::ExpressionKw);
        self.consume_trivia();
        self.expect(FshSyntaxKind::Colon);
        self.consume_trivia();
        self.expect(FshSyntaxKind::String);
        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse a rule (starts with * or ^)
    fn parse_rule(&mut self) {
        // Check if it's a caret rule (^extension[FMM].value = 4)
        if self.at(FshSyntaxKind::Caret) {
            self.builder.start_node(FshSyntaxKind::CaretValueRule);

            // Parse caret path
            self.parse_path();
            self.consume_trivia();

            // Assignment (= or +=)
            if self.at(FshSyntaxKind::Equals) || self.at(FshSyntaxKind::PlusEquals) {
                self.add_current_token();
                self.advance();
                self.consume_trivia();

                // Value expression
                self.parse_value_expression();
            }

            self.consume_trivia_and_newlines();
            self.builder.finish_node();
            return;
        }

        // Regular rule starting with *
        self.expect(FshSyntaxKind::Asterisk);
        self.consume_trivia();

        // Check if it's an insert rule
        if self.at(FshSyntaxKind::InsertKw) {
            self.builder.start_node(FshSyntaxKind::InsertRule);
            self.add_current_token();
            self.advance();
            self.consume_trivia();

            // RuleSet name
            self.expect(FshSyntaxKind::Ident);
            self.consume_trivia();

            // Optional arguments: (arg1, arg2)
            if self.at(FshSyntaxKind::LParen) {
                self.parse_insert_arguments();
            }

            self.consume_trivia_and_newlines();
            self.builder.finish_node();
            return;
        }

        // Parse path
        self.parse_path();
        self.consume_trivia();

        // Determine rule type based on what follows the path
        let rule_kind = if self.at(FshSyntaxKind::Arrow) {
            FshSyntaxKind::MappingRule // Mapping: path -> "target"
        } else if self.at(FshSyntaxKind::Equals) || self.at(FshSyntaxKind::PlusEquals) {
            FshSyntaxKind::FixedValueRule // Assignment rule
        } else if self.at(FshSyntaxKind::ContainsKw) {
            FshSyntaxKind::ContainsRule
        } else if self.at(FshSyntaxKind::FromKw) {
            FshSyntaxKind::ValuesetRule
        } else if self.at(FshSyntaxKind::OnlyKw) {
            FshSyntaxKind::OnlyRule
        } else if self.at(FshSyntaxKind::ObeysKw) {
            FshSyntaxKind::ObeysRule
        } else {
            FshSyntaxKind::CardRule // Default: cardinality/flags
        };

        self.builder.start_node(rule_kind);

        // Parse the rest of the rule based on type
        match rule_kind {
            FshSyntaxKind::MappingRule => {
                // Mapping: path -> "target" "comment"? #language?
                self.expect(FshSyntaxKind::Arrow);
                self.consume_trivia();

                // Target string (required)
                self.expect(FshSyntaxKind::String);
                self.consume_trivia();

                // Optional comment string
                if self.at(FshSyntaxKind::String) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                }

                // Optional language code
                if self.at(FshSyntaxKind::Hash) {
                    self.add_current_token();
                    self.advance();
                    if self.at(FshSyntaxKind::Ident) {
                        self.add_current_token();
                        self.advance();
                    }
                }
            }
            FshSyntaxKind::FixedValueRule => {
                // Assignment: path = value or path += value
                if self.at(FshSyntaxKind::Equals) || self.at(FshSyntaxKind::PlusEquals) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                    self.parse_value_expression();
                }
            }
            FshSyntaxKind::ContainsRule => {
                // Contains: path contains item1 and item2
                self.expect(FshSyntaxKind::ContainsKw);
                self.consume_trivia();

                // Parse contains items
                while !self.at_end() && !self.at(FshSyntaxKind::Newline) {
                    if self.at(FshSyntaxKind::Ident) {
                        self.add_current_token();
                        self.advance();
                        self.consume_trivia();

                        // Optional cardinality
                        if self.at(FshSyntaxKind::Integer) {
                            self.add_current_token();
                            self.advance();
                            self.consume_trivia();
                            if self.at(FshSyntaxKind::Range) {
                                self.add_current_token();
                                self.advance();
                                self.consume_trivia();
                                if self.at(FshSyntaxKind::Integer)
                                    || self.at(FshSyntaxKind::Asterisk)
                                {
                                    self.add_current_token();
                                    self.advance();
                                    self.consume_trivia();
                                }
                            }
                        }

                        // Flags
                        while self.at(FshSyntaxKind::MsFlag) || self.at(FshSyntaxKind::SuFlag) {
                            self.add_current_token();
                            self.advance();
                            self.consume_trivia();
                        }

                        if self.at(FshSyntaxKind::AndKw) {
                            self.add_current_token();
                            self.advance();
                            self.consume_trivia();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            _ => {
                // Generic: consume tokens until newline
                while !self.at_end()
                    && !self.at(FshSyntaxKind::Newline)
                    && !self.at_definition_keyword()
                {
                    if self.at(FshSyntaxKind::CommentLine) {
                        break;
                    }
                    self.add_current_token();
                    self.advance();
                }
            }
        }

        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse a value expression (right-hand side of assignment)
    fn parse_value_expression(&mut self) {
        // Value can be:
        // - String: "value"
        // - Code: #code or System#code "display"
        // - Boolean: true/false
        // - Number: 123 or 1.23
        // - Quantity: 5.4 'mg' "display"
        // - Reference: Reference(Type)
        // - Identifier/path: SomeValue

        if self.at(FshSyntaxKind::String) {
            // String value
            self.add_current_token();
            self.advance();
        } else if self.at(FshSyntaxKind::Hash) {
            // Code value: #code
            self.add_current_token();
            self.advance();
            if self.at(FshSyntaxKind::Ident) {
                self.add_current_token();
                self.advance();
            }
            // Optional display string
            self.consume_trivia();
            if self.at(FshSyntaxKind::String) {
                self.add_current_token();
                self.advance();
            }
        } else if self.at(FshSyntaxKind::True) || self.at(FshSyntaxKind::False) {
            // Boolean
            self.add_current_token();
            self.advance();
        } else if self.at(FshSyntaxKind::Unit) {
            // Quantity without number: 'mg' "display"
            // But also check for Ratio: 'mg':'mL'
            let checkpoint = self.pos;
            self.parse_quantity_value(false);
            self.consume_trivia();

            if self.at(FshSyntaxKind::Colon) {
                // This is part of a Ratio: quantity:quantity
                // Rewind and parse as Ratio
                self.pos = checkpoint;
                self.parse_ratio_value();
            }
        } else if self.at(FshSyntaxKind::Integer) || self.at(FshSyntaxKind::Decimal) {
            // Could be: Number, Quantity, or Ratio
            // Lookahead to determine which
            let checkpoint = self.pos;
            self.add_current_token();
            self.advance();
            self.consume_trivia();

            if self.at(FshSyntaxKind::Colon) {
                // This is a Ratio: NUMBER:NUMBER or quantity:quantity
                self.pos = checkpoint;
                self.parse_ratio_value();
            } else if self.at(FshSyntaxKind::Unit) {
                // This is a Quantity value: NUMBER UNIT STRING?
                // But could also be part of a Ratio
                self.pos = checkpoint;
                let qty_checkpoint = self.pos;
                self.parse_quantity_value(true);
                self.consume_trivia();

                if self.at(FshSyntaxKind::Colon) {
                    // Actually a Ratio with quantities
                    self.pos = qty_checkpoint;
                    self.parse_ratio_value();
                }
            }
            // Otherwise, it's just a number (already consumed)
        } else if self.at(FshSyntaxKind::Ident) {
            // Could be: Reference(Type), Canonical(Type), CodeableReference(Type), identifier, or System#code
            let ident_text = self.current().map(|t| t.text.as_str()).unwrap_or("");

            if ident_text == "Reference"
                || ident_text == "Canonical"
                || ident_text == "CodeableReference"
            {
                // Reference(Type) or Canonical(Type) or CodeableReference(Type)
                self.add_current_token();
                self.advance();
                if self.at(FshSyntaxKind::LParen) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                    if self.at(FshSyntaxKind::Ident) {
                        self.add_current_token();
                        self.advance();
                    }
                    if self.at(FshSyntaxKind::RParen) {
                        self.add_current_token();
                        self.advance();
                    }
                }
            } else {
                // Identifier or System#code
                self.add_current_token();
                self.advance();

                // Check for # (System#code pattern)
                if self.at(FshSyntaxKind::Hash) {
                    self.add_current_token();
                    self.advance();
                    if self.at(FshSyntaxKind::Ident) {
                        self.add_current_token();
                        self.advance();
                    }
                    // Optional display string
                    self.consume_trivia();
                    if self.at(FshSyntaxKind::String) {
                        self.add_current_token();
                        self.advance();
                    }
                }
            }
        }
    }

    /// Parse a Quantity value: NUMBER? UNIT STRING?
    fn parse_quantity_value(&mut self, has_number: bool) {
        self.builder.start_node(FshSyntaxKind::Quantity);

        // Optional number (may already be consumed)
        if has_number && (self.at(FshSyntaxKind::Integer) || self.at(FshSyntaxKind::Decimal)) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        // Unit (required): 'mg', 'kg', etc.
        if self.at(FshSyntaxKind::Unit) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        // Optional display string
        if self.at(FshSyntaxKind::String) {
            self.add_current_token();
            self.advance();
        }

        self.builder.finish_node(); // QUANTITY
    }

    /// Parse a Ratio value: ratioPart COLON ratioPart
    /// where ratioPart can be NUMBER or Quantity
    fn parse_ratio_value(&mut self) {
        self.builder.start_node(FshSyntaxKind::Ratio);

        // Parse numerator (can be number or quantity)
        self.parse_ratio_part();
        self.consume_trivia();

        // Colon separator
        if self.at(FshSyntaxKind::Colon) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        // Parse denominator (can be number or quantity)
        self.parse_ratio_part();

        self.builder.finish_node(); // RATIO
    }

    /// Parse one part of a ratio (numerator or denominator)
    fn parse_ratio_part(&mut self) {
        // Can be: NUMBER or NUMBER UNIT
        if self.at(FshSyntaxKind::Integer) || self.at(FshSyntaxKind::Decimal) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();

            // Check for unit (making it a quantity)
            if self.at(FshSyntaxKind::Unit) {
                self.add_current_token();
                self.advance();
            }
        } else if self.at(FshSyntaxKind::Unit) {
            // Quantity without number
            self.add_current_token();
            self.advance();
        }
    }

    /// Parse a path expression (e.g., name.given, identifier[0].value, ^extension[FMM].value)
    fn parse_path(&mut self) {
        self.builder.start_node(FshSyntaxKind::Path);

        // Start with optional caret for metadata paths
        if self.at(FshSyntaxKind::Caret) {
            self.add_current_token();
            self.advance();
        }

        while !self.at_end() {
            // CRITICAL FIX: Break if we're not at an identifier
            // This prevents infinite loop when path ends with trailing dot
            if !self.at(FshSyntaxKind::Ident) {
                break;
            }

            self.add_current_token();
            self.advance();

            // Check for brackets: [index], [+], [=], [ProfileName]
            while self.at(FshSyntaxKind::LBracket) {
                self.add_current_token();
                self.advance();

                // Content can be: integer, identifier, +, =
                if self.at(FshSyntaxKind::Ident)
                    || self.at(FshSyntaxKind::Integer)
                    || self.at(FshSyntaxKind::Plus)
                    || self.at(FshSyntaxKind::Equals)
                {
                    self.add_current_token();
                    self.advance();
                }

                if self.at(FshSyntaxKind::RBracket) {
                    self.add_current_token();
                    self.advance();
                }
            }

            // Check for dot
            if self.at(FshSyntaxKind::Dot) {
                self.add_current_token();
                self.advance();
                // Continue loop to parse next segment
            } else {
                // No dot means end of path
                break;
            }
        }

        self.builder.finish_node(); // PATH
    }

    // Helper methods

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.at(FshSyntaxKind::Eof)
    }

    fn current(&self) -> Option<&CstToken> {
        self.tokens.get(self.pos)
    }

    fn current_kind(&self) -> FshSyntaxKind {
        self.current().map(|t| t.kind).unwrap_or(FshSyntaxKind::Eof)
    }

    fn at(&self, kind: FshSyntaxKind) -> bool {
        self.current_kind() == kind
    }

    fn at_trivia(&self) -> bool {
        matches!(
            self.current_kind(),
            FshSyntaxKind::Whitespace | FshSyntaxKind::CommentLine | FshSyntaxKind::CommentBlock
        )
    }

    fn at_definition_keyword(&self) -> bool {
        matches!(
            self.current_kind(),
            FshSyntaxKind::ProfileKw
                | FshSyntaxKind::ExtensionKw
                | FshSyntaxKind::ValuesetKw
                | FshSyntaxKind::CodesystemKw
                | FshSyntaxKind::InstanceKw
                | FshSyntaxKind::InvariantKw
                | FshSyntaxKind::MappingKw
                | FshSyntaxKind::LogicalKw
                | FshSyntaxKind::ResourceKw
                | FshSyntaxKind::AliasKw
                | FshSyntaxKind::RulesetKw
        )
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn add_current_token(&mut self) {
        if self.pos < self.tokens.len() {
            let token = &self.tokens[self.pos];
            self.builder.add_token(token);
        }
    }

    fn expect(&mut self, kind: FshSyntaxKind) {
        if self.at(kind) {
            self.add_current_token();
            self.advance();
        } else {
            // Create error token
            self.builder.token(FshSyntaxKind::Error, "");
        }
    }

    fn consume_trivia(&mut self) {
        while self.at_trivia() && !self.at(FshSyntaxKind::Newline) {
            self.add_current_token();
            self.advance();
        }
    }

    fn consume_trivia_and_newlines(&mut self) {
        while self.at_trivia() || self.at(FshSyntaxKind::Newline) {
            self.add_current_token();
            self.advance();
        }
    }

    #[allow(dead_code)]
    fn skip_until_definition(&mut self) {
        while !self.at_end() && !self.at_definition_keyword() {
            self.add_current_token();
            self.advance();
        }
    }

    /// Parse a Logical/Resource rule (can be addElementRule or standard sdRule)
    fn parse_lr_rule(&mut self) {
        self.expect(FshSyntaxKind::Asterisk);
        self.consume_trivia();

        // Parse path
        self.parse_path();
        self.consume_trivia();

        // Check if this is an addElementRule by looking for cardinality pattern
        // AddElementRule: * path CARD flags* TYPE STRING
        // CardRule: * path CARD flags* (no TYPE)
        // OtherRule: * path KEYWORD

        if self.at(FshSyntaxKind::Integer) {
            // Might be addElementRule, addCRElementRule, or cardRule
            // We need to look ahead to distinguish:
            // - If we see contentreference after cardinality/flags -> addCRElementRule
            // - If we see TYPE (Ident that looks like a type) after cardinality/flags -> addElementRule
            // - Otherwise -> cardRule

            if self.is_add_cr_element_rule() {
                // It's an addCRElementRule
                self.parse_add_cr_element_rule();
                return;
            } else if self.is_add_element_rule() {
                // It's an addElementRule - rebuild it with proper node
                self.builder.start_node(FshSyntaxKind::AddElementRule);

                // Cardinality (min..max)
                self.expect(FshSyntaxKind::Integer); // min
                self.consume_trivia();
                self.expect(FshSyntaxKind::Range); // ..
                self.consume_trivia();
                if self.at(FshSyntaxKind::Integer) || self.at(FshSyntaxKind::Asterisk) {
                    self.add_current_token(); // max
                    self.advance();
                }
                self.consume_trivia();

                // Flags (MS, SU, etc.)
                while self.at(FshSyntaxKind::MsFlag)
                    || self.at(FshSyntaxKind::SuFlag)
                    || self.at(FshSyntaxKind::TuFlag)
                    || self.at(FshSyntaxKind::NFlag)
                    || self.at(FshSyntaxKind::DFlag)
                {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                }

                // Target type(s) - at least one required
                self.expect(FshSyntaxKind::Ident); // First type
                self.consume_trivia();

                // Additional types with "or"
                while self.at(FshSyntaxKind::OrKw) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                    self.expect(FshSyntaxKind::Ident);
                    self.consume_trivia();
                }

                // Short description (required)
                self.expect(FshSyntaxKind::String);
                self.consume_trivia();

                // Full definition (optional)
                if self.at(FshSyntaxKind::String) {
                    self.add_current_token();
                    self.advance();
                }

                self.consume_trivia_and_newlines();
                self.builder.finish_node(); // ADD_ELEMENT_RULE
                return;
            }
        }

        // Not an addElementRule - parse as standard sdRule
        // We need to reparse this as a standard rule
        // The path is already consumed, so we continue from here
        self.parse_standard_sd_rule();
    }

    /// Check if current position indicates an addElementRule
    /// Returns true if we see: CARD flags* TYPE STRING
    fn is_add_element_rule(&self) -> bool {
        let mut peek_pos = self.pos;

        // Should be at INTEGER for cardinality
        if peek_pos >= self.tokens.len() || self.tokens[peek_pos].kind != FshSyntaxKind::Integer {
            return false;
        }
        peek_pos += 1;

        // Skip whitespace
        while peek_pos < self.tokens.len()
            && self.tokens[peek_pos].kind == FshSyntaxKind::Whitespace
        {
            peek_pos += 1;
        }

        // Should see Range (..)
        if peek_pos >= self.tokens.len() || self.tokens[peek_pos].kind != FshSyntaxKind::Range {
            return false;
        }
        peek_pos += 1;

        // Skip whitespace
        while peek_pos < self.tokens.len()
            && self.tokens[peek_pos].kind == FshSyntaxKind::Whitespace
        {
            peek_pos += 1;
        }

        // Should see max (INTEGER or ASTERISK)
        if peek_pos >= self.tokens.len() {
            return false;
        }
        if self.tokens[peek_pos].kind != FshSyntaxKind::Integer
            && self.tokens[peek_pos].kind != FshSyntaxKind::Asterisk
        {
            return false;
        }
        peek_pos += 1;

        // Skip whitespace
        while peek_pos < self.tokens.len()
            && self.tokens[peek_pos].kind == FshSyntaxKind::Whitespace
        {
            peek_pos += 1;
        }

        // Skip flags (MS, SU, TU, N, D)
        while peek_pos < self.tokens.len() {
            let kind = self.tokens[peek_pos].kind;
            if matches!(
                kind,
                FshSyntaxKind::MsFlag
                    | FshSyntaxKind::SuFlag
                    | FshSyntaxKind::TuFlag
                    | FshSyntaxKind::NFlag
                    | FshSyntaxKind::DFlag
            ) {
                peek_pos += 1;
                // Skip whitespace after flag
                while peek_pos < self.tokens.len()
                    && self.tokens[peek_pos].kind == FshSyntaxKind::Whitespace
                {
                    peek_pos += 1;
                }
            } else {
                break;
            }
        }

        // Now we should see either:
        // - IDENT (type name) followed by STRING -> addElementRule
        // - Newline or EOF -> cardRule

        if peek_pos >= self.tokens.len() {
            return false;
        }

        // If we see IDENT followed by STRING, it's likely addElementRule
        if self.tokens[peek_pos].kind == FshSyntaxKind::Ident {
            // Peek ahead for STRING
            let mut next_pos = peek_pos + 1;
            while next_pos < self.tokens.len()
                && self.tokens[next_pos].kind == FshSyntaxKind::Whitespace
            {
                next_pos += 1;
            }

            // Check if next is STRING or OrKw (multiple types) or another IDENT
            if next_pos < self.tokens.len() {
                let next_kind = self.tokens[next_pos].kind;
                if next_kind == FshSyntaxKind::String || next_kind == FshSyntaxKind::OrKw {
                    return true; // Definitely addElementRule
                }
            }
        }

        false
    }

    /// Check if current position indicates an addCRElementRule
    /// Returns true if we see: CARD flags* contentreference REF STRING
    fn is_add_cr_element_rule(&self) -> bool {
        let mut peek_pos = self.pos;

        // Should be at INTEGER for cardinality
        if peek_pos >= self.tokens.len() || self.tokens[peek_pos].kind != FshSyntaxKind::Integer {
            return false;
        }
        peek_pos += 1;

        // Skip whitespace
        while peek_pos < self.tokens.len()
            && self.tokens[peek_pos].kind == FshSyntaxKind::Whitespace
        {
            peek_pos += 1;
        }

        // Should see Range (..)
        if peek_pos >= self.tokens.len() || self.tokens[peek_pos].kind != FshSyntaxKind::Range {
            return false;
        }
        peek_pos += 1;

        // Skip whitespace
        while peek_pos < self.tokens.len()
            && self.tokens[peek_pos].kind == FshSyntaxKind::Whitespace
        {
            peek_pos += 1;
        }

        // Should see max (INTEGER or ASTERISK)
        if peek_pos >= self.tokens.len() {
            return false;
        }
        if self.tokens[peek_pos].kind != FshSyntaxKind::Integer
            && self.tokens[peek_pos].kind != FshSyntaxKind::Asterisk
        {
            return false;
        }
        peek_pos += 1;

        // Skip whitespace
        while peek_pos < self.tokens.len()
            && self.tokens[peek_pos].kind == FshSyntaxKind::Whitespace
        {
            peek_pos += 1;
        }

        // Skip flags (MS, SU, TU, N, D)
        while peek_pos < self.tokens.len() {
            let kind = self.tokens[peek_pos].kind;
            if matches!(
                kind,
                FshSyntaxKind::MsFlag
                    | FshSyntaxKind::SuFlag
                    | FshSyntaxKind::TuFlag
                    | FshSyntaxKind::NFlag
                    | FshSyntaxKind::DFlag
            ) {
                peek_pos += 1;
                // Skip whitespace after flag
                while peek_pos < self.tokens.len()
                    && self.tokens[peek_pos].kind == FshSyntaxKind::Whitespace
                {
                    peek_pos += 1;
                }
            } else {
                break;
            }
        }

        // Should see contentreference keyword
        if peek_pos >= self.tokens.len()
            || self.tokens[peek_pos].kind != FshSyntaxKind::ContentreferenceKw
        {
            return false;
        }

        true
    }

    /// Parse addCRElementRule: * path CARD flags* contentreference REF STRING STRING?
    fn parse_add_cr_element_rule(&mut self) {
        self.builder.start_node(FshSyntaxKind::AddCRElementRule);

        // Cardinality (min..max)
        self.expect(FshSyntaxKind::Integer); // min
        self.consume_trivia();
        self.expect(FshSyntaxKind::Range); // ..
        self.consume_trivia();
        if self.at(FshSyntaxKind::Integer) || self.at(FshSyntaxKind::Asterisk) {
            self.add_current_token(); // max
            self.advance();
        }
        self.consume_trivia();

        // Flags (MS, SU, etc.)
        while self.at(FshSyntaxKind::MsFlag)
            || self.at(FshSyntaxKind::SuFlag)
            || self.at(FshSyntaxKind::TuFlag)
            || self.at(FshSyntaxKind::NFlag)
            || self.at(FshSyntaxKind::DFlag)
        {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        // contentreference keyword
        self.expect(FshSyntaxKind::ContentreferenceKw);
        self.consume_trivia();

        // Reference URL or code (can be URL or #code)
        if self.at(FshSyntaxKind::Ident) {
            // URL like http://example.org/StructureDefinition/Type
            self.add_current_token();
            self.advance();
        } else if self.at(FshSyntaxKind::Hash) {
            // Code like #LocalType
            self.add_current_token();
            self.advance();
            if self.at(FshSyntaxKind::Ident) {
                self.add_current_token();
                self.advance();
            }
        }
        self.consume_trivia();

        // Short description (required)
        self.expect(FshSyntaxKind::String);
        self.consume_trivia();

        // Full definition (optional)
        if self.at(FshSyntaxKind::String) {
            self.add_current_token();
            self.advance();
        }

        self.consume_trivia_and_newlines();
        self.builder.finish_node(); // ADD_CR_ELEMENT_RULE
    }

    /// Parse the rest of a standard sdRule after path has been consumed
    fn parse_standard_sd_rule(&mut self) {
        // Determine rule type based on what follows the path
        let rule_kind = if self.at(FshSyntaxKind::Arrow) {
            FshSyntaxKind::MappingRule
        } else if self.at(FshSyntaxKind::Equals) || self.at(FshSyntaxKind::PlusEquals) {
            FshSyntaxKind::FixedValueRule
        } else if self.at(FshSyntaxKind::ContainsKw) {
            FshSyntaxKind::ContainsRule
        } else if self.at(FshSyntaxKind::FromKw) {
            FshSyntaxKind::ValuesetRule
        } else if self.at(FshSyntaxKind::OnlyKw) {
            FshSyntaxKind::OnlyRule
        } else if self.at(FshSyntaxKind::ObeysKw) {
            FshSyntaxKind::ObeysRule
        } else {
            FshSyntaxKind::CardRule // Default: cardinality/flags
        };

        self.builder.start_node(rule_kind);

        // Delegate to the appropriate parsing logic from parse_rule()
        self.parse_sd_rule_body(rule_kind);

        self.consume_trivia_and_newlines();
        self.builder.finish_node();
    }

    /// Parse the body of a standard SD rule
    fn parse_sd_rule_body(&mut self, rule_kind: FshSyntaxKind) {
        match rule_kind {
            FshSyntaxKind::MappingRule => {
                self.expect(FshSyntaxKind::Arrow);
                self.consume_trivia();
                self.expect(FshSyntaxKind::String);
                self.consume_trivia();
                if self.at(FshSyntaxKind::String) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                }
                if self.at(FshSyntaxKind::Hash) {
                    self.add_current_token();
                    self.advance();
                    if self.at(FshSyntaxKind::Ident) {
                        self.add_current_token();
                        self.advance();
                    }
                }
            }
            FshSyntaxKind::FixedValueRule => {
                if self.at(FshSyntaxKind::Equals) || self.at(FshSyntaxKind::PlusEquals) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                    self.parse_value_expression();
                }
            }
            FshSyntaxKind::CardRule => {
                // Cardinality and flags
                if self.at(FshSyntaxKind::Integer) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                    if self.at(FshSyntaxKind::Range) {
                        self.add_current_token();
                        self.advance();
                        self.consume_trivia();
                        if self.at(FshSyntaxKind::Integer) || self.at(FshSyntaxKind::Asterisk) {
                            self.add_current_token();
                            self.advance();
                            self.consume_trivia();
                        }
                    }
                }
                // Flags
                while self.at(FshSyntaxKind::MsFlag)
                    || self.at(FshSyntaxKind::SuFlag)
                    || self.at(FshSyntaxKind::TuFlag)
                {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();
                }
            }
            _ => {
                // For other rules (contains, from, only, obeys), just consume tokens until newline
                while !self.at_end() && !self.at(FshSyntaxKind::Newline) {
                    self.add_current_token();
                    self.advance();
                }
            }
        }
    }

    /// Parse a CodeSystem concept definition
    fn parse_concept(&mut self) {
        self.builder.start_node(FshSyntaxKind::Concept);

        self.expect(FshSyntaxKind::Asterisk);
        self.consume_trivia();

        // One or more CODE tokens (for hierarchy)
        // Format: * #code or * #parent #child
        while self.at(FshSyntaxKind::Hash) {
            self.add_current_token(); // #
            self.advance();
            self.consume_trivia();

            // Code identifier after #
            if self.at(FshSyntaxKind::Ident) || self.at(FshSyntaxKind::Integer) {
                self.add_current_token();
                self.advance();
                self.consume_trivia();
            }
        }

        // Optional display string
        if self.at(FshSyntaxKind::String) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        // Optional definition string (second string)
        if self.at(FshSyntaxKind::String) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        self.consume_trivia_and_newlines();
        self.builder.finish_node(); // CONCEPT
    }

    /// Parse a ValueSet rule (include/exclude components)
    fn parse_vs_rule(&mut self) {
        self.expect(FshSyntaxKind::Asterisk);
        self.consume_trivia();

        // Check for optional include/exclude keywords
        let has_include_exclude =
            self.at(FshSyntaxKind::IncludeKw) || self.at(FshSyntaxKind::ExcludeKw);

        if has_include_exclude {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        // Determine if this is concept component or filter component
        if self.at(FshSyntaxKind::CodesKw) {
            // Filter component: * include codes from system ...
            self.parse_vs_filter_component();
        } else {
            // Concept component: * include http://system#code "display"
            // or just: * http://system#code (implicit include)
            self.parse_vs_concept_component();
        }
    }

    /// Parse ValueSet concept component: system#code "display"
    fn parse_vs_concept_component(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsConceptComponent);

        // Parse code reference (can be URL#code or just #code)
        self.parse_vs_code();
        self.consume_trivia();

        // Optional display string
        if self.at(FshSyntaxKind::String) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        // Optional "from" clause
        if self.at(FshSyntaxKind::FromKw) {
            self.parse_vs_component_from();
        }

        self.consume_trivia_and_newlines();
        self.builder.finish_node(); // VS_CONCEPT_COMPONENT
    }

    /// Parse a code reference for ValueSet (can be URL#code)
    fn parse_vs_code(&mut self) {
        self.builder.start_node(FshSyntaxKind::CodeRef);

        // Parse system part (if present) - can be URL or identifier
        // URLs like http://loinc.org are multiple tokens
        let mut has_system = false;

        while !self.at_end()
            && !self.at(FshSyntaxKind::Hash)
            && !self.at(FshSyntaxKind::String)
            && !self.at(FshSyntaxKind::Newline)
            && !self.at(FshSyntaxKind::FromKw)
        {
            if self.at(FshSyntaxKind::Whitespace) {
                break;
            }
            self.add_current_token();
            self.advance();
            has_system = true;
        }

        // Hash separator
        if self.at(FshSyntaxKind::Hash) {
            self.add_current_token();
            self.advance();

            // Code part (after #)
            if self.at(FshSyntaxKind::Ident) || self.at(FshSyntaxKind::Integer) {
                self.add_current_token();
                self.advance();
            }
        } else if !has_system {
            // No system and no hash - might be just a code starting with #
            // This case is handled by the hash check above
        }

        self.builder.finish_node(); // CODE_REF
    }

    /// Parse ValueSet filter component: codes from system ... where ...
    fn parse_vs_filter_component(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsFilterComponent);

        // "codes" keyword
        self.expect(FshSyntaxKind::CodesKw);
        self.consume_trivia();

        // "from" clause (required for filter component)
        if self.at(FshSyntaxKind::FromKw) {
            self.parse_vs_component_from();
        }

        // Optional "where" clause with filters
        if self.at(FshSyntaxKind::WhereKw) {
            self.parse_vs_where_clause();
        }

        self.consume_trivia_and_newlines();
        self.builder.finish_node(); // VS_FILTER_COMPONENT
    }

    /// Parse "from system X and valueset Y" clause
    fn parse_vs_component_from(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsComponentFrom);

        self.expect(FshSyntaxKind::FromKw);
        self.consume_trivia();

        // Can be "system X" or "valueset Y" or both with "and"
        loop {
            if self.at(FshSyntaxKind::SystemKw) {
                self.parse_vs_from_system();
            } else if self.at(FshSyntaxKind::ValuesetRefKw) {
                self.parse_vs_from_valueset();
            } else {
                break;
            }

            self.consume_trivia();

            // Check for "and" to continue
            if self.at(FshSyntaxKind::AndKw) {
                self.add_current_token();
                self.advance();
                self.consume_trivia();
            } else {
                break;
            }
        }

        self.builder.finish_node(); // VS_COMPONENT_FROM
    }

    /// Parse "system URL" part
    fn parse_vs_from_system(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsFromSystem);

        self.expect(FshSyntaxKind::SystemKw);
        self.consume_trivia();

        // System URL or name - consume tokens until we hit a keyword or newline
        // URLs are multiple tokens, so we need to consume them all
        while !self.at_end()
            && !self.at(FshSyntaxKind::Newline)
            && !self.at(FshSyntaxKind::AndKw)
            && !self.at(FshSyntaxKind::WhereKw)
            && !self.at(FshSyntaxKind::Whitespace)
        {
            self.add_current_token();
            self.advance();
        }

        self.builder.finish_node(); // VS_FROM_SYSTEM
    }

    /// Parse "valueset URL" part
    fn parse_vs_from_valueset(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsFromValueset);

        self.expect(FshSyntaxKind::ValuesetRefKw);
        self.consume_trivia();

        // ValueSet URL or name
        while !self.at_end()
            && !self.at(FshSyntaxKind::Newline)
            && !self.at(FshSyntaxKind::AndKw)
            && !self.at(FshSyntaxKind::Whitespace)
        {
            self.add_current_token();
            self.advance();
        }

        self.builder.finish_node(); // VS_FROM_VALUESET
    }

    /// Parse "where" filter list
    fn parse_vs_where_clause(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsFilterList);

        self.expect(FshSyntaxKind::WhereKw);
        self.consume_trivia();

        // Parse filter definitions separated by "and"
        loop {
            self.parse_vs_filter_definition();
            self.consume_trivia();

            if self.at(FshSyntaxKind::AndKw) {
                self.add_current_token();
                self.advance();
                self.consume_trivia();
            } else {
                break;
            }
        }

        self.builder.finish_node(); // VS_FILTER_LIST
    }

    /// Parse single filter: property operator value
    fn parse_vs_filter_definition(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsFilterDefinition);

        // Property name (e.g., "concept", "designation")
        if self.at(FshSyntaxKind::Ident) {
            self.add_current_token();
            self.advance();
            self.consume_trivia();
        }

        // Operator (e.g., "is-a", "descendent-of", "=")
        self.parse_vs_filter_operator();
        self.consume_trivia();

        // Value (optional for some operators)
        if !self.at_end() && !self.at(FshSyntaxKind::AndKw) && !self.at(FshSyntaxKind::Newline) {
            self.parse_vs_filter_value();
        }

        self.builder.finish_node(); // VS_FILTER_DEFINITION
    }

    /// Parse filter operator
    fn parse_vs_filter_operator(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsFilterOperator);

        if self.at(FshSyntaxKind::Equals) {
            self.add_current_token();
            self.advance();
        } else if self.at(FshSyntaxKind::Ident) {
            // Operators like "is-a", "descendent-of" are lexed as IDENT
            // They may contain hyphens, so consume the full identifier
            self.add_current_token();
            self.advance();
        }

        self.builder.finish_node(); // VS_FILTER_OPERATOR
    }

    /// Parse filter value
    fn parse_vs_filter_value(&mut self) {
        self.builder.start_node(FshSyntaxKind::VsFilterValue);

        if self.at(FshSyntaxKind::Hash) {
            // Code: #12345
            self.add_current_token();
            self.advance();
            if self.at(FshSyntaxKind::Ident) || self.at(FshSyntaxKind::Integer) {
                self.add_current_token();
                self.advance();
            }
        } else if self.at(FshSyntaxKind::True) || self.at(FshSyntaxKind::False) {
            // Boolean
            self.add_current_token();
            self.advance();
        } else if self.at(FshSyntaxKind::String) {
            // String
            self.add_current_token();
            self.advance();
        } else if self.at(FshSyntaxKind::Slash) {
            // Potential regex /pattern/ - for now just consume as slash
            // TODO: Implement proper regex parsing in lexer
            self.add_current_token();
            self.advance();
        } else {
            // Default: consume as identifier or other token
            self.add_current_token();
            self.advance();
        }

        self.builder.finish_node(); // VS_FILTER_VALUE
    }

    fn error_and_recover(&mut self) {
        // Consume the error token
        self.builder.start_node(FshSyntaxKind::Error);
        self.add_current_token();
        self.advance();
        self.builder.finish_node();

        // Skip to next line
        while !self.at_end() && !self.at(FshSyntaxKind::Newline) {
            self.add_current_token();
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_profile() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (cst, errors) = parse_fsh(source);

        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);

        // Find Profile node
        let profile = cst.children().find(|n| n.kind() == FshSyntaxKind::Profile);
        assert!(profile.is_some());
    }

    #[test]
    fn test_parse_profile_with_metadata() {
        let source = r#"Profile: MyPatient
Parent: Patient
Id: my-patient
Title: "My Patient Profile"
Description: "A test profile""#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);

        // Verify structure
        let profile = cst
            .children()
            .find(|n| n.kind() == FshSyntaxKind::Profile)
            .unwrap();

        // Should have Parent, Id, Title, Description clauses
        assert!(
            profile
                .children()
                .any(|n| n.kind() == FshSyntaxKind::ParentClause)
        );
        assert!(
            profile
                .children()
                .any(|n| n.kind() == FshSyntaxKind::IdClause)
        );
        assert!(
            profile
                .children()
                .any(|n| n.kind() == FshSyntaxKind::TitleClause)
        );
        assert!(
            profile
                .children()
                .any(|n| n.kind() == FshSyntaxKind::DescriptionClause)
        );
    }

    #[test]
    fn test_parse_profile_with_rules() {
        let source = r#"Profile: MyPatient
Parent: Patient
* name 1..1 MS
* birthDate 0..1"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);

        let profile = cst
            .children()
            .find(|n| n.kind() == FshSyntaxKind::Profile)
            .unwrap();

        // Should have 2 rules
        let rules: Vec<_> = profile
            .children()
            .filter(|n| n.kind() == FshSyntaxKind::CardRule)
            .collect();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_parse_multiple_definitions() {
        let source = r#"Profile: Profile1
Parent: Patient

Extension: Extension1
Id: ext-1"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);

        // Should have both Profile and Extension
        assert!(cst.children().any(|n| n.kind() == FshSyntaxKind::Profile));
        assert!(cst.children().any(|n| n.kind() == FshSyntaxKind::Extension));
    }

    #[test]
    fn test_parse_with_comments() {
        let source = r#"// This is a profile
Profile: MyPatient // inline comment
Parent: Patient"#;

        let (cst, errors) = parse_fsh(source);
        assert!(errors.is_empty());
        assert_eq!(cst.text().to_string(), source);

        // Comments should be preserved
        assert!(cst.text().to_string().contains("// This is a profile"));
        assert!(cst.text().to_string().contains("// inline comment"));
    }

    #[test]
    fn test_lossless_complex_example() {
        let source = r#"Profile:  MyPatient   // Extra spaces!
Parent: Patient
Id: my-patient
Title: "My Patient"

* name 1..1 MS
* birthDate 0..1"#;

        let (cst, _) = parse_fsh(source);

        // Perfect lossless roundtrip
        assert_eq!(cst.text().to_string(), source);
    }
}
