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
/// use fsh_lint_core::cst::parse_fsh;
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

        while !self.at_end() {
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
                FshSyntaxKind::RulesetKw => self.parse_ruleset(),
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
                FshSyntaxKind::Asterisk => self.parse_rule(),

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
                FshSyntaxKind::Asterisk => self.parse_rule(),
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

        // TODO: Parse ValueSet-specific rules
        self.skip_until_definition();

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

        // TODO: Parse CodeSystem concepts
        self.skip_until_definition();

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
                FshSyntaxKind::Asterisk => self.parse_rule(),

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

            // Alias value (URL or identifier)
            if self.at(FshSyntaxKind::Ident) || self.at(FshSyntaxKind::String) {
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
            self.add_current_token();
            self.advance();
            self.consume_trivia();

            // Parse parameter list
            while !self.at_end() && !self.at(FshSyntaxKind::RParen) {
                if self.at(FshSyntaxKind::Ident) {
                    self.add_current_token();
                    self.advance();
                    self.consume_trivia();

                    // Comma separator
                    if self.at(FshSyntaxKind::Comma) {
                        self.add_current_token();
                        self.advance();
                        self.consume_trivia();
                    }
                } else {
                    break;
                }
            }

            if self.at(FshSyntaxKind::RParen) {
                self.add_current_token();
                self.advance();
            }
        }

        self.consume_trivia_and_newlines();

        // Parse rules in the RuleSet body
        while !self.at_end() && !self.at_definition_keyword() {
            if self.at_trivia() {
                self.consume_trivia();
                continue;
            }

            match self.current_kind() {
                FshSyntaxKind::Asterisk => self.parse_rule(),
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

        self.builder.finish_node(); // RULE_SET
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
                FshSyntaxKind::Asterisk => self.parse_rule(),
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
                FshSyntaxKind::Asterisk => self.parse_rule(),
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
                FshSyntaxKind::Asterisk => self.parse_rule(),
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
        self.expect(FshSyntaxKind::Ident);
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
                self.add_current_token();
                self.advance();
                self.consume_trivia();

                // Parse arguments
                while !self.at_end() && !self.at(FshSyntaxKind::RParen) {
                    // Argument can be identifier, path, or string
                    if self.at(FshSyntaxKind::Ident)
                        || self.at(FshSyntaxKind::String)
                        || self.at(FshSyntaxKind::Integer)
                    {
                        self.add_current_token();
                        self.advance();
                        self.consume_trivia();

                        if self.at(FshSyntaxKind::Comma) {
                            self.add_current_token();
                            self.advance();
                            self.consume_trivia();
                        }
                    } else if self.at(FshSyntaxKind::LBrace) {
                        // Template parameter {path}
                        self.add_current_token();
                        self.advance();
                        if self.at(FshSyntaxKind::Ident) {
                            self.add_current_token();
                            self.advance();
                        }
                        if self.at(FshSyntaxKind::RBrace) {
                            self.add_current_token();
                            self.advance();
                        }
                        self.consume_trivia();
                    } else {
                        break;
                    }
                }

                if self.at(FshSyntaxKind::RParen) {
                    self.add_current_token();
                    self.advance();
                }
            }

            self.consume_trivia_and_newlines();
            self.builder.finish_node();
            return;
        }

        // Parse path
        self.parse_path();
        self.consume_trivia();

        // Determine rule type based on what follows the path
        let rule_kind = if self.at(FshSyntaxKind::Equals) || self.at(FshSyntaxKind::PlusEquals) {
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
        } else if self.at(FshSyntaxKind::Integer) || self.at(FshSyntaxKind::Decimal) {
            // Number
            self.add_current_token();
            self.advance();
        } else if self.at(FshSyntaxKind::Ident) {
            // Could be: Reference(Type), Canonical(Type), identifier, or System#code
            let ident_text = self.current().map(|t| t.text.as_str()).unwrap_or("");

            if ident_text == "Reference" || ident_text == "Canonical" {
                // Reference(Type) or Canonical(Type)
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

    /// Parse a path expression (e.g., name.given, identifier[0].value, ^extension[FMM].value)
    fn parse_path(&mut self) {
        self.builder.start_node(FshSyntaxKind::Path);

        // Start with optional caret for metadata paths
        if self.at(FshSyntaxKind::Caret) {
            self.add_current_token();
            self.advance();
        }

        while !self.at_end() {
            if self.at(FshSyntaxKind::Ident) {
                self.add_current_token();
                self.advance();

                // Check for brackets: [index], [+], [=], [ProfileName]
                while self.at(FshSyntaxKind::LBracket) {
                    self.add_current_token();
                    self.advance();

                    // Content can be: integer, identifier, +, =
                    if self.at(FshSyntaxKind::Ident) || self.at(FshSyntaxKind::Integer) {
                        self.add_current_token();
                        self.advance();
                    } else if self.at(FshSyntaxKind::Plus) {
                        self.add_current_token();
                        self.advance();
                    } else if self.at(FshSyntaxKind::Equals) {
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
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        self.builder.finish_node();
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

    fn skip_until_definition(&mut self) {
        while !self.at_end() && !self.at_definition_keyword() {
            self.add_current_token();
            self.advance();
        }
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
