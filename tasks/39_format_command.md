# Task 39: Format Command CLI Integration

**Phase**: 3 (Formatter - Week 14)
**Time Estimate**: 2 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Task 38 (FSH Formatter)

## Overview

Implement the `maki format` command-line interface to provide users with easy access to FSH code formatting. This includes file/directory processing, check mode for CI/CD, configuration management, parallel processing, and integration with the autofix engine.

**Part of Formatter Phase**: Week 14 focuses on CLI integration and production-ready features for the formatter implemented in Week 13 (Task 38).

## Context

A formatter is most useful when it's easy to invoke:

- **One-command formatting**: Format entire project with single command
- **CI/CD integration**: Verify formatting in pipelines with `--check`
- **Editor integration**: Can be called from editor save hooks
- **Configuration**: Respect project-specific formatting preferences
- **Performance**: Fast enough for large projects (parallel processing)

The format command bridges the Formatter core (Task 38) with end-user workflows.

## Goals

1. **Implement `maki format` command** - Format files and directories
2. **Add check mode (`--check`)** - Verify formatting without modifying files
3. **Support configuration files** - `.maki.toml` and CLI flag overrides
4. **Parallel processing** - Format multiple files concurrently
5. **Progress reporting** - Show progress for large projects
6. **Integration with lint/fix** - Format after applying autofixes
7. **CI/CD support** - Appropriate exit codes and output

## Technical Specification

### Command Structure (Rust)

```rust
use clap::Parser;
use maki_core::cst::formatter::{Formatter, FormattingOptions};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "format")]
#[command(about = "Format FSH files according to style guidelines")]
pub struct FormatCommand {
    /// Paths to format (files or directories)
    #[arg(value_name = "PATH", default_value = "input/fsh")]
    paths: Vec<PathBuf>,

    /// Check if files are formatted without modifying them
    #[arg(long)]
    check: bool,

    /// Indent style: "spaces" or "tabs"
    #[arg(long, value_name = "STYLE")]
    indent_style: Option<String>,

    /// Number of spaces per indent level (if using spaces)
    #[arg(long, value_name = "SIZE", default_value = "2")]
    indent_size: usize,

    /// Maximum line width
    #[arg(long, value_name = "WIDTH", default_value = "120")]
    line_width: usize,

    /// Align rule assignments
    #[arg(long)]
    align: bool,

    /// Show current formatting configuration
    #[arg(long)]
    show_config: bool,

    /// Configuration file path
    #[arg(long, short = 'c', value_name = "PATH")]
    config: Option<PathBuf>,

    /// Parallel processing jobs
    #[arg(long, value_name = "JOBS")]
    jobs: Option<usize>,
}

impl FormatCommand {
    /// Execute format command
    pub async fn execute(&self) -> Result<()> {
        // Load configuration
        let mut options = FormattingOptions::default();

        if let Some(config_path) = &self.config {
            options = FormattingOptions::from_file(config_path)?;
        }

        // Override with CLI flags
        if let Some(indent_style) = &self.indent_style {
            options.indent_style = parse_indent_style(indent_style)?;
        }
        options.indent_size = self.indent_size;
        options.line_width = self.line_width;
        options.align_rules = self.align;

        // Collect files to format
        let mut files = Vec::new();
        for path in &self.paths {
            if path.is_file() {
                files.push(path.clone());
            } else if path.is_dir() {
                files.extend(find_fsh_files(path)?);
            }
        }

        // Format files (with parallelization)
        let num_jobs = self.jobs.unwrap_or_else(num_cpus::get);
        let results = format_files_parallel(&files, &options, num_jobs).await?;

        // Handle results
        if self.check {
            check_formatting(&results)?;
        } else {
            write_formatted_files(&results)?;
        }

        Ok(())
    }
}
```

    /// Number of parallel jobs (default: number of CPUs)
    #[arg(long, short = 'j', value_name = "N")]
    jobs: Option<usize>,

    /// Show progress for large projects
    #[arg(long)]
    progress: bool,

    /// Quiet mode (only show errors)
    #[arg(long, short = 'q')]
    quiet: bool,

    /// Verbose output
    #[arg(long, short = 'v')]
    verbose: bool,
}

