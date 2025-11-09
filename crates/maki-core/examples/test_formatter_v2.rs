//! Test formatter_v2 on real FSH files
//!
//! Run with: cargo run --example test_formatter_v2

use maki_core::cst::{
    ast::{AstNode, Document},
    formatter_v2::{
        format_alias_optimized, format_codesystem_optimized, format_extension_optimized,
        format_instance_optimized, format_invariant_optimized, format_logical_optimized,
        format_mapping_optimized, format_profile_optimized, format_resource_optimized,
        format_valueset_optimized,
    },
    parse_fsh,
    printer::{Printer, PrinterOptions},
};
use std::fs;

fn main() {
    let test_files = vec![
        "examples/simple-profile.fsh",
        "examples/patient-profile.fsh",
        "examples/comprehensive-test.fsh",
    ];

    println!("Testing formatter_v2 on example FSH files\n");
    println!("{}", "=".repeat(80));

    for file_path in test_files {
        println!("\n📄 Processing: {}", file_path);

        let source = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                println!("  ❌ Error reading file: {}", e);
                continue;
            }
        };

        let (cst, lexer_errors, parse_errors) = parse_fsh(&source);

        if !lexer_errors.is_empty() {
            println!("  ⚠️  {} lexer errors", lexer_errors.len());
        }

        if !parse_errors.is_empty() {
            println!("  ⚠️  {} parse errors", parse_errors.len());
        }

        let doc = match Document::cast(cst) {
            Some(doc) => doc,
            None => {
                println!("  ❌ Failed to cast to Document");
                continue;
            }
        };

        let mut printer = Printer::new(PrinterOptions::default());
        let mut total_formatted = 0;

        // Format Profiles
        for profile in doc.profiles() {
            let elements = format_profile_optimized(&profile);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = profile.name() {
                        println!("  ✅ Formatted Profile: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting profile: {}", e);
                }
            }
            printer.reset();
        }

        // Format Aliases
        for alias in doc.aliases() {
            let elements = format_alias_optimized(&alias);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = alias.name() {
                        println!("  ✅ Formatted Alias: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting alias: {}", e);
                }
            }
            printer.reset();
        }

        // Format Instances
        for instance in doc.instances() {
            let elements = format_instance_optimized(&instance);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = instance.name() {
                        println!("  ✅ Formatted Instance: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting instance: {}", e);
                }
            }
            printer.reset();
        }

        // Format Invariants
        for invariant in doc.invariants() {
            let elements = format_invariant_optimized(&invariant);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = invariant.name() {
                        println!("  ✅ Formatted Invariant: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting invariant: {}", e);
                }
            }
            printer.reset();
        }

        // Format Mappings
        for mapping in doc.mappings() {
            let elements = format_mapping_optimized(&mapping);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = mapping.name() {
                        println!("  ✅ Formatted Mapping: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting mapping: {}", e);
                }
            }
            printer.reset();
        }

        // Format ValueSets
        for valueset in doc.value_sets() {
            let elements = format_valueset_optimized(&valueset);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = valueset.name() {
                        println!("  ✅ Formatted ValueSet: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting valueset: {}", e);
                }
            }
            printer.reset();
        }

        // Format CodeSystems
        for codesystem in doc.code_systems() {
            let elements = format_codesystem_optimized(&codesystem);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = codesystem.name() {
                        println!("  ✅ Formatted CodeSystem: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting codesystem: {}", e);
                }
            }
            printer.reset();
        }

        // Format Extensions
        for extension in doc.extensions() {
            let elements = format_extension_optimized(&extension);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = extension.name() {
                        println!("  ✅ Formatted Extension: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting extension: {}", e);
                }
            }
            printer.reset();
        }

        // Format Logical models
        for logical in doc.logicals() {
            let elements = format_logical_optimized(&logical);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = logical.name() {
                        println!("  ✅ Formatted Logical: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting logical: {}", e);
                }
            }
            printer.reset();
        }

        // Format Resources
        for resource in doc.resources() {
            let elements = format_resource_optimized(&resource);
            match printer.print(&elements) {
                Ok(output) => {
                    total_formatted += 1;
                    if let Some(name) = resource.name() {
                        println!("  ✅ Formatted Resource: {}", name);
                        println!("     Output length: {} chars", output.len());
                    }
                }
                Err(e) => {
                    println!("  ❌ Error formatting resource: {}", e);
                }
            }
            printer.reset();
        }

        println!("\n  📊 Total items formatted: {}", total_formatted);
    }

    println!("\n{}", "=".repeat(80));
    println!("✨ Formatter v2 testing complete!");
}
