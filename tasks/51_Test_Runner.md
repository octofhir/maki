# Task 51: Test Runner CLI

**Phase**: 5 (Testing Framework - Week 21)
**Time Estimate**: 1-2 days
**Status**: 📝 Planned
**Priority**: Medium
**Dependencies**: Task 50 (Test DSL)

## Overview

Implement the CLI for running FSH tests with watch mode, filtering, and coverage reporting.

## Goals

1. **`maki test` command** - Run all tests in project
2. **Filter tests** - By name/pattern
3. **Watch mode** - Re-run on file changes
4. **Progress reporting** - Show test execution progress
5. **Exit codes** - Proper codes for CI/CD

## Technical Specification

```rust
#[derive(Parser)]
pub struct TestCommand {
    /// Test name filter pattern
    #[arg(value_name = "PATTERN")]
    filter: Option<String>,

    /// Watch mode (re-run on changes)
    #[arg(long, short = 'w')]
    watch: bool,

    /// Show coverage report
    #[arg(long)]
    coverage: bool,
}

impl TestCommand {
    pub fn execute(&self) -> Result<()> {
        let workspace = Workspace::load(".")?;
        let runner = TestRunner::new(workspace);

        let suites = runner.discover_tests(".")?;
        let filtered = self.filter_suites(&suites);

        let results = runner.run_suites(&filtered)?;

        self.display_results(&results);

        if results.has_failures() {
            std::process::exit(1);
        }

        Ok(())
    }
}
```

## Acceptance Criteria

- [ ] `maki test` runs all tests
- [ ] Filter by pattern works
- [ ] Watch mode detects changes
- [ ] Progress shows during execution
- [ ] Exit codes correct for CI/CD
- [ ] Coverage option works

---

**Status**: Ready for implementation
**Estimated Complexity**: Low
**Priority**: Medium
**Updated**: 2025-11-03