impl FormatCommand {
    pub fn execute(&self) -> Result<()> {
        // Load configuration
        let config = self.load_configuration()?;

        // Show configuration if requested
        if self.show_config {
            return self.display_config(&config);
        }

        // Collect files to format
        let files = self.collect_files()?;

        if files.is_empty() {
            eprintln!("No FSH files found");
            return Ok(());
        }

        // Format files
        if self.check {
            self.check_formatting(&files, &config)
        } else {
            self.format_files(&files, &config)
        }
    }

    /// Load configuration from file and CLI overrides
    fn load_configuration(&self) -> Result<FormattingOptions> {
        // Start with defaults
        let mut config = FormattingOptions::default();

        // Load from config file
        if let Some(config_path) = &self.config {
            config = FormattingOptions::from_file(config_path)?;
        } else {
            // Try default config files
            for default_path in &[".maki.toml", ".makirc.toml", ".makiformat"] {
                if Path::new(default_path).exists() {
                    config = FormattingOptions::from_file(default_path)?;
                    break;
                }
            }
        }

        // Override with CLI flags
        if let Some(ref style) = self.indent_style {
            config.indent_style = match style.as_str() {
                "spaces" => IndentStyle::Spaces(self.indent_size),
                "tabs" => IndentStyle::Tabs,
                _ => return Err(anyhow!("Invalid indent style: {}", style)),
            };
        }

        if self.align {
            config.align_rules = true;
        }

        config.line_width = self.line_width;

        Ok(config)
    }

    /// Collect FSH files from input paths
    fn collect_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for path in &self.paths {
            if path.is_file() {
                if path.extension().map_or(false, |ext| ext == "fsh") {
                    files.push(path.clone());
                }
            } else if path.is_dir() {
                files.extend(find_fsh_files(path)?);
            } else {
                eprintln!("Warning: Path does not exist: {}", path.display());
            }
        }

