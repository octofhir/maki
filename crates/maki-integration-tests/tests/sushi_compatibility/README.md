# SUSHI Compatibility Test Suite

This directory contains the test harness for ensuring MAKI is compatible with SUSHI (the reference FSH compiler).

## Overview

The SUSHI compatibility tests compare MAKI's output with SUSHI's output to ensure:
- MAKI produces equivalent FHIR resources
- Implementation Guides compiled with MAKI are identical to those compiled with SUSHI
- Regressions are detected quickly
- Compatibility percentage is tracked over time

## Structure

```
sushi_compatibility/
├── README.md              # This file
├── mod.rs                 # Module exports
├── comparator.rs          # JSON comparison logic
├── runner.rs              # Test execution logic
├── fixtures/              # Test fixtures
│   ├── sushi_tests/       # SUSHI's original test suite (104 tests)
│   └── real_world_igs/    # Real-world Implementation Guides
├── expected_outputs/      # Golden files (SUSHI outputs)
└── reports/               # Generated compatibility reports
```

## Running Tests

### Prerequisites

- MAKI must be built: `cargo build`
- For comparison tests, SUSHI must be installed: `npm install -g fsh-sushi`

### Basic Tests

```bash
# Run basic compatibility tests (doesn't require SUSHI)
cargo test --test sushi_compatibility_test

# Run with SUSHI comparison (requires SUSHI installed)
cargo test --test sushi_compatibility_test -- --ignored

# Run all integration tests including compatibility
cargo test --package maki-integration-tests
```

### Individual Test Cases

```bash
# Run specific test
cargo test --test sushi_compatibility_test test_harness_creation

# Run with output
cargo test --test sushi_compatibility_test -- --nocapture
```

## Test Categories

### 1. Basic Unit Tests

Simple tests that don't require SUSHI:
- JSON comparison logic
- Difference detection
- Acceptable difference identification

### 2. Integration Tests (requires SUSHI)

Tests that compare MAKI and SUSHI outputs:
- Basic FSH profiles
- Extensions
- ValueSets and CodeSystems
- Instances
- Real-world IGs

### 3. Performance Tests

Compare execution times:
- MAKI should be significantly faster (target: >10x)
- Measure memory usage
- Track performance over time

## Adding Test Cases

### Adding a Simple Test

1. Add FSH file to `fixtures/` or use existing examples
2. Add test case in `sushi_compatibility_test.rs`:

```rust
#[test]
#[ignore]
fn test_my_feature() {
    let mut harness = SushiCompatibilityHarness::new().unwrap();

    let test_case = TestCase {
        name: "my-feature".to_string(),
        fsh_files: vec![PathBuf::from("fixtures/my-feature.fsh")],
        config_file: None,
        expected_outputs: vec![],
    };

    harness.add_test_case(test_case);
    let results = harness.run_all_tests();

    // Assert compatibility
    assert!(results[0].passed);
}
```

### Adding a Real-World IG

1. Download the IG to `fixtures/real_world_igs/`
2. Add test in `sushi_compatibility_test.rs`
3. Run with `--ignored` flag

## Acceptable Differences

Some differences between MAKI and SUSHI are acceptable and don't indicate incompatibility:

### Metadata Fields

- `date` - Generation timestamp
- `publisher` - "SUSHI" vs "MAKI"
- `version` - Tool version
- `generator` - Generator name
- `timestamp` - Generation time
- `_generatedBy` - Generator metadata

### Formatting

- JSON key ordering (both tools may order fields differently)
- Whitespace differences
- Indentation

### Implementation-Specific

- Internal IDs (as long as references are consistent)
- Generated UUIDs

## Compatibility Goals

- **Phase 0 (Current)**: ≥90% compatibility with basic tests
- **Phase 1**: ≥95% compatibility with SUSHI test suite
- **Phase 2**: ≥99% compatibility with real-world IGs
- **Phase 3**: 100% compatibility (MAKI as drop-in replacement)

## Current Status

Status will be tracked here as tests are implemented:

```
Overall Compatibility: TBD
Basic Tests: TBD
SUSHI Test Suite: TBD
Real-World IGs: TBD
```

## Troubleshooting

### "MAKI binary not found"

Build MAKI first:
```bash
cargo build --package maki-cli
```

### "SUSHI not available"

Install SUSHI:
```bash
npm install -g fsh-sushi
```

Or skip SUSHI comparison tests (they're marked with `#[ignore]`).

### Tests Failing

1. Check that MAKI build command is implemented (Task 28)
2. Verify SUSHI is installed and in PATH
3. Check test fixtures exist
4. Review differences in test output

## Future Enhancements

- [ ] Clone SUSHI's full test suite (104 tests)
- [ ] Download real-world IGs automatically
- [ ] Generate HTML compatibility reports
- [ ] Track compatibility over time (trend analysis)
- [ ] Add performance benchmarking
- [ ] Integrate with CI/CD pipeline
- [ ] Create compatibility dashboard

## Related Tasks

- **Task 28**: Build Command (required for full testing)
- **Task 29**: SUSHI Parity Testing (will use this harness)
- **Task 02**: CI/CD Setup (integration)

## References

- [SUSHI Repository](https://github.com/FHIR/sushi)
- [SUSHI Test Suite](https://github.com/FHIR/sushi/tree/master/test)
- [FSH Specification](https://hl7.org/fhir/uv/shorthand/)
