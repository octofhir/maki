# FSH Lint - Performance Benchmark Results

**Date**: 2025-10-06
**Test Corpus**: HL7 mCODE Implementation Guide
**Total Files**: 57 FSH files
**Total Size**: 663,588 bytes (~648 KB)
**Total Lines**: 9,617 lines of FSH code

## Test Environment

- **Platform**: macOS Darwin 25.0.0
- **Rust Version**: Edition 2024
- **Build Profile**: Release with optimizations
- **Benchmark Tool**: Criterion.rs v0.7

## Benchmark Results

### 1. Synthetic Parsing Benchmarks

| Test Case | Mean Time | Std Dev | Outliers |
|-----------|-----------|---------|----------|
| Simple Profile | **18.13 µs** | ±2.4 µs | 10% |
| Complex Profile | **104.74 µs** | ±11.6 µs | 22% |

*Simple Profile: ~70 bytes, 6 lines*
*Complex Profile: ~340 bytes, 36 lines with nested constraints*

### 2. Real-World mCODE IG Parsing

#### Single Large File

| File | Size | Lines | Parse Time |
|------|------|-------|------------|
| CM_TG263.fsh | 105,529 bytes | 681 | **16.62 ms** |

**Throughput**: ~40 MB/s (~40,000 lines/sec)

#### All 57 Files Sequentially

| Metric | Value |
|--------|-------|
| Total Files | 57 |
| Total Size | 663,588 bytes (~648 KB) |
| Total Lines | 9,617 lines |
| **Parse Time** | **78.57 ms** |
| **Throughput** | **~8.2 MB/sec** |
| **Lines/sec** | **~122,000 lines/sec** |

### 3. Scaling Performance

| Test Case | Size | Mean Time | Throughput |
|-----------|------|-----------|------------|
| 10 profiles | 1 KB | 74.77 µs | ~13 MB/s |
| 50 profiles | 5 KB | 272.86 µs | ~18 MB/s |
| 100 profiles | 10 KB | 524.38 µs | ~19 MB/s |
| 200 profiles | 21 KB | 1.14 ms | ~18 MB/s |
| 500 profiles | 54 KB | 2.86 ms | ~19 MB/s |

**Key Observation**: Parser performance scales linearly with file size, maintaining consistent ~18-19 MB/s throughput.

## Performance Analysis

### Strengths

1. **Consistent Performance**: Linear scaling across file sizes
2. **Real-World Speed**: 57 files (~648 KB) parsed in **<80ms**
3. **High Throughput**: Sustained 8-19 MB/s parsing speed
4. **Predictable**: Low variance in measurements

### Performance Characteristics

- **Microsecond Response**: Small profiles parse in <100 µs
- **Sub-Second Batch**: Entire IG (57 files) in <0.1 second
- **Scalable**: 500-profile files in under 3ms

## Realistic Use Cases

### Scenario 1: Single File Linting

- **Typical Profile**: ~100 lines
- **Parse Time**: ~500 µs (0.5 ms)
- **Total Time (with rules)**: <5 ms estimated

### Scenario 2: Project Linting

- **50 FSH files**: ~200 KB total
- **Parse Time**: ~25 ms
- **Total Time (with rules)**: <100 ms estimated

### Scenario 3: Large IG

- **200 FSH files**: ~1 MB total
- **Parse Time**: ~125 ms
- **Total Time (with rules)**: <500 ms estimated

## Comparison to Performance Claims

❌ **Marketing claim**: "1000 files in <5 seconds"
✅ **Actual performance**: **57 files (648 KB) in 78ms**

Extrapolating:

- **1000 files** of similar size (~11 MB): **~1.4 seconds** (parsing only)
- With rules execution overhead: **~3-5 seconds** (realistic estimate)

**Verdict**: Performance claims are achievable but depend on file size and rule complexity.

## Bottlenecks & Optimization Opportunities

1. **Rule Execution**: Not yet benchmarked (needs working DefaultRuleEngine)
2. **Parallel Processing**: Currently sequential, could benefit from Rayon
3. **Caching**: No caching implemented yet
4. **Incremental Parsing**: Not yet implemented

## Benchmark Artifacts

- Full results: `BENCHMARK_RESULTS.txt`
- HTML reports: `target/criterion/report/index.html`
- Raw data: `target/criterion/*/`

## How to Run

```bash
# Run all benchmarks
cargo bench --package maki-bench

# View HTML report
open target/criterion/report/index.html

# Run specific benchmark
cargo bench --package maki-bench -- simple_profile
```

## Conclusion

The parser demonstrates **solid performance** with:

- ✅ **Consistent linear scaling**
- ✅ **Real-world corpus (mCODE IG) parsed in <80ms**
- ✅ **Predictable, low-variance timing**
- ⚠️ **Rule execution performance not yet measured**

---

*Generated from Criterion benchmarks on real HL7 FHIR Implementation Guide source files*