        Ok(files)
    }

    /// Format files in-place
    fn format_files(
        &self,
        files: &[PathBuf],
        config: &FormattingOptions,
    ) -> Result<()> {
        let formatter = Formatter::new(config.clone());

        // Determine parallelism
        let num_jobs = self.jobs.unwrap_or_else(num_cpus::get);

        // Set up progress bar
        let progress = if self.progress && !self.quiet {
            Some(ProgressBar::new(files.len() as u64))
        } else {
            None
        };

        // Format files in parallel
        let results: Vec<_> = files
            .par_iter()
            .with_max_threads(num_jobs)
            .map(|file| {
                let result = self.format_file(&formatter, file);
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
                result
            })
            .collect();

        if let Some(pb) = progress {
            pb.finish_with_message("Formatting complete");
        }

        // Summarize results
        let mut formatted_count = 0;
        let mut error_count = 0;

        for result in results {
            match result {
                Ok(true) => formatted_count += 1,
                Ok(false) => {}
                Err(e) => {
                    eprintln!("Error: {}", e);
                    error_count += 1;
                }
            }
        }

        if !self.quiet {
            if formatted_count > 0 {
                println!("Formatted {} file(s)", formatted_count);
            } else {
                println!("All files already formatted");
            }
        }

        if error_count > 0 {
            Err(anyhow!("Failed to format {} file(s)", error_count))
        } else {
            Ok(())
        }
    }

    /// Format a single file
    fn format_file(
        &self,
        formatter: &Formatter,
        path: &Path,
    ) -> Result<bool> {
        // Read source
        let source = fs::read_to_string(path)?;

        // Format
        let formatted = formatter.format_file(&source)
            .context(format!("Failed to format {}", path.display()))?;

        // Check if changed
        if formatted == source {
            if self.verbose {
                println!("Already formatted: {}", path.display());
            }
            return Ok(false);
        }

        // Write back
        fs::write(path, formatted)?;

        if !self.quiet {
            println!("Formatted: {}", path.display());
        }

        Ok(true)
    }

    /// Check formatting without modifying files
    fn check_formatting(
        &self,
        files: &[PathBuf],
        config: &FormattingOptions,
    ) -> Result<()> {
        let formatter = Formatter::new(config.clone());
        let num_jobs = self.jobs.unwrap_or_else(num_cpus::get);

        let progress = if self.progress && !self.quiet {
            Some(ProgressBar::new(files.len() as u64))
        } else {
            None
        };

        // Check files in parallel
        let unformatted: Vec<_> = files
            .par_iter()
            .with_max_threads(num_jobs)
            .filter_map(|file| {
                let result = self.check_file(&formatter, file);
                if let Some(ref pb) = progress {
                    pb.inc(1);
                }
                match result {
                    Ok(true) => Some(file.clone()),  // Needs formatting
                    Ok(false) => None,  // Already formatted
                    Err(e) => {
                        eprintln!("Error checking {}: {}", file.display(), e);
                        Some(file.clone())
                    }
                }
            })
            .collect();

        if let Some(pb) = progress {
            pb.finish_with_message("Check complete");
        }

        // Report results
        if unformatted.is_empty() {
            if !self.quiet {
                println!("All {} file(s) are formatted correctly", files.len());
            }
            Ok(())
        } else {
            eprintln!("The following {} file(s) need formatting:", unformatted.len());
            for file in &unformatted {
                eprintln!("  {}", file.display());
            }
            eprintln!();
            eprintln!("Run 'maki format' to format these files");
            std::process::exit(1);
        }
    }

    /// Check if a single file needs formatting
    fn check_file(&self, formatter: &Formatter, path: &Path) -> Result<bool> {
        let source = fs::read_to_string(path)?;
        let formatted = formatter.format_file(&source)?;

        // Return true if file needs formatting
        Ok(formatted != source)
    }

    /// Display current configuration
    fn display_config(&self, config: &FormattingOptions) -> Result<()> {
        println!("Current formatting configuration:");
        println!();
        println!("  Indent style:    {}", config.indent_style);
        println!("  Line width:      {}", config.line_width);
        println!("  Align rules:     {}", config.align_rules);
        println!("  Group rules:     {}", config.group_rules);
        println!("  Sort rules:      {}", config.sort_rules);
        println!("  Normalize spacing: {}", config.normalize_spacing);
        println!("  Max blank lines: {}", config.max_blank_lines);
        println!();

        Ok(())
    }
}

/// Find all FSH files in a directory recursively
fn find_fsh_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "fsh") {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}
```

### Integration with Lint Command

```rust
// In maki-cli/src/commands/lint.rs

impl LintCommand {
    pub fn execute(&self) -> Result<()> {
        // ... existing lint logic ...

        // After applying autofixes, format the code
        if self.fix || self.fix_unsafe {
            self.format_fixed_files(&fixed_files)?;
        }

        Ok(())
    }

    fn format_fixed_files(&self, files: &[PathBuf]) -> Result<()> {
        // Load formatting config
        let config = FormattingOptions::load_default()?;
        let formatter = Formatter::new(config);

        for file in files {
            let source = fs::read_to_string(file)?;
            let formatted = formatter.format_file(&source)?;

            if formatted != source {
                fs::write(file, formatted)?;
                println!("Formatted: {}", file.display());
            }
        }

        Ok(())
    }
}
```

### Configuration File Format

**`.maki.toml` (TOML format):**

```toml
[format]
# Indent style: "spaces" or "tabs"
indent_style = "spaces"
indent_size = 2

# Maximum line width
line_width = 120

# Rule formatting
align_rules = true
group_rules = false
sort_rules = false

# Spacing
normalize_spacing = true
blank_lines_between_groups = 1

# Blank line handling
preserve_blank_lines = true
max_blank_lines = 2
```

**`.makiformat` (Simple key-value format):**

```
indent_style=spaces
indent_size=2
line_width=120
align_rules=true
normalize_spacing=true
max_blank_lines=2
```

## CLI Usage Examples

```bash
# Format all FSH files in default directory (input/fsh/)
maki format

