//! Performance Baseline Tests
//!
//! These tests establish performance baselines for the decompiler.
//! Run with: `MAKI_QUICK_INIT=1 cargo test --package maki-decompiler --release --test performance_baseline -- --include-ignored --nocapture`

use maki_decompiler::models::*;
use maki_decompiler::test_helpers::*;
use maki_decompiler::*;
use serial_test::serial;
use std::time::Instant;

/// Helper to decompile a StructureDefinition and measure time
async fn decompile_and_measure(sd: StructureDefinition) -> (String, std::time::Duration) {
    let start = Instant::now();

    let session = setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
        .await
        .unwrap();
    let lake = ResourceLake::new(session);

    let processor = StructureDefinitionProcessor::new(&lake);
    let exportable = processor.process(&sd).await.unwrap();
    let fsh = exportable.to_fsh();

    let duration = start.elapsed();
    (fsh, duration)
}

#[tokio::test]
#[serial]
#[ignore] // Performance test - run explicitly with --include-ignored
async fn test_baseline_simple_profile() {
    let profile = fixtures::simple_patient_profile();
    let (_fsh, duration) = decompile_and_measure(profile).await;

    println!("Simple profile decompilation: {:?}", duration);

    // Baseline expectation: should be reasonably fast
    // This is a baseline test, not a strict requirement
    assert!(
        duration.as_millis() < 5000,
        "Simple profile took {:?}, expected < 5000ms (baseline)",
        duration
    );
}

#[tokio::test]
#[serial]
#[ignore] // Performance test - run explicitly with --include-ignored
async fn test_baseline_complex_profile() {
    let profile = fixtures::complex_patient_profile();
    let (_fsh, duration) = decompile_and_measure(profile).await;

    println!("Complex profile decompilation: {:?}", duration);

    assert!(
        duration.as_millis() < 5000,
        "Complex profile took {:?}, expected < 5000ms (baseline)",
        duration
    );
}

#[tokio::test]
#[serial]
#[ignore] // Performance test - run explicitly with --include-ignored
async fn test_baseline_slicing_profile() {
    let profile = fixtures::observation_with_slicing();
    let (_fsh, duration) = decompile_and_measure(profile).await;

    println!("Slicing profile decompilation: {:?}", duration);

    assert!(
        duration.as_millis() < 5000,
        "Slicing profile took {:?}, expected < 5000ms (baseline)",
        duration
    );
}

#[tokio::test]
#[serial]
#[ignore] // Long-running test
async fn test_baseline_batch_10_profiles() {
    let profiles: Vec<StructureDefinition> = (0..10)
        .map(|_| fixtures::simple_patient_profile())
        .collect();

    let start = Instant::now();

    let session = setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
        .await
        .unwrap();
    let lake = ResourceLake::new(session);

    for profile in profiles {
        let processor = StructureDefinitionProcessor::new(&lake);
        let exportable = processor.process(&profile).await.unwrap();
        let _fsh = exportable.to_fsh();
    }

    let duration = start.elapsed();
    let per_profile = duration.as_millis() / 10;

    println!(
        "10 profiles (sequential): {:?} total, ~{}ms per profile",
        duration, per_profile
    );

    // Target: < 1s for 10 resources (baseline allows up to 10s)
    assert!(
        duration.as_secs() < 10,
        "10 profiles took {:?}, expected < 10s (baseline)",
        duration
    );
}

#[tokio::test]
#[serial]
#[ignore] // Very long-running test
async fn test_baseline_batch_100_profiles() {
    let profiles: Vec<StructureDefinition> = (0..100)
        .map(|_| fixtures::simple_patient_profile())
        .collect();

    let start = Instant::now();

    let session = setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
        .await
        .unwrap();
    let lake = ResourceLake::new(session);

    for profile in profiles {
        let processor = StructureDefinitionProcessor::new(&lake);
        let exportable = processor.process(&profile).await.unwrap();
        let _fsh = exportable.to_fsh();
    }

    let duration = start.elapsed();
    let per_profile = duration.as_millis() / 100;

    println!(
        "100 profiles (sequential): {:?} total, ~{}ms per profile",
        duration, per_profile
    );

    // Target: < 5s for 100 resources (baseline allows up to 60s)
    assert!(
        duration.as_secs() < 60,
        "100 profiles took {:?}, expected < 60s (baseline)",
        duration
    );
}
