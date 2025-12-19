//! Exportable types for FSH output generation
//!
//! This module contains types that represent FSH constructs in memory.
//! Each Exportable knows how to convert itself to FSH text via the `to_fsh()` method.

pub mod code_system;
pub mod config;
pub mod extension;
pub mod instance;
pub mod logical;
pub mod profile;
pub mod resource;
pub mod rules;
pub mod value_set;

// Re-exports
pub use code_system::*;
pub use config::*;
pub use extension::*;
pub use instance::*;
pub use logical::*;
pub use profile::*;
pub use resource::*;
pub use rules::*;
pub use value_set::*;

use std::fmt;

/// Core trait for types that can be exported to FSH
///
/// This trait requires `Send + Sync` to allow exportables to be used in async contexts
/// and shared across threads.
pub trait Exportable: Send + Sync {
    /// Convert this exportable to FSH text
    fn to_fsh(&self) -> String;

    /// Get the unique identifier for this exportable
    fn id(&self) -> &str;

    /// Get the name of this exportable
    fn name(&self) -> &str;

    /// Get mutable access to rules (for optimizers)
    fn get_rules_mut(&mut self) -> &mut Vec<Box<dyn ExportableRule + Send + Sync>>;
}

/// FSH value types that can appear in rules
#[derive(Debug, Clone, PartialEq)]
pub enum FshValue {
    Boolean(bool),
    Integer(i32),
    Decimal(f64),
    String(String),
    Code(FshCode),
    Quantity(FshQuantity),
    CodeableConcept(FshCodeableConcept),
    Coding(FshCoding),
    Reference(FshReference),
    Canonical(String),
    Url(String),
    Uuid(String),
    Oid(String),
    Id(String),
}

impl FshValue {
    /// Convert the value to FSH syntax
    pub fn to_fsh(&self) -> String {
        match self {
            FshValue::Boolean(b) => b.to_string(),
            FshValue::Integer(i) => i.to_string(),
            FshValue::Decimal(d) => d.to_string(),
            FshValue::String(s) => format!("\"{}\"", escape_string(s)),
            FshValue::Code(code) => code.to_fsh(),
            FshValue::Quantity(qty) => qty.to_fsh(),
            FshValue::CodeableConcept(cc) => cc.to_fsh(),
            FshValue::Coding(coding) => coding.to_fsh(),
            FshValue::Reference(reference) => reference.to_fsh(),
            FshValue::Canonical(s) => format!("Canonical({})", s),
            FshValue::Url(s) => format!("\"{}\"", s),
            FshValue::Uuid(s) => format!("\"{}\"", s),
            FshValue::Oid(s) => format!("\"{}\"", s),
            FshValue::Id(s) => s.clone(),
        }
    }
}

/// FSH Code: #code or system#code
#[derive(Debug, Clone, PartialEq)]
pub struct FshCode {
    pub system: Option<String>,
    pub code: String,
}

impl FshCode {
    pub fn to_fsh(&self) -> String {
        match &self.system {
            Some(system) => format!("{}#{}", system, self.code),
            None => format!("#{}", self.code),
        }
    }
}

/// FSH Quantity: value 'unit' or value system#code "display"
#[derive(Debug, Clone, PartialEq)]
pub struct FshQuantity {
    pub value: Option<f64>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}

impl FshQuantity {
    pub fn to_fsh(&self) -> String {
        let mut parts = Vec::new();

        if let Some(val) = self.value {
            parts.push(val.to_string());
        }

        // If we have system and code, use system#code format
        if let (Some(sys), Some(c)) = (&self.system, &self.code) {
            parts.push(format!("{}#{}", sys, c));
        } else if let Some(u) = &self.unit {
            // Otherwise use unit format
            parts.push(format!("'{}'", u));
        }

        parts.join(" ")
    }
}

/// FSH CodeableConcept
#[derive(Debug, Clone, PartialEq)]
pub struct FshCodeableConcept {
    pub codings: Vec<FshCoding>,
    pub text: Option<String>,
}

