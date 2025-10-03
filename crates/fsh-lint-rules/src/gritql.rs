//! GritQL pattern compilation and execution

use fsh_lint_core::{Diagnostic, FshLintError, Location, Result, Severity};
use std::collections::HashMap;
use std::path::PathBuf;
use tree_sitter::{Node, Tree};

/// A compiled GritQL pattern ready for execution
#[derive(Debug, Clone)]
pub struct CompiledGritQLPattern {
    /// The original pattern string
    pub pattern: String,
    /// Rule ID for error reporting
    pub rule_id: String,
    /// Variable captures from the pattern
    captures: Vec<String>,
    /// Compiled pattern state (placeholder for now)
    _compiled_state: (),
}

/// Range information for a match
#[derive(Debug, Clone)]
pub struct MatchRange {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// Result of executing a GritQL pattern
#[derive(Debug, Clone)]
pub struct GritQLMatch {
    /// The matched node range
    pub range: MatchRange,
    /// Captured variables and their values
    pub captures: HashMap<String, String>,
    /// The matched text
    pub matched_text: String,
}

/// Compiler for GritQL patterns
pub struct GritQLCompiler {
    // For now, this is a simple compiler without external dependencies
}

impl GritQLCompiler {
    /// Create a new GritQL compiler for FSH
    pub fn new() -> Result<Self> {
        // For now, create a simple compiler without external dependencies
        // TODO: Integrate actual GritQL library once API is stable
        Ok(Self {})
    }

    /// Compile a GritQL pattern string into an executable pattern
    pub fn compile_pattern(&self, pattern: &str, rule_id: &str) -> Result<CompiledGritQLPattern> {
        // Validate the pattern
        if pattern.trim().is_empty() {
            return Err(FshLintError::rule_error(
                rule_id,
                "GritQL pattern cannot be empty",
            ));
        }

        // Basic pattern validation
        self.validate_pattern_syntax(pattern, rule_id)?;

        // Extract variable captures from the pattern
        let captures = self.extract_captures_from_pattern(pattern);

        Ok(CompiledGritQLPattern {
            pattern: pattern.to_string(),
            rule_id: rule_id.to_string(),
            captures,
            _compiled_state: (),
        })
    }

    /// Validate basic GritQL pattern syntax
    fn validate_pattern_syntax(&self, pattern: &str, rule_id: &str) -> Result<()> {
        // Basic syntax validation
        let balanced_braces = pattern.chars().fold(0i32, |acc, c| match c {
            '{' => acc + 1,
            '}' => acc - 1,
            _ => acc,
        });

        if balanced_braces != 0 {
            return Err(FshLintError::rule_error(
                rule_id,
                "Unbalanced braces in GritQL pattern",
            ));
        }

        let balanced_parens = pattern.chars().fold(0i32, |acc, c| match c {
            '(' => acc + 1,
            ')' => acc - 1,
            _ => acc,
        });

        if balanced_parens != 0 {
            return Err(FshLintError::rule_error(
                rule_id,
                "Unbalanced parentheses in GritQL pattern",
            ));
        }

        Ok(())
    }

    /// Extract variable captures from a pattern string
    fn extract_captures_from_pattern(&self, pattern: &str) -> Vec<String> {
        let mut captures = Vec::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                // Found a variable, extract the name
                let mut var_name = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        var_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if !var_name.is_empty() && !captures.contains(&var_name) {
                    captures.push(var_name);
                }
            }
        }

        captures
    }
}

impl CompiledGritQLPattern {
    /// Execute this pattern against a syntax tree
    pub fn execute(&self, tree: &Tree, source: &str, _rule_id: &str) -> Result<Vec<GritQLMatch>> {
        let mut matches = Vec::new();

        // For now, implement a basic pattern matching system
        // This will be replaced with actual GritQL integration later
        let root_node = tree.root_node();
        self.execute_on_node(root_node, source, &mut matches)?;

        Ok(matches)
    }

