use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use maki_core::{FshParser, Parser};
use std::hint::black_box;

// Sample FSH content for synthetic benchmarks
const SIMPLE_PROFILE: &str = r#"Profile: SimplePatient
Parent: Patient
Description: "A simple patient profile for benchmarking"
* name 1..1 MS
* birthDate 0..1
"#;

const COMPLEX_PROFILE: &str = r#"Profile: ComplexPatient
Parent: Patient
Id: complex-patient
Title: "Complex Patient Profile"
Description: "A more complex patient profile with multiple constraints"
* name 1..* MS
  * given 1..* MS
  * family 1..1 MS
* birthDate 1..1 MS
* address 0..* MS
  * line 1..* MS
  * city 1..1 MS
  * state 0..1
  * postalCode 0..1
  * country 1..1 MS
* telecom 0..* MS
  * system 1..1 MS
  * value 1..1 MS
  * use 0..1
* identifier 1..* MS
  * system 1..1 MS
  * value 1..1 MS
"#;

/// Benchmark parsing different FSH constructs
fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    group.bench_function("simple_profile", |b| {
        b.iter(|| {
            let mut parser = FshParser;
            black_box(parser.parse(SIMPLE_PROFILE))
        });
    });

    group.bench_function("complex_profile", |b| {
        b.iter(|| {
            let mut parser = FshParser;
            black_box(parser.parse(COMPLEX_PROFILE))
        });
    });

    group.finish();
}

/// Benchmark parsing large generated files
fn bench_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_files");

    // Generate files of different sizes
    for &size in &[10, 50, 100, 200, 500] {
        let mut large_content = String::new();
        for i in 0..size {
            large_content.push_str(&format!(
                r#"
Profile: TestProfile{i}
Parent: Patient
Description: "Test profile number {i}"
* name 1..1 MS
* birthDate 0..1
"#
            ));
        }

        let bytes = large_content.len();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_profiles_{}KB", size, bytes / 1024)),
            &large_content,
            |b, content| {
                b.iter(|| {
                    let mut parser = FshParser;
                    black_box(parser.parse(content))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parser, bench_large_files);
criterion_main!(benches);
