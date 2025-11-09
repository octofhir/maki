//! Next-generation formatter using Token optimization
//!
//! This module demonstrates the Token optimization pattern applied to FSH formatting.
//! It shows how to use `token()` for static keywords/operators and `text()` for dynamic content.
//!
//! # Example
//!
//! ```rust,ignore
//! use maki_core::cst::formatter_v2::format_profile_optimized;
//! use maki_core::cst::ast::Profile;
//!
//! let formatted_elements = format_profile_optimized(&profile);
//! let output = Printer::new(options).print(&formatted_elements)?;
//! ```

#![allow(clippy::vec_init_then_push)] // Intentional pattern for building format elements

use super::{
    ast::{
        Alias, AstNode, CodeSystem, Extension, Instance, Invariant, Logical, Mapping, Profile,
        Resource, ValueSet,
    },
    format_element::{FormatElement, hard_line_break, space, text, token},
};

/// Format a profile using Token optimization
///
/// This function demonstrates the proper use of `token()` vs `text()`:
/// - `token()` for FSH keywords: "Profile", "Parent", "Id", etc.
/// - `token()` for operators: ":", "=", "*", etc.
/// - `text()` for dynamic content from source: profile names, paths, values
pub fn format_profile_optimized(profile: &Profile) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // Profile header: "Profile: <name>"
    elements.push(token("Profile")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = profile.name() {
        let pos = profile.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic content
    }
    elements.push(hard_line_break());

    // Parent clause: "Parent: <value>"
    if let Some(parent) = profile.parent() {
        elements.push(token("Parent")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = parent.value() {
            let pos = parent.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic content
        }
        elements.push(hard_line_break());
    }

    // Id clause: "Id: <value>"
    if let Some(id) = profile.id() {
        elements.push(token("Id")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = id.value() {
            let pos = id.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic content
        }
        elements.push(hard_line_break());
    }

    // Title clause: "Title: \"<value>\""
    if let Some(title) = profile.title() {
        elements.push(token("Title")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = title.value() {
            let pos = title.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic content
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Description clause: "Description: \"<value>\""
    if let Some(description) = profile.description() {
        elements.push(token("Description")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = description.value() {
            let pos = description.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic content
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    elements
}

/// Format an alias using Token optimization
///
/// Demonstrates: "Alias: <name> = <value>"
pub fn format_alias_optimized(alias: &Alias) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    elements.push(token("Alias")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = alias.name() {
        let pos = alias.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }

    elements.push(space());
    elements.push(token("=")); // Fast path: operator
    elements.push(space());

    if let Some(value_str) = alias.value() {
        let pos = alias.syntax().text_range().start();
        elements.push(text(&value_str, pos)); // Slow path: dynamic value
    }

    elements.push(hard_line_break());

    elements
}

/// Demonstrates formatting a simple rule: "* <path> <cardinality> <flags>"
///
/// Example: "* name 1..1 MS"
pub fn format_card_rule_optimized(
    path: &str,
    path_pos: rowan::TextSize,
    cardinality: &str,
    flags: &[&'static str],
) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    elements.push(token("*")); // Fast path: rule prefix
    elements.push(space());
    elements.push(text(path, path_pos)); // Slow path: dynamic path
    elements.push(space());
    elements.push(text(cardinality, path_pos)); // Slow path: cardinality from source

    for &flag in flags {
        elements.push(space());
        elements.push(token(flag)); // Fast path: MS, SU, etc.
    }

    elements.push(hard_line_break());

    elements
}

/// Format an instance using Token optimization
///
/// Demonstrates: "Instance: <name>" with InstanceOf clause
///
/// Example:
/// ```fsh
/// Instance: MyPatientExample
/// InstanceOf: Patient
/// Usage: #example
/// ```
pub fn format_instance_optimized(instance: &Instance) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // Instance header: "Instance: <name>"
    elements.push(token("Instance")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = instance.name() {
        let pos = instance.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }
    elements.push(hard_line_break());

    // InstanceOf clause: "InstanceOf: <type>"
    if let Some(instance_of) = instance.instance_of() {
        elements.push(token("InstanceOf")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(type_str) = instance_of.value() {
            let pos = instance_of.syntax().text_range().start();
            elements.push(text(&type_str, pos)); // Slow path: dynamic type
        }
        elements.push(hard_line_break());
    }

    // Usage clause: "Usage: #example"
    if let Some(usage) = instance.usage() {
        elements.push(token("Usage")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = usage.value() {
            let pos = usage.syntax().text_range().start();
            // Parser strips the # prefix, we need to add it back
            elements.push(token("#")); // Fast path: prefix
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }
        elements.push(hard_line_break());
    }

    elements
}

/// Format an invariant using Token optimization
///
/// Demonstrates: "Invariant: <name>" with Description, Severity, Expression, XPath clauses
///
/// Example:
/// ```fsh
/// Invariant: inv-1
/// Description: "Must have either a value or children"
/// Severity: #error
/// Expression: "value.exists() or children.exists()"
/// XPath: "exists(f:value) or exists(f:children)"
/// ```
pub fn format_invariant_optimized(invariant: &Invariant) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // Invariant header: "Invariant: <name>"
    elements.push(token("Invariant")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = invariant.name() {
        let pos = invariant.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }
    elements.push(hard_line_break());

    // Description clause: "Description: \"<value>\""
    if let Some(description) = invariant.description() {
        elements.push(token("Description")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = description.value() {
            let pos = description.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Severity clause: "Severity: #<value>"
    if let Some(severity) = invariant.severity() {
        elements.push(token("Severity")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = severity.value() {
            let pos = severity.syntax().text_range().start();
            // Parser returns just the identifier (e.g., "error"), we need to add # prefix
            elements.push(token("#")); // Fast path: hash prefix
            elements.push(text(&value_str, pos)); // Slow path: dynamic value (error, warning, etc.)
        }
        elements.push(hard_line_break());
    }

    // Expression clause: "Expression: \"<fhirpath>\""
    if let Some(expression) = invariant.expression() {
        elements.push(token("Expression")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = expression.value() {
            let pos = expression.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: FHIRPath expression
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // XPath clause: "XPath: \"<xpath>\""
    if let Some(xpath) = invariant.xpath() {
        elements.push(token("XPath")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = xpath.value() {
            let pos = xpath.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: XPath expression
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    elements
}

/// Format a mapping using Token optimization
///
/// Demonstrates: "Mapping: <name>" with Id, Source, Target, Title, Description clauses
///
/// Example:
/// ```fsh
/// Mapping: PatientToV2
/// Id: patient-to-v2
/// Source: Patient
/// Target: "HL7 V2 PID segment"
/// Title: "FHIR Patient to V2 PID Mapping"
/// Description: "Maps FHIR Patient to HL7 V2 PID segment"
/// ```
pub fn format_mapping_optimized(mapping: &Mapping) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // Mapping header: "Mapping: <name>"
    elements.push(token("Mapping")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = mapping.name() {
        let pos = mapping.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }
    elements.push(hard_line_break());

    // Id clause: "Id: <value>"
    if let Some(id) = mapping.id() {
        elements.push(token("Id")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = id.value() {
            let pos = id.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }
        elements.push(hard_line_break());
    }

    // Source clause: "Source: <value>"
    if let Some(source) = mapping.source() {
        elements.push(token("Source")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = source.value() {
            let pos = source.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: source identifier
        }
        elements.push(hard_line_break());
    }

    // Target clause: "Target: \"<value>\""
    if let Some(target) = mapping.target() {
        elements.push(token("Target")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = target.value() {
            let pos = target.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: target string
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Title clause: "Title: \"<value>\""
    if let Some(title) = mapping.title() {
        elements.push(token("Title")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = title.value() {
            let pos = title.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Description clause: "Description: \"<value>\""
    if let Some(description) = mapping.description() {
        elements.push(token("Description")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = description.value() {
            let pos = description.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    elements
}

/// Format a value set using Token optimization
///
/// Demonstrates: "ValueSet: <name>" with Id, Title, Description clauses
///
/// Example:
/// ```fsh
/// ValueSet: MaritalStatusVS
/// Id: marital-status-vs
/// Title: "Marital Status Value Set"
/// Description: "A value set for marital status codes"
/// ```
pub fn format_valueset_optimized(valueset: &ValueSet) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // ValueSet header: "ValueSet: <name>"
    elements.push(token("ValueSet")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = valueset.name() {
        let pos = valueset.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }
    elements.push(hard_line_break());

    // Id clause: "Id: <value>"
    if let Some(id) = valueset.id() {
        elements.push(token("Id")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = id.value() {
            let pos = id.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }
        elements.push(hard_line_break());
    }

    // Title clause: "Title: \"<value>\""
    if let Some(title) = valueset.title() {
        elements.push(token("Title")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = title.value() {
            let pos = title.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Description clause: "Description: \"<value>\""
    if let Some(description) = valueset.description() {
        elements.push(token("Description")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = description.value() {
            let pos = description.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    elements
}

/// Format a code system using Token optimization
///
/// Demonstrates: "CodeSystem: <name>" with Id, Title, Description clauses
///
/// Example:
/// ```fsh
/// CodeSystem: MyCodeSystem
/// Id: my-code-system
/// Title: "My Code System"
/// Description: "A custom code system"
/// ```
pub fn format_codesystem_optimized(codesystem: &CodeSystem) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // CodeSystem header: "CodeSystem: <name>"
    elements.push(token("CodeSystem")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = codesystem.name() {
        let pos = codesystem.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }
    elements.push(hard_line_break());

    // Id clause: "Id: <value>"
    if let Some(id) = codesystem.id() {
        elements.push(token("Id")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = id.value() {
            let pos = id.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }
        elements.push(hard_line_break());
    }

    // Title clause: "Title: \"<value>\""
    if let Some(title) = codesystem.title() {
        elements.push(token("Title")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = title.value() {
            let pos = title.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Description clause: "Description: \"<value>\""
    if let Some(description) = codesystem.description() {
        elements.push(token("Description")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = description.value() {
            let pos = description.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    elements
}

/// Format an extension using Token optimization
///
/// Demonstrates: "Extension: <name>" with Parent, Id, Title, Description clauses
///
/// Example:
/// ```fsh
/// Extension: MyExtension
/// Parent: Extension
/// Id: my-extension
/// Title: "My Custom Extension"
/// Description: "An extension for additional data"
/// ```
pub fn format_extension_optimized(extension: &Extension) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // Extension header: "Extension: <name>"
    elements.push(token("Extension")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = extension.name() {
        let pos = extension.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }
    elements.push(hard_line_break());

    // Parent clause: "Parent: <value>"
    if let Some(parent) = extension.parent() {
        elements.push(token("Parent")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = parent.value() {
            let pos = parent.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic content
        }
        elements.push(hard_line_break());
    }

    // Id clause: "Id: <value>"
    if let Some(id) = extension.id() {
        elements.push(token("Id")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = id.value() {
            let pos = id.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }
        elements.push(hard_line_break());
    }

    // Title clause: "Title: \"<value>\""
    if let Some(title) = extension.title() {
        elements.push(token("Title")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = title.value() {
            let pos = title.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Description clause: "Description: \"<value>\""
    if let Some(description) = extension.description() {
        elements.push(token("Description")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = description.value() {
            let pos = description.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    elements
}

/// Format a logical model using Token optimization
///
/// Demonstrates: "Logical: <name>" with Parent, Id, Title, Description clauses
///
/// Example:
/// ```fsh
/// Logical: MyLogicalModel
/// Parent: Element
/// Id: my-logical-model
/// Title: "My Logical Model"
/// Description: "A logical model for data definition"
/// ```
pub fn format_logical_optimized(logical: &Logical) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // Logical header: "Logical: <name>"
    elements.push(token("Logical")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = logical.name() {
        let pos = logical.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }
    elements.push(hard_line_break());

    // Parent clause: "Parent: <value>"
    if let Some(parent) = logical.parent() {
        elements.push(token("Parent")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = parent.value() {
            let pos = parent.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic content
        }
        elements.push(hard_line_break());
    }

    // Id clause: "Id: <value>"
    if let Some(id) = logical.id() {
        elements.push(token("Id")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = id.value() {
            let pos = id.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }
        elements.push(hard_line_break());
    }

    // Title clause: "Title: \"<value>\""
    if let Some(title) = logical.title() {
        elements.push(token("Title")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = title.value() {
            let pos = title.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Description clause: "Description: \"<value>\""
    if let Some(description) = logical.description() {
        elements.push(token("Description")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = description.value() {
            let pos = description.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    elements
}

/// Format a resource using Token optimization
///
/// Demonstrates: "Resource: <name>" with Parent, Id, Title, Description clauses
///
/// Example:
/// ```fsh
/// Resource: MyResource
/// Parent: DomainResource
/// Id: my-resource
/// Title: "My Custom Resource"
/// Description: "A custom resource definition"
/// ```
pub fn format_resource_optimized(resource: &Resource) -> Vec<FormatElement> {
    let mut elements = Vec::new();

    // Resource header: "Resource: <name>"
    elements.push(token("Resource")); // Fast path: keyword
    elements.push(token(":")); // Fast path: punctuation
    elements.push(space());

    if let Some(name_str) = resource.name() {
        let pos = resource.syntax().text_range().start();
        elements.push(text(&name_str, pos)); // Slow path: dynamic name
    }
    elements.push(hard_line_break());

    // Parent clause: "Parent: <value>"
    if let Some(parent) = resource.parent() {
        elements.push(token("Parent")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = parent.value() {
            let pos = parent.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic content
        }
        elements.push(hard_line_break());
    }

    // Id clause: "Id: <value>"
    if let Some(id) = resource.id() {
        elements.push(token("Id")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());

        if let Some(value_str) = id.value() {
            let pos = id.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }
        elements.push(hard_line_break());
    }

    // Title clause: "Title: \"<value>\""
    if let Some(title) = resource.title() {
        elements.push(token("Title")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = title.value() {
            let pos = title.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    // Description clause: "Description: \"<value>\""
    if let Some(description) = resource.description() {
        elements.push(token("Description")); // Fast path: keyword
        elements.push(token(":")); // Fast path: punctuation
        elements.push(space());
        elements.push(token("\"")); // Fast path: quote

        if let Some(value_str) = description.value() {
            let pos = description.syntax().text_range().start();
            elements.push(text(&value_str, pos)); // Slow path: dynamic value
        }

        elements.push(token("\"")); // Fast path: quote
        elements.push(hard_line_break());
    }

    elements
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::{
        ast::Document,
        parse_fsh,
        printer::{Printer, PrinterOptions},
    };

    #[test]
    fn test_format_alias_optimized() {
        let source = "Alias: SCT = http://snomed.info/sct";
        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();
        let alias = doc.aliases().next().unwrap();

        let elements = format_alias_optimized(&alias);

        let printer = Printer::new(PrinterOptions::default());
        let mut printer = printer;
        let output = printer.print(&elements).unwrap();

        assert_eq!(output.trim(), "Alias: SCT = http://snomed.info/sct");
    }

    #[test]
    fn test_format_profile_basic() {
        let source = r#"Profile: MyPatient
Parent: Patient
Id: my-patient"#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();
        let profile = doc.profiles().next().unwrap();

        let elements = format_profile_optimized(&profile);

        let printer = Printer::new(PrinterOptions::default());
        let mut printer = printer;
        let output = printer.print(&elements).unwrap();

        assert!(output.contains("Profile: MyPatient"));
        assert!(output.contains("Parent: Patient"));
        assert!(output.contains("Id: my-patient"));
    }

    #[test]
    fn test_token_vs_text_usage() {
        // This test demonstrates the Token optimization pattern
        let elements = vec![
            token("Profile"),                            // Fast path: static keyword
            token(":"),                                  // Fast path: static punctuation
            space(),                                     // Fast path: space
            text("MyProfile", rowan::TextSize::from(8)), // Slow path: dynamic
        ];

        let printer = Printer::new(PrinterOptions::default());
        let mut printer = printer;
        let output = printer.print(&elements).unwrap();

        assert_eq!(output.trim(), "Profile: MyProfile");
    }

    #[test]
    fn test_card_rule_optimized() {
        let elements =
            format_card_rule_optimized("name", rowan::TextSize::from(0), "1..1", &["MS"]);

        let printer = Printer::new(PrinterOptions::default());
        let mut printer = printer;
        let output = printer.print(&elements).unwrap();

        assert_eq!(output.trim(), "* name 1..1 MS");
    }

    #[test]
    fn test_format_instance_optimized() {
        let source = r#"Instance: MyPatientExample
InstanceOf: Patient
Usage: #example"#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();

        if let Some(instance) = doc.instances().next() {
            let elements = format_instance_optimized(&instance);

            let printer = Printer::new(PrinterOptions::default());
            let mut printer = printer;
            let output = printer.print(&elements).unwrap();

            assert!(output.contains("Instance: MyPatientExample"));
            assert!(output.contains("InstanceOf: Patient"));
            assert!(output.contains("Usage: #example"));
        }
    }

    #[test]
    fn test_format_invariant_optimized() {
        let source = r#"Invariant: inv-1
Description: "Must have a value"
Severity: #error
Expression: "value.exists()""#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();

        if let Some(invariant) = doc.invariants().next() {
            let elements = format_invariant_optimized(&invariant);

            let printer = Printer::new(PrinterOptions::default());
            let mut printer = printer;
            let output = printer.print(&elements).unwrap();

            assert!(output.contains("Invariant: inv-1"));
            assert!(output.contains("Description: \"Must have a value\""));
            assert!(output.contains("Severity: #error"));
            assert!(output.contains("Expression: \"value.exists()\""));
        }
    }

    #[test]
    fn test_format_mapping_optimized() {
        let source = r#"Mapping: PatientToV2
Id: patient-to-v2
Source: Patient
Target: "HL7 V2 PID segment"
Title: "FHIR Patient to V2 PID Mapping""#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();

        if let Some(mapping) = doc.mappings().next() {
            let elements = format_mapping_optimized(&mapping);

            let printer = Printer::new(PrinterOptions::default());
            let mut printer = printer;
            let output = printer.print(&elements).unwrap();

            assert!(output.contains("Mapping: PatientToV2"));
            assert!(output.contains("Id: patient-to-v2"));
            assert!(output.contains("Source: Patient"));
            assert!(output.contains("Target: \"HL7 V2 PID segment\""));
            assert!(output.contains("Title: \"FHIR Patient to V2 PID Mapping\""));
        }
    }

    #[test]
    fn test_format_valueset_optimized() {
        let source = r#"ValueSet: MaritalStatusVS
Id: marital-status-vs
Title: "Marital Status Value Set"
Description: "A value set for marital status codes""#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();

        if let Some(valueset) = doc.value_sets().next() {
            let elements = format_valueset_optimized(&valueset);

            let printer = Printer::new(PrinterOptions::default());
            let mut printer = printer;
            let output = printer.print(&elements).unwrap();

            assert!(output.contains("ValueSet: MaritalStatusVS"));
            assert!(output.contains("Id: marital-status-vs"));
            assert!(output.contains("Title: \"Marital Status Value Set\""));
            assert!(output.contains("Description: \"A value set for marital status codes\""));
        }
    }

    #[test]
    fn test_format_codesystem_optimized() {
        let source = r#"CodeSystem: MyCodeSystem
Id: my-code-system
Title: "My Code System"
Description: "A custom code system""#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();

        if let Some(codesystem) = doc.code_systems().next() {
            let elements = format_codesystem_optimized(&codesystem);

            let printer = Printer::new(PrinterOptions::default());
            let mut printer = printer;
            let output = printer.print(&elements).unwrap();

            assert!(output.contains("CodeSystem: MyCodeSystem"));
            assert!(output.contains("Id: my-code-system"));
            assert!(output.contains("Title: \"My Code System\""));
            assert!(output.contains("Description: \"A custom code system\""));
        }
    }

    #[test]
    fn test_format_extension_optimized() {
        let source = r#"Extension: MyExtension
Id: my-extension
Title: "My Custom Extension"
Description: "An extension for additional data""#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();

        if let Some(extension) = doc.extensions().next() {
            let elements = format_extension_optimized(&extension);

            let printer = Printer::new(PrinterOptions::default());
            let mut printer = printer;
            let output = printer.print(&elements).unwrap();

            assert!(output.contains("Extension: MyExtension"));
            assert!(output.contains("Id: my-extension"));
            assert!(output.contains("Title: \"My Custom Extension\""));
            assert!(output.contains("Description: \"An extension for additional data\""));
        }
    }

    #[test]
    fn test_format_logical_optimized() {
        let source = r#"Logical: MyLogicalModel
Parent: Element
Id: my-logical-model
Title: "My Logical Model"
Description: "A logical model for data definition""#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();

        if let Some(logical) = doc.logicals().next() {
            let elements = format_logical_optimized(&logical);

            let printer = Printer::new(PrinterOptions::default());
            let mut printer = printer;
            let output = printer.print(&elements).unwrap();

            assert!(output.contains("Logical: MyLogicalModel"));
            assert!(output.contains("Parent: Element"));
            assert!(output.contains("Id: my-logical-model"));
            assert!(output.contains("Title: \"My Logical Model\""));
            assert!(output.contains("Description: \"A logical model for data definition\""));
        }
    }

    #[test]
    fn test_format_resource_optimized() {
        let source = r#"Resource: MyResource
Parent: DomainResource
Id: my-resource
Title: "My Custom Resource"
Description: "A custom resource definition""#;

        let (cst, _, _) = parse_fsh(source);
        let doc = Document::cast(cst).unwrap();

        if let Some(resource) = doc.resources().next() {
            let elements = format_resource_optimized(&resource);

            let printer = Printer::new(PrinterOptions::default());
            let mut printer = printer;
            let output = printer.print(&elements).unwrap();

            assert!(output.contains("Resource: MyResource"));
            assert!(output.contains("Parent: DomainResource"));
            assert!(output.contains("Id: my-resource"));
            assert!(output.contains("Title: \"My Custom Resource\""));
            assert!(output.contains("Description: \"A custom resource definition\""));
        }
    }
}