impl FshCodeableConcept {
    pub fn to_fsh(&self) -> String {
        let mut parts = Vec::new();
        for coding in &self.codings {
            parts.push(coding.to_fsh());
        }
        if let Some(text) = &self.text {
            parts.push(format!("\"{}\"", escape_string(text)));
        }
        parts.join(" ")
    }
}

/// FSH Coding: system#code "display"
#[derive(Debug, Clone, PartialEq)]
pub struct FshCoding {
    pub system: Option<String>,
    pub code: String,
    pub display: Option<String>,
}

impl FshCoding {
    pub fn to_fsh(&self) -> String {
        let code_part = match &self.system {
            Some(system) => format!("{}#{}", system, self.code),
            None => format!("#{}", self.code),
        };

        match &self.display {
            Some(display) => format!("{} \"{}\"", code_part, escape_string(display)),
            None => code_part,
        }
    }
}

/// FSH Reference: Reference(Type) "display"
#[derive(Debug, Clone, PartialEq)]
pub struct FshReference {
    pub reference: String,
    pub display: Option<String>,
}

impl FshReference {
    pub fn to_fsh(&self) -> String {
        if let Some(display) = &self.display {
            format!(
                "Reference({}) \"{}\"",
                self.reference,
                escape_string(display)
            )
        } else {
            format!("Reference({})", self.reference)
        }
    }
}

/// Escape special characters in FSH strings
pub fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Format multi-line string for FSH (use triple quotes if needed)
pub fn format_multiline_string(s: &str) -> String {
    if s.contains('\n') {
        format!("\"\"\"{}\"\"\"", s)
    } else {
        format!("\"{}\"", escape_string(s))
    }
}

impl fmt::Display for FshValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_fsh())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fsh_value_boolean() {
        assert_eq!(FshValue::Boolean(true).to_fsh(), "true");
        assert_eq!(FshValue::Boolean(false).to_fsh(), "false");
    }

    #[test]
    fn test_fsh_value_integer() {
        assert_eq!(FshValue::Integer(42).to_fsh(), "42");
        assert_eq!(FshValue::Integer(-1).to_fsh(), "-1");
    }

    #[test]
    fn test_fsh_value_string() {
        assert_eq!(FshValue::String("hello".to_string()).to_fsh(), "\"hello\"");
        assert_eq!(
            FshValue::String("with \"quotes\"".to_string()).to_fsh(),
            "\"with \\\"quotes\\\"\""
        );
    }

    #[test]
    fn test_fsh_code() {
        let code = FshCode {
            system: None,
            code: "active".to_string(),
        };
        assert_eq!(code.to_fsh(), "#active");

        let code_with_system = FshCode {
            system: Some("http://hl7.org/fhir/status".to_string()),
            code: "active".to_string(),
        };
        assert_eq!(
            code_with_system.to_fsh(),
            "http://hl7.org/fhir/status#active"
        );
    }

    #[test]
    fn test_fsh_quantity() {
        let qty = FshQuantity {
            value: Some(5.0),
            unit: Some("mg".to_string()),
            system: None,
            code: None,
        };
        assert_eq!(qty.to_fsh(), "5 'mg'");

        let qty_no_unit = FshQuantity {
            value: Some(10.5),
            unit: None,
            system: None,
            code: None,
        };
        assert_eq!(qty_no_unit.to_fsh(), "10.5");

        let qty_with_code = FshQuantity {
            value: Some(100.0),
            unit: None,
            system: Some("http://unitsofmeasure.org".to_string()),
            code: Some("mg".to_string()),
        };
        assert_eq!(qty_with_code.to_fsh(), "100 http://unitsofmeasure.org#mg");
    }

    #[test]
    fn test_fsh_coding() {
        let coding = FshCoding {
            system: Some("http://snomed.info/sct".to_string()),
            code: "123456".to_string(),
            display: Some("Example".to_string()),
        };
        assert_eq!(coding.to_fsh(), "http://snomed.info/sct#123456 \"Example\"");
    }

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(escape_string("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_string("tab\there"), "tab\\there");
    }

    #[test]
    fn test_format_multiline_string() {
        assert_eq!(format_multiline_string("single line"), "\"single line\"");
        assert_eq!(
            format_multiline_string("line1\nline2"),
            "\"\"\"line1\nline2\"\"\""
        );
    }
}
