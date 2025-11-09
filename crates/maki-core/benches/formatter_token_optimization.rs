//! Benchmark comparing Token optimization vs traditional string building
//!
//! This benchmark measures the performance impact of the Token optimization pattern
//! used in formatter_v2. Expected improvement: 2-5% based on Ruff/Biome results.

use criterion::{Criterion, criterion_group, criterion_main};
use maki_core::cst::{
    ast::{AstNode, Document},
    formatter_v2::{
        format_alias_optimized, format_codesystem_optimized, format_extension_optimized,
        format_instance_optimized, format_logical_optimized, format_profile_optimized,
        format_resource_optimized, format_valueset_optimized,
    },
    parse_fsh,
    printer::{Printer, PrinterOptions},
};
use std::hint::black_box;

/// Benchmark Profile formatting with Token optimization
fn bench_profile_token_optimized(c: &mut Criterion) {
    let source = r#"Profile: MyPatient
Parent: Patient
Id: my-patient
Title: "My Patient Profile"
Description: "A patient profile for testing""#;

    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let profile = doc.profiles().next().unwrap();

    c.bench_function("profile_token_optimized", |b| {
        b.iter(|| {
            let elements = format_profile_optimized(black_box(&profile));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

/// Benchmark Alias formatting with Token optimization
fn bench_alias_token_optimized(c: &mut Criterion) {
    let source = "Alias: SCT = http://snomed.info/sct";
    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let alias = doc.aliases().next().unwrap();

    c.bench_function("alias_token_optimized", |b| {
        b.iter(|| {
            let elements = format_alias_optimized(black_box(&alias));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

/// Benchmark Instance formatting with Token optimization
fn bench_instance_token_optimized(c: &mut Criterion) {
    let source = r#"Instance: MyPatientExample
InstanceOf: Patient
Usage: #example"#;

    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let instance = doc.instances().next().unwrap();

    c.bench_function("instance_token_optimized", |b| {
        b.iter(|| {
            let elements = format_instance_optimized(black_box(&instance));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

/// Benchmark large Profile with multiple clauses
fn bench_large_profile(c: &mut Criterion) {
    let source = r#"Profile: ComplexPatient
Parent: Patient
Id: complex-patient
Title: "Complex Patient Profile with Many Fields"
Description: "A comprehensive patient profile demonstrating multiple clauses and longer content for realistic benchmarking scenarios"
* name 1..1 MS "Patient name is required"
* birthDate 1..1 MS "Birth date is required"
* gender 1..1 MS "Gender is required"
* address 0..* MS "Patient addresses"
* telecom 0..* MS "Contact information"
* identifier 1..* MS "Business identifiers"
* active 1..1 MS "Active flag"
* managingOrganization 0..1 MS "Managing organization""#;

    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let profile = doc.profiles().next().unwrap();

    c.bench_function("large_profile_token_optimized", |b| {
        b.iter(|| {
            let elements = format_profile_optimized(black_box(&profile));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

/// Benchmark ValueSet formatting with Token optimization
fn bench_valueset_token_optimized(c: &mut Criterion) {
    let source = r#"ValueSet: MaritalStatusVS
Id: marital-status-vs
Title: "Marital Status Value Set"
Description: "A value set for marital status codes""#;

    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let valueset = doc.value_sets().next().unwrap();

    c.bench_function("valueset_token_optimized", |b| {
        b.iter(|| {
            let elements = format_valueset_optimized(black_box(&valueset));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

/// Benchmark CodeSystem formatting with Token optimization
fn bench_codesystem_token_optimized(c: &mut Criterion) {
    let source = r#"CodeSystem: MyCodeSystem
Id: my-code-system
Title: "My Code System"
Description: "A custom code system""#;

    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let codesystem = doc.code_systems().next().unwrap();

    c.bench_function("codesystem_token_optimized", |b| {
        b.iter(|| {
            let elements = format_codesystem_optimized(black_box(&codesystem));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

/// Benchmark Extension formatting with Token optimization
fn bench_extension_token_optimized(c: &mut Criterion) {
    let source = r#"Extension: MyExtension
Id: my-extension
Title: "My Custom Extension"
Description: "An extension for additional data""#;

    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let extension = doc.extensions().next().unwrap();

    c.bench_function("extension_token_optimized", |b| {
        b.iter(|| {
            let elements = format_extension_optimized(black_box(&extension));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

/// Benchmark Logical formatting with Token optimization
fn bench_logical_token_optimized(c: &mut Criterion) {
    let source = r#"Logical: MyLogicalModel
Parent: Element
Id: my-logical-model
Title: "My Logical Model"
Description: "A logical model for data definition""#;

    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let logical = doc.logicals().next().unwrap();

    c.bench_function("logical_token_optimized", |b| {
        b.iter(|| {
            let elements = format_logical_optimized(black_box(&logical));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

/// Benchmark Resource formatting with Token optimization
fn bench_resource_token_optimized(c: &mut Criterion) {
    let source = r#"Resource: MyResource
Parent: DomainResource
Id: my-resource
Title: "My Custom Resource"
Description: "A custom resource definition""#;

    let (cst, _, _) = parse_fsh(source);
    let doc = Document::cast(cst).unwrap();
    let resource = doc.resources().next().unwrap();

    c.bench_function("resource_token_optimized", |b| {
        b.iter(|| {
            let elements = format_resource_optimized(black_box(&resource));
            let mut printer = Printer::new(PrinterOptions::default());
            let _ = printer.print(black_box(&elements));
        });
    });
}

criterion_group!(
    benches,
    bench_profile_token_optimized,
    bench_alias_token_optimized,
    bench_instance_token_optimized,
    bench_large_profile,
    bench_valueset_token_optimized,
    bench_codesystem_token_optimized,
    bench_extension_token_optimized,
    bench_logical_token_optimized,
    bench_resource_token_optimized
);

criterion_main!(benches);
