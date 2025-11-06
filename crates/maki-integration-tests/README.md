# MAKI Integration Tests

Comprehensive integration testing for MAKI, including SUSHI parity testing.

## SUSHI Parity Testing

The parity testing framework runs MAKI against SUSHI's test suite to verify compatibility.

### Prerequisites

1. **Build MAKI**:

   ```bash
   cargo build --release --package maki-cli
   ```

2. **Install SUSHI** (or use existing installation):

   ```bash
   npm install -g fsh-sushi
   ```

3. **Clone SUSHI repository** (for test fixtures):

   ```bash
   git clone https://github.com/FHIR/sushi.git /path/to/sushi
   ```

### Running Parity Tests

#### Basic Usage

```bash
cargo run --bin run-parity-tests
```

#### With Custom Paths

```bash
cargo run --bin run-parity-tests -- \
  --sushi /usr/local/bin/sushi \
  --maki ./target/release/maki \
  --fixtures /path/to/sushi/test/ig/fixtures \
  --output ./parity-reports
```

#### Environment Variables

You can also set paths via environment variables:

```bash
export SUSHI_PATH=/usr/local/bin/sushi
export MAKI_PATH=./target/release/maki
export SUSHI_FIXTURES=/path/to/sushi/test/ig/fixtures

cargo run --bin run-parity-tests
```

#### Filtering Tests

Run only tests matching a pattern:

```bash
cargo run --bin run-parity-tests -- --filter simple-ig
```

#### Verbose Output

Get detailed output for each test:

```bash
cargo run --bin run-parity-tests -- --verbose
```

### Understanding Reports

The test runner generates two report files in the output directory:

#### 1. JSON Report (`parity_report.json`)

Machine-readable report with full test details:

```json
{
  "total_tests": 104,
  "passed_tests": 99,
  "failed_tests": 5,
  "compatibility_percent": 95.19,
  "test_results": [...],
  "difference_categories": {
    "profiles": 3,
    "instances": 2
  },
  "sushi_version": "3.0.0",
  "maki_version": "0.0.2"
}
```

#### 2. Markdown Report (`parity_report.md`)

Human-readable report with:

- Summary statistics
- Test results table
- Detailed failure analysis

### Interpretation

- **Compatibility >= 95%**: Target met ✅
- **Compatibility < 95%**: Needs improvement ⚠️

Common difference categories:

- **profiles**: Profile generation differences
- **extensions**: Extension generation differences
- **valuesets**: ValueSet generation differences
- **codesystems**: CodeSystem generation differences
- **instances**: Instance generation differences
- **other**: Miscellaneous differences

### CI Integration

Add to your CI pipeline:

```yaml
# .github/workflows/parity-tests.yml
name: SUSHI Parity Tests

on: [push, pull_request]

jobs:
  parity:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install SUSHI
        run: npm install -g fsh-sushi

      - name: Clone SUSHI fixtures
        run: git clone --depth=1 https://github.com/FHIR/sushi.git /tmp/sushi

      - name: Build MAKI
        run: cargo build --release --package maki-cli

      - name: Run Parity Tests
        run: |
          cargo run --bin run-parity-tests -- \
            --fixtures /tmp/sushi/test/ig/fixtures \
            --output ./parity-reports

      - name: Upload Reports
        uses: actions/upload-artifact@v4
        with:
          name: parity-reports
          path: parity-reports/
```

### Troubleshooting

#### "Maki executable not found"

Ensure you've built the release version:

```bash
cargo build --release --package maki-cli
```

#### "SUSHI fixtures directory not found"

Update the fixtures path:

```bash
cargo run --bin run-parity-tests -- --fixtures /correct/path/to/sushi/test/ig/fixtures
```

#### "SUSHI command not found"

Install SUSHI globally:

```bash
npm install -g fsh-sushi
# Or specify custom path
cargo run --bin run-parity-tests -- --sushi /path/to/sushi
```

### Development

To add new comparison logic:

1. Edit `src/sushi_parity.rs`
2. Update the `compare_outputs()` method
3. Add new difference categories as needed
4. Run tests:

   ```bash
   cargo test --package maki-integration-tests
   ```

### Test Structure

The parity testing framework consists of:

- **`ParityTestRunner`**: Orchestrates test execution
- **`ParityTestResult`**: Individual test result
- **`ParityReport`**: Aggregated report
- **`OutputSummary`**: Build output statistics

### Extending

To add new test types:

1. Create fixture directories with `sushi-config.yaml`
2. Place FSH files in `input/fsh/`
3. Run parity tests - new fixtures are auto-discovered

### Performance

- **Test discovery**: ~100ms for 104 fixtures
- **Per-test execution**: ~2-5 seconds (SUSHI + MAKI)
- **Total runtime**: ~5-10 minutes for full suite

### Known Limitations

1. Currently compares only resource counts and file lists
2. Deep JSON comparison not yet implemented
3. Some SUSHI-specific features may not be supported yet
4. Test fixtures must have `sushi-config.yaml` to be discovered

### Future Enhancements

- [ ] Deep JSON content comparison
- [ ] Diff visualization in reports
- [ ] Parallel test execution
- [ ] Progressive test filtering
- [ ] Performance benchmarking
- [ ] Snapshot testing for outputs
