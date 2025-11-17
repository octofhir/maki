//! Decompiler Performance Benchmarks
//!
//! This benchmark suite measures the performance of the FHIR to FSH decompiler.
//!
//! Run with: `cargo bench --package maki-bench decompiler_benchmark`

use criterion::{Criterion, criterion_group, criterion_main};
use maki_decompiler::test_helpers::*;
use maki_decompiler::*;
use std::hint::black_box;
use std::time::Duration;

/// Benchmark: Decompile a simple patient profile
fn bench_simple_profile_decompile(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("decompile_simple_patient_profile", |b| {
        b.iter(|| {
            rt.block_on(async {
                let profile = black_box(fixtures::simple_patient_profile());

                // Set up canonical environment
                let session =
                    setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
                        .await
                        .unwrap();
                let lake = ResourceLake::new(session);

                // Process the profile
                let processor = StructureDefinitionProcessor::new(&lake);
                let exportable = processor.process(&profile).await.unwrap();

                // Generate FSH
                black_box(exportable.to_fsh())
            })
        });
    });
}

/// Benchmark: Decompile a complex patient profile
fn bench_complex_profile_decompile(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("decompile_complex_patient_profile", |b| {
        b.iter(|| {
            rt.block_on(async {
                let profile = black_box(fixtures::complex_patient_profile());

                let session =
                    setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
                        .await
                        .unwrap();
                let lake = ResourceLake::new(session);

                let processor = StructureDefinitionProcessor::new(&lake);
                let exportable = processor.process(&profile).await.unwrap();

                black_box(exportable.to_fsh())
            })
        });
    });
}

/// Benchmark: Decompile profile with slicing
fn bench_slicing_profile_decompile(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("decompile_profile_with_slicing", |b| {
        b.iter(|| {
            rt.block_on(async {
                let profile = black_box(fixtures::observation_with_slicing());

                let session =
                    setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
                        .await
                        .unwrap();
                let lake = ResourceLake::new(session);

                let processor = StructureDefinitionProcessor::new(&lake);
                let exportable = processor.process(&profile).await.unwrap();

                black_box(exportable.to_fsh())
            })
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(20);  // Fewer samples due to async overhead
    targets =
        bench_simple_profile_decompile,
        bench_complex_profile_decompile,
        bench_slicing_profile_decompile
}

criterion_main!(benches);
