//! Invariant Processor
//!
//! Converts FSH Invariant definitions to FHIR ElementDefinitionConstraint entries.
//!
//! # Overview
//!
//! FSH Invariants define constraints with FHIRPath expressions that can be applied
//! to elements via ObeysRule. The InvariantProcessor converts Invariant AST nodes
//! to ElementDefinitionConstraint structures.
//!
//! # Example
//!
//! ```fsh
//! Invariant: inv-1
//! Description: "SHALL have a contact party or an organization or both"
//! Expression: "telecom or name"
//! Severity: #error
//!
//! Profile: MyProfile
//! * telecom obeys inv-1
//! ```

use crate::cst::ast::Invariant;
use crate::export::fhir_types::ElementDefinitionConstraint;
use tracing::{debug, warn};

// ============================================================================
// Invariant Processor
// ============================================================================

/// Processes FSH Invariant definitions into FHIR constraints
///
/// # Example
///
/// ```rust,no_run
/// use maki_core::export::InvariantProcessor;
/// use maki_core::cst::ast::Invariant;
///
/// let invariant: Invariant = todo!();
/// let constraint = InvariantProcessor::process(&invariant)?;
/// ```
pub struct InvariantProcessor;

impl InvariantProcessor {
    /// Convert an Invariant AST node to ElementDefinitionConstraint
    ///
    /// # Arguments
    ///
    /// * `invariant` - FSH Invariant AST node
    ///
    /// # Returns
    ///
    /// ElementDefinitionConstraint with key, severity, human description, and expression
    ///
    /// # Errors
    ///
    /// Returns error string if:
    /// - Invariant name is missing
    /// - Required fields are missing
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::InvariantProcessor;
    /// use maki_core::cst::ast::Invariant;
    ///
    /// let invariant: Invariant = todo!();
    /// let constraint = InvariantProcessor::process(&invariant)?;
    /// ```
    pub fn process(invariant: &Invariant) -> Result<ElementDefinitionConstraint, String> {
        // Get invariant name/key (required)
        let key = invariant
            .name()
            .ok_or_else(|| "Invariant must have a name".to_string())?;

        debug!("Processing invariant: {}", key);

        // Get severity (required, defaults to "error" if missing)
        let severity = invariant.severity().and_then(|s| s.value()).or_else(|| {
            warn!("Invariant {} missing Severity, defaulting to 'error'", key);
            Some("error".to_string())
        });

        // Get human description (required)
        let human = invariant
            .description()
            .and_then(|d| d.value())
            .ok_or_else(|| format!("Invariant {} must have Description", key))?;

        // Get FHIRPath expression (required)
        let expression = invariant
            .expression()
            .and_then(|e| e.value())
            .ok_or_else(|| format!("Invariant {} must have Expression", key))?;

        // Get XPath (optional)
        let xpath = invariant.xpath().and_then(|x| x.value());

        debug!("Created constraint {} with severity: {:?}", key, severity);

        Ok(ElementDefinitionConstraint {
            key,
            severity,
            human,
            expression: Some(expression),
        })
    }

    /// Process multiple invariants and return a map of key -> constraint
    ///
    /// # Arguments
    ///
    /// * `invariants` - Iterator of Invariant AST nodes
    ///
    /// # Returns
    ///
    /// HashMap mapping invariant keys to ElementDefinitionConstraints
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::InvariantProcessor;
    /// use maki_core::cst::ast::Document;
    ///
    /// let document: Document = todo!();
    /// let constraints = InvariantProcessor::process_all(document.invariants());
    /// ```
    pub fn process_all<I>(
        invariants: I,
    ) -> std::collections::HashMap<String, ElementDefinitionConstraint>
    where
        I: Iterator<Item = Invariant>,
    {
        let mut map = std::collections::HashMap::new();

        for invariant in invariants {
            match Self::process(&invariant) {
                Ok(constraint) => {
                    let key = constraint.key.clone();
                    if map.insert(key.clone(), constraint).is_some() {
                        warn!("Duplicate invariant key: {}", key);
                    }
                }
                Err(e) => {
                    warn!("Failed to process invariant: {}", e);
                }
            }
        }

        debug!("Processed {} invariants", map.len());
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::{
        ast::{AstNode, Document},
        parse_fsh,
    };

    #[test]
    fn test_process_invariant() {
        let source = r#"
Invariant: inv-1
Description: "SHALL have a contact party or an organization or both"
Expression: "telecom or name"
Severity: #error
"#;

        let (syntax, _lexer_errors, _errors) = parse_fsh(source);
        let doc = Document::cast(syntax).expect("Expected document");
        let invariant = doc.invariants().next().expect("Expected invariant");

        let constraint = InvariantProcessor::process(&invariant).unwrap();

        assert_eq!(constraint.key, "inv-1");
        assert_eq!(
            constraint.human,
            "SHALL have a contact party or an organization or both"
        );
        assert_eq!(constraint.expression, Some("telecom or name".to_string()));
        assert_eq!(constraint.severity, Some("error".to_string()));
    }

    #[test]
    fn test_process_invariant_without_severity() {
        let source = r#"
Invariant: inv-2
Description: "Must have focus or specimen"
Expression: "focus.exists() or specimen.exists()"
"#;

        let (syntax, _lexer_errors, _errors) = parse_fsh(source);
        let doc = Document::cast(syntax).expect("Expected document");
        let invariant = doc.invariants().next().expect("Expected invariant");

        let constraint = InvariantProcessor::process(&invariant).unwrap();

        assert_eq!(constraint.key, "inv-2");
        assert_eq!(constraint.human, "Must have focus or specimen");
        assert_eq!(
            constraint.expression,
            Some("focus.exists() or specimen.exists()".to_string())
        );
        // Should default to "error"
        assert_eq!(constraint.severity, Some("error".to_string()));
    }

    #[test]
    fn test_process_multiple_invariants() {
        let source = r#"
Invariant: inv-1
Description: "First invariant"
Expression: "value1.exists()"
Severity: #error

Invariant: inv-2
Description: "Second invariant"
Expression: "value2.exists()"
Severity: #warning
"#;

        let (syntax, _lexer_errors, _errors) = parse_fsh(source);
        let doc = Document::cast(syntax).expect("Expected document");

        let constraints = InvariantProcessor::process_all(doc.invariants());

        assert_eq!(constraints.len(), 2);
        assert!(constraints.contains_key("inv-1"));
        assert!(constraints.contains_key("inv-2"));

        let inv1 = &constraints["inv-1"];
        assert_eq!(inv1.severity, Some("error".to_string()));

        let inv2 = &constraints["inv-2"];
        assert_eq!(inv2.severity, Some("warning".to_string()));
    }
}