# Format specific directory
maki format input/fsh/profiles/

# Format single file
maki format input/fsh/MyProfile.fsh

# Check formatting (CI mode)
maki format --check
# Exit code 0 if all formatted, 1 if any need formatting

# Format with custom configuration
maki format --indent-style spaces --indent-size 4 --line-width 100

# Show current configuration
maki format --show-config

# Format with progress bar
maki format --progress

# Format using specific config file
maki format --config .maki-custom.toml

# Parallel processing with 8 threads
maki format --jobs 8

# Quiet mode (only show errors)
maki format --quiet

# Verbose mode (show all processed files)
maki format --verbose

# Format multiple paths
maki format input/fsh/profiles/ input/fsh/extensions/
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Check Formatting
on: [push, pull_request]

jobs:
  format:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install maki
        run: cargo install maki-cli

      - name: Check formatting
        run: maki format --check

      - name: Show unformatted files
        if: failure()
        run: |
          echo "The following files need formatting:"
          maki format --check || true
```

### GitLab CI

```yaml
format-check:
  stage: test
  script:
    - cargo install maki-cli
    - maki format --check
  only:
    - merge_requests
    - main
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Format staged FSH files
git diff --cached --name-only --diff-filter=ACM | grep '\.fsh$' | while read file; do
    maki format "$file"
    git add "$file"
done
```

## Implementation Location

**Primary File**: `crates/maki-cli/src/commands/format.rs` (new file)

**Supporting Files**:
- `crates/maki-core/src/formatter.rs` - Core formatter (Task 38)
- `crates/maki-cli/src/config.rs` - Configuration loading
- `crates/maki-cli/src/main.rs` - Command registration

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_format_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.fsh");

        fs::write(&file_path, "Profile:MyProfile\nParent:Patient").unwrap();

        let cmd = FormatCommand {
            paths: vec![file_path.clone()],
            check: false,
            ..Default::default()
        };

        cmd.execute().unwrap();

        let formatted = fs::read_to_string(&file_path).unwrap();
        assert!(formatted.contains("Profile: MyProfile"));
        assert!(formatted.contains("Parent: Patient"));
    }

    #[test]
    fn test_check_mode_unformatted() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.fsh");

        fs::write(&file_path, "Profile:MyProfile").unwrap();

        let cmd = FormatCommand {
            paths: vec![file_path],
            check: true,
            ..Default::default()
        };

        let result = cmd.execute();
        assert!(result.is_err());  // Should fail for unformatted file
    }

    #[test]
    fn test_check_mode_formatted() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.fsh");

        fs::write(&file_path, "Profile: MyProfile\nParent: Patient\n").unwrap();

        let cmd = FormatCommand {
            paths: vec![file_path],
            check: true,
            ..Default::default()
        };

        cmd.execute().unwrap();
    }

    #[test]
    fn test_config_override() {
        let config = FormattingOptions::default();

        let cmd = FormatCommand {
            indent_size: 4,
            align: true,
            line_width: 100,
            ..Default::default()
        };

        let merged = cmd.load_configuration().unwrap();

        assert_eq!(merged.line_width, 100);
        assert!(merged.align_rules);
    }

    #[test]
    fn test_collect_files_directory() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("test1.fsh"), "").unwrap();
        fs::write(temp_dir.path().join("test2.fsh"), "").unwrap();
        fs::write(temp_dir.path().join("readme.md"), "").unwrap();

        let cmd = FormatCommand {
            paths: vec![temp_dir.path().to_path_buf()],
            ..Default::default()
        };

        let files = cmd.collect_files().unwrap();
        assert_eq!(files.len(), 2);  // Only .fsh files
    }
}
```

### Integration Tests

```bash
# Test basic formatting
cd test-project
maki format
git diff --exit-code  # Should show changes

# Test check mode
maki format --check
echo $?  # Should be 1 (needs formatting)

# Apply formatting
maki format

# Check again
maki format --check
echo $?  # Should be 0 (already formatted)

# Test configuration
echo "indent_size=4" > .makiformat
maki format
grep "    " input/fsh/MyProfile.fsh  # Should find 4-space indents

# Test parallel processing
time maki format --jobs 1
time maki format --jobs 8
# Should be faster with more jobs
```