    /// Execute the pattern on a specific node (basic implementation)
    fn execute_on_node(
        &self,
        node: Node,
        source: &str,
        matches: &mut Vec<GritQLMatch>,
    ) -> Result<()> {
        // Basic pattern matching - for now, just match node types
        // This is a placeholder implementation that will be replaced with actual GritQL

        let node_kind = node.kind();

        // Simple pattern matching based on node type
        let pattern_matches = match self.pattern.as_str() {
            // Match profile definitions
            "profile_definition" | "Profile" => node_kind == "profile_definition",
            // Match extension definitions
            "extension_definition" | "Extension" => node_kind == "extension_definition",
            // Match value set definitions
            "valueset_definition" | "ValueSet" => node_kind == "valueset_definition",
            // Match any identifier
            "identifier" => node_kind == "identifier",
            // Match any string literal
            "string" => node_kind == "string",
            // Default: check if pattern is contained in node kind
            _ => node_kind.contains(&self.pattern.to_lowercase()),
        };

        if pattern_matches {
            let range = MatchRange {
                start_line: node.start_position().row + 1,
                start_column: node.start_position().column + 1,
                end_line: node.end_position().row + 1,
                end_column: node.end_position().column + 1,
            };

            let matched_text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();

            // For now, create empty captures - will be implemented with actual GritQL
            let captures = HashMap::new();

            matches.push(GritQLMatch {
                range,
                captures,
                matched_text,
            });
        }

        // Recursively check child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.execute_on_node(child, source, matches)?;
        }

        Ok(())
    }

    /// Get the original pattern string
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Get the variable captures defined in this pattern
    pub fn captures(&self) -> &[String] {
        &self.captures
    }

    /// Get the rule ID
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }
}

impl Default for GritQLCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create default GritQL compiler")
    }
}

/// Convert GritQL matches to diagnostics
pub fn matches_to_diagnostics(
    matches: Vec<GritQLMatch>,
    rule_id: &str,
    severity: Severity,
    message_template: &str,
    file_path: &str,
) -> Vec<Diagnostic> {
    matches
        .into_iter()
        .map(|m| {
            // Replace variables in message template with captured values
            let mut message = message_template.to_string();
            for (var, value) in &m.captures {
                message = message.replace(&format!("${}", var), value);
            }

            Diagnostic {
                rule_id: rule_id.to_string(),
                severity,
                message,
                location: Location {
                    file: PathBuf::from(file_path),
                    line: m.range.start_line,
                    column: m.range.start_column,
                    end_line: Some(m.range.end_line),
                    end_column: Some(m.range.end_column),
                    offset: 0, // TODO: Calculate actual offset
                    length: m.matched_text.len(),
                    span: None,
                },
                suggestions: Vec::new(),
                code_snippet: Some(m.matched_text),
                code: None,
                source: Some("gritql".to_string()),
                category: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tree creation test removed for now since we don't need it for basic pattern compilation tests

    #[test]
    fn test_gritql_compiler_creation() {
        let compiler = GritQLCompiler::new();
        assert!(compiler.is_ok());
    }

    #[test]
    fn test_pattern_compilation() {
        let compiler = GritQLCompiler::new().unwrap();

        // Test with a simple pattern
        let pattern = "profile_definition";
        let result = compiler.compile_pattern(pattern, "test-rule");

        // Should successfully compile basic patterns
        assert!(result.is_ok());

        let compiled = result.unwrap();
        assert_eq!(compiled.pattern(), pattern);
        assert_eq!(compiled.rule_id(), "test-rule");
    }

    #[test]
    fn test_matches_to_diagnostics() {
        let matches = vec![GritQLMatch {
            range: MatchRange {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
            },
            captures: {
                let mut map = HashMap::new();
                map.insert("name".to_string(), "TestProfile".to_string());
                map
            },
            matched_text: "TestProfile".to_string(),
        }];

        let diagnostics = matches_to_diagnostics(
            matches,
            "test-rule",
            Severity::Warning,
            "Profile $name has an issue",
            "test.fsh",
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Profile TestProfile has an issue");
        assert_eq!(diagnostics[0].rule_id, "test-rule");
    }
}
