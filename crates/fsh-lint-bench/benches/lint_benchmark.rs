use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use fsh_lint_core::{FshParser, Parser};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;

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

/// Benchmark real-world FSH files from mCODE IG
fn bench_mcode_files(c: &mut Criterion) {
    let mcode_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/mcode-ig");

    if !mcode_dir.exists() {
        eprintln!(
            "Skipping mCODE benchmarks - directory not found: {:?}",
            mcode_dir
        );
        return;
    }

    let fsh_files: Vec<_> = fs::read_dir(&mcode_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("fsh"))
        .collect();

    if fsh_files.is_empty() {
        eprintln!("Skipping mCODE benchmarks - no FSH files found");
        return;
    }

    println!("\n=== mCODE IG Benchmarking ===");
    println!("Found {} FSH files", fsh_files.len());

    let mut group = c.benchmark_group("mcode_real_world");

    // Benchmark parsing a single large file
    let large_file = fsh_files
        .iter()
        .max_by_key(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .unwrap();

    let large_content = fs::read_to_string(large_file.path()).unwrap();
    let file_name = large_file.file_name();
    let file_name_str = file_name.to_str().unwrap();

    println!(
        "Largest file: {} ({} bytes, {} lines)",
        file_name_str,
        large_content.len(),
        large_content.lines().count()
    );

    group.bench_function(format!("parse_{}", file_name_str), |b| {
        b.iter(|| {
            let mut parser = FshParser;
            black_box(parser.parse(&large_content))
        });
    });

    // Benchmark parsing all files
    let all_contents: Vec<String> = fsh_files
        .iter()
        .map(|e| fs::read_to_string(e.path()).unwrap())
        .collect();

    let total_size: usize = all_contents.iter().map(|s| s.len()).sum();
    let total_lines: usize = all_contents.iter().map(|s| s.lines().count()).sum();

    println!(
        "Total corpus: {} files, {} bytes, {} lines\n",
        all_contents.len(),
        total_size,
        total_lines
    );

    group.bench_function(format!("parse_all_{}_files", all_contents.len()), |b| {
        b.iter(|| {
            let mut parser = FshParser;
            for content in &all_contents {
                black_box(parser.parse(content));
            }
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
Profile: TestProfile{}
Parent: Patient
Description: "Test profile number {}"
* name 1..1 MS
* birthDate 0..1
"#,
                i, i
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

criterion_group!(benches, bench_parser, bench_mcode_files, bench_large_files,);
criterion_main!(benches);