## Performance Considerations

- **Parallel processing**: Use rayon for multi-threaded file formatting
- **File I/O optimization**: Read/write files efficiently
- **Caching**: Consider caching formatted results based on file hash
- **Progress reporting**: Update progress bar efficiently (not every file)
- **Memory usage**: Process files in batches for very large projects

**Performance Targets:**
- Single file: <50ms
- 100 files: <2 seconds (parallel)
- 1000 files: <10 seconds (parallel)

### Benchmarking Token Optimization (Required)

Since Task 38 implements Token optimization, the CLI must provide benchmarking capabilities to measure and verify the performance improvement:

```bash
# Run formatter with timing information
maki format --bench input/fsh/

# Example output:
# Formatting Benchmark Results:
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Files processed: 150
# Total time: 1.23s (8.2ms avg per file)
# Token fast path: 78% of text operations
# Unicode slow path: 22% of text operations
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

#### Benchmark Implementation

```rust
#[derive(Parser, Debug)]
#[command(name = "format")]
pub struct FormatCommand {
    // ... existing fields ...

    /// Show detailed performance metrics
    #[arg(long)]
    bench: bool,
}

impl FormatCommand {
    fn format_files_with_bench(&self, files: &[PathBuf], config: &FormattingOptions) -> Result<()> {
        let start = Instant::now();
        let formatter = Formatter::new(config.clone());

        // Track token vs text operations if available
        let mut token_ops = 0u64;
        let mut text_ops = 0u64;

        for file in files {
            let file_start = Instant::now();
            let result = self.format_file(&formatter, file)?;
            let file_duration = file_start.elapsed();

            // Collect stats if formatter exposes them
            if let Some(stats) = formatter.last_format_stats() {
                token_ops += stats.token_operations;
                text_ops += stats.text_operations;
            }

            if self.verbose {
                println!("{}: {:.2}ms", file.display(), file_duration.as_secs_f64() * 1000.0);
            }
        }

        let total_duration = start.elapsed();

        // Print benchmark results
        println!("\nFormatting Benchmark Results:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Files processed: {}", files.len());
        println!("Total time: {:.2}s ({:.1}ms avg per file)",
            total_duration.as_secs_f64(),
            total_duration.as_millis() as f64 / files.len() as f64
        );

        if token_ops + text_ops > 0 {
            let total_ops = token_ops + text_ops;
            let token_pct = (token_ops as f64 / total_ops as f64) * 100.0;
            let text_pct = (text_ops as f64 / total_ops as f64) * 100.0;

            println!("Token fast path: {:.0}% of text operations", token_pct);
            println!("Unicode slow path: {:.0}% of text operations", text_pct);

            // Estimate improvement
            let estimated_improvement = calculate_token_improvement(token_pct);
            println!("\nEstimated improvement vs non-optimized: {:.1}%", estimated_improvement);
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        Ok(())
    }
}

/// Estimate performance improvement based on token percentage
/// Based on Ruff data: 2-3% improvement with ~80% token usage
fn calculate_token_improvement(token_percentage: f64) -> f64 {
    // Linear estimate: 3% improvement at 100% tokens
    (token_percentage / 100.0) * 3.0
}
```

#### Integration Testing for Performance

```rust
#[test]
fn test_format_performance_target() {
    let temp_dir = create_test_ig_with_files(100);

    let start = Instant::now();
    let output = Command::new("maki")
        .args(&["format", temp_dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    let duration = start.elapsed();

    assert!(output.status.success());

    // Performance target: 100 files in <2s
    assert!(
        duration.as_millis() < 2000,
        "Formatting 100 files took {}ms (expected <2000ms)",
        duration.as_millis()
    );
}

#[test]
fn test_bench_output_format() {
    let temp_dir = create_test_ig_with_files(10);

    let output = Command::new("maki")
        .args(&[
            "format",
            "--bench",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify benchmark output contains expected metrics
    assert!(stdout.contains("Formatting Benchmark Results"));
    assert!(stdout.contains("Files processed: 10"));
    assert!(stdout.contains("Total time:"));
    assert!(stdout.contains("avg per file"));
}
```

#### Formatter Stats API

To support benchmarking, the formatter should expose statistics:

```rust
// In maki-core/src/cst/formatter.rs

pub struct FormatStats {
    /// Number of Token operations (fast path)
    pub token_operations: u64,

    /// Number of Text operations (Unicode slow path)
    pub text_operations: u64,

    /// Total characters processed
    pub characters_processed: usize,
}

impl Formatter {
    /// Get statistics from last format operation
    pub fn last_format_stats(&self) -> Option<FormatStats> {
        self.stats.clone()
    }
}
```

**Note**: Benchmarking is required to verify Token optimization performance. Expected improvements: 2-5% for FSH files. See `TOKEN_OPTIMIZATION_ANALYSIS.md` for detailed analysis.

## Dependencies

### Crate Dependencies

```toml
[dependencies]
clap = { version = "4.0", features = ["derive"] }
anyhow = "1.0"
rayon = "1.10"
walkdir = "2.4"
indicatif = "0.17"  # Progress bars
num_cpus = "1.16"
```

### Required Components
- **Formatter** (Task 38): Core formatting engine
- **Configuration System**: Load/merge config files
- **File System Utils**: Walk directories, find FSH files

## Acceptance Criteria

### Core Functionality
- [ ] `maki format` formats files in-place
- [ ] `maki format --check` verifies formatting without modifying
- [ ] `--check` exits with code 1 if any files need formatting
- [ ] Configuration files (`.maki.toml`, `.makiformat`) are supported
- [ ] CLI flags override configuration file settings
- [ ] `--show-config` displays current configuration
- [ ] Parallel processing works correctly with `--jobs`
- [ ] Progress bar shows for large projects with `--progress`
- [ ] Single files and directories are both supported
- [ ] Integration with `maki lint --fix` formats after autofixes
- [ ] Error handling is robust (skip unreadable files)
- [ ] Exit codes are correct for CI/CD use
- [ ] Unit tests cover all command modes
- [ ] Integration tests verify end-to-end workflows
- [ ] Performance meets targets (<10s for 1000 files)

### Benchmarking Support (Required)
- [ ] `--bench` flag displays performance metrics
- [ ] Benchmark output shows token vs text operation distribution (70-85% token usage expected)
- [ ] Benchmark output shows estimated improvement vs non-optimized
- [ ] Performance tests verify formatter meets targets (<50ms per file)
- [ ] Integration with `FormatStats` API from formatter
- [ ] Verification that Token optimization achieves >1% improvement

## Edge Cases

1. **Empty directories**: Should not fail, just report "No files found"
2. **Mixed formatted/unformatted**: Format only unformatted files
3. **Parse errors**: Skip files that don't parse, report error
4. **Permission errors**: Skip unreadable/unwritable files, report error
5. **Symlinks**: Follow symlinks (WalkDir default)
6. **Hidden files**: Skip hidden directories (`.git`, etc.)
7. **Very large files**: Handle files >1MB efficiently
8. **Concurrent modification**: Detect if file changed during formatting

## Future Enhancements

1. **Watch mode**: `maki format --watch` for continuous formatting
2. **Diff output**: Show diffs for `--check` mode
3. **Partial formatting**: Format only changed lines (git diff integration)
4. **Format on save**: Editor integration via LSP
5. **Custom formatters**: Allow user-defined formatting plugins
6. **Format statistics**: Report formatting metrics (alignment, line length)

## Related Tasks

- **Task 38: FSH Formatter** - Core formatting implementation
- **Task 37: Autofix Engine** - Integration with `maki lint --fix`
- **Task 42: LSP Formatting** - Format on save via LSP
- **Task 47: Code Actions** - LSP format document/range actions

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium (CLI integration with existing formatter)
**Priority**: High (completes Phase 3)
**Updated**: 2025-11-03
