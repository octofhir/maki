//! Structural Rules Implementation - Simplified Version
//!
//! Implements core FSH structural rules for modifying FHIR StructureDefinitions.

use crate::export::fhir_types::{ElementDefinition, ElementDefinitionType, StructureDefinition};
use thiserror::Error;
use tracing::{debug, trace};

#[derive(Debug, Error)]
pub enum StructuralRuleError {
    #[error("Invalid rule: {0}")]
    InvalidRule(String),
}

type Result<T> = std::result::Result<T, StructuralRuleError>;

/// Cardinality rule
#[derive(Debug, Clone)]
pub struct CardinalityRule {
    pub path: String,
    pub min: Option<u32>,
    pub max: Option<String>,
}

/// Flag rule
#[derive(Debug, Clone)]
pub struct FlagRule {
    pub path: String,
    pub flags: Vec<String>,
}

/// Type restriction rule
#[derive(Debug, Clone)]
pub struct OnlyRule {
    pub path: String,
    pub types: Vec<String>,
}

/// Apply cardinality rule
pub fn apply_cardinality_rule(sd: &mut StructureDefinition, rule: &CardinalityRule) -> Result<()> {
    debug!("Applying CardinalityRule to {}", rule.path);
    let element = find_or_create_element(sd, &rule.path)?;
    if let Some(min) = rule.min {
        element.min = Some(min);
    }
    if let Some(max) = &rule.max {
        element.max = Some(max.clone());
    }
    Ok(())
}

/// Apply flag rule
pub fn apply_flag_rule(sd: &mut StructureDefinition, rule: &FlagRule) -> Result<()> {
    debug!("Applying FlagRule to {}", rule.path);
    let element = find_or_create_element(sd, &rule.path)?;
    for flag in &rule.flags {
        match flag.as_str() {
            "MS" => element.must_support = Some(true),
            "SU" => element.is_summary = Some(true),
            "?!" => element.is_modifier = Some(true),
            _ => trace!("Skipping unsupported flag: {}", flag),
        }
    }
    Ok(())
}

/// Apply OnlyRule
pub fn apply_only_rule(sd: &mut StructureDefinition, rule: &OnlyRule) -> Result<()> {
    debug!("Applying OnlyRule to {}", rule.path);
    let element = find_or_create_element(sd, &rule.path)?;
    element.type_ = Some(
        rule.types
            .iter()
            .map(|t| ElementDefinitionType::new(t.clone()))
            .collect(),
    );
    Ok(())
}

fn find_or_create_element<'a>(
    sd: &'a mut StructureDefinition,
    path: &str,
) -> Result<&'a mut ElementDefinition> {
    if sd.differential.is_none() {
        sd.differential = Some(crate::export::fhir_types::StructureDefinitionDifferential {
            element: Vec::new(),
        });
    }
    let diff = sd.differential.as_mut().unwrap();

    // Check if element exists
    let exists = diff.element.iter().any(|e| e.path == path);

    if !exists {
        diff.element.push(ElementDefinition::new(path.to_string()));
    }

    Ok(diff.element.iter_mut().find(|e| e.path == path).unwrap())
}
