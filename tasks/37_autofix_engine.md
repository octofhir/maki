# Task 37: Autofix Engine Enhancement

**Phase**: 2 (Enhanced Linter - Week 12)
**Time Estimate**: 3-4 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Tasks 01-29, Tasks 30-36 (All linting rules that provide fixes)

## Overview

Enhance the autofix engine to intelligently apply code fixes from linting rules, with sophisticated conflict detection, safety classification, and user-friendly preview capabilities. This task completes Phase 2 by enabling automatic correction of the 50+ lint rules implemented.

**Part of Enhanced Linter Phase**: Week 12 focuses on the autofix infrastructure that makes all the lint rules actionable.

## Context

Many linting violations can be automatically fixed:
- **Safe fixes**: No semantic changes (formatting, adding metadata)
- **Unsafe fixes**: Semantic changes (adding constraints, changing types)

The autofix engine must:
- Classify fixes by safety level
- Detect and resolve conflicts between overlapping fixes
- Provide clear previews before applying changes
- Support both safe-only and all-fixes modes

## Goals

1. **Implement safe/unsafe fix classification** - Clear safety levels for all fixes
2. **Build conflict detection system** - Handle overlapping text ranges
3. **Create fix prioritization logic** - Order fixes by importance
4. **Implement --fix flag** - Apply safe fixes only
5. **Implement --fix-unsafe flag** - Apply all fixes including unsafe
6. **Add dry-run preview mode** - Show changes before applying
7. **Integrate with all Phase 2 rules** - Connect 50+ rules to autofix

## Technical Specification

### Fix Safety Classification

```rust
/// Safety classification for automatic fixes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FixSafety {
    /// Safe: No semantic change
    /// Examples: formatting, adding required metadata, fixing typos
    Safe,

    /// Unsafe: Semantic change
    /// Examples: adding constraints, changing cardinality, adding bindings
    Unsafe,
}

impl FixSafety {
    pub fn description(&self) -> &str {
        match self {
            FixSafety::Safe => "no semantic change",
            FixSafety::Unsafe => "semantic change - review carefully",
        }
    }
}
```

### CodeSuggestion Structure (ACTUAL API)

**Note**: MAKI uses `CodeSuggestion` not `Fix` struct. This is the actual implementation API:

```rust
use maki_core::autofix::CodeSuggestion;
use maki_core::diagnostic::{Diagnostic, Severity};

/// CodeSuggestion represents an automatic fix
/// Already integrated into Diagnostic
impl CodeSuggestion {
    /// Create a safe fix (no semantic change)
    pub fn safe_fix(description: &str, replacement: String, location: DiagnosticLocation) -> Self {
        CodeSuggestion {
            description: description.to_string(),
            replacement,
            location,
            is_safe: true,
        }
    }

    /// Create an unsafe fix (semantic change)
    pub fn unsafe_fix(description: &str, replacement: String, location: DiagnosticLocation) -> Self {
        CodeSuggestion {
            description: description.to_string(),
            replacement,
            location,
            is_safe: false,
        }
    }

    /// Mark safe fix as unsafe
    pub fn as_unsafe(mut self) -> Self {
        self.is_safe = false;
        self
    }
}

/// Diagnostic includes suggestions
impl Diagnostic {
    pub fn with_suggestion(mut self, suggestion: CodeSuggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }
}
```

### Autofix Engine (ACTUAL IMPLEMENTATION)

```rust
use maki_core::autofix::AutofixEngine;
use maki_core::diagnostic::Diagnostic;

/// Main autofix engine for applying multiple fixes
pub struct AutofixEngine {
    diagnostics: Vec<Diagnostic>,
    source: String,
}

impl AutofixEngine {
    pub fn new(source: String) -> Self {
        Self {
            diagnostics: Vec::new(),
            source,
        }
    }

    /// Add diagnostics with suggestions
    pub fn add_diagnostics(&mut self, diags: Vec<Diagnostic>) {
        self.diagnostics.extend(diags);
    }

    /// Apply safe fixes only
    pub fn apply_safe_fixes(&self) -> Result<String> {
        self.apply_fixes(false)
    }

    /// Apply all fixes (safe + unsafe)
    pub fn apply_all_fixes(&self) -> Result<String> {
        self.apply_fixes(true)
    }

    /// Internal: apply fixes with safety filter
    fn apply_fixes(&self, include_unsafe: bool) -> Result<String> {
        let mut result = self.source.clone();
        let mut suggestions = Vec::new();

        // Collect suggestions
        for diag in &self.diagnostics {
            for suggestion in &diag.suggestions {
                if include_unsafe || suggestion.is_safe {
                    suggestions.push(suggestion.clone());
                }
            }
        }

        // Sort by location (reverse to avoid offset shifts)
        suggestions.sort_by(|a, b| b.location.start_offset().cmp(&a.location.start_offset()));

        // Check for conflicts
        self.detect_conflicts(&suggestions)?;

        // Apply fixes in reverse order (to preserve offsets)
        for suggestion in suggestions {
            let start = suggestion.location.start_offset();
            let end = suggestion.location.end_offset();
            result.replace_range(start..end, &suggestion.replacement);
        }

        Ok(result)
    }

    /// Detect overlapping fixes and report conflicts
    fn detect_conflicts(&self, suggestions: &[&CodeSuggestion]) -> Result<()> {
        for i in 0..suggestions.len() {
            for j in (i + 1)..suggestions.len() {
                let a = suggestions[i];
                let b = suggestions[j];

                if a.location.overlaps(&b.location) {
                    return Err(format!(
                        "Conflicting fixes: '{}' overlaps with '{}'",
                        a.description, b.description
                    ));
                }
            }
        }
        Ok(())
    }
}
```

**Integration with Rule Engine:**
```rust
// In rule engine (crates/maki-rules/src/engine.rs)
pub fn check_all_rules(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Run all rules
    diagnostics.extend(check_naming_conventions(model));
    diagnostics.extend(check_required_parent(model));
    diagnostics.extend(check_required_id(model));
    // ... more rules ...

    diagnostics
}

// In CLI (crates/maki-cli/src/commands/lint.rs)
if args.fix || args.fix_unsafe {
    let mut engine = AutofixEngine::new(source.clone());
    engine.add_diagnostics(diagnostics);

    let fixed = if args.fix_unsafe {
        engine.apply_all_fixes()?
    } else {
        engine.apply_safe_fixes()?
    };

    std::fs::write(file_path, fixed)?;
}
```

    /// Apply only safe fixes
    pub fn apply_safe_fixes(&self) -> Result<String> {
        let safe_fixes: Vec<_> = self.fixes.iter()
            .filter(|f| f.safety == FixSafety::Safe)
            .collect();

        self.apply_fixes_internal(&safe_fixes)
    }

    /// Apply all fixes (safe and unsafe)
    pub fn apply_all_fixes(&self) -> Result<String> {
        self.apply_fixes_internal(&self.fixes.iter().collect::<Vec<_>>())
    }

    /// Preview fixes without applying
    pub fn preview_fixes(&self, safety_level: Option<FixSafety>) -> String {
        let fixes_to_preview = match safety_level {
            Some(FixSafety::Safe) => self.fixes.iter()
                .filter(|f| f.safety == FixSafety::Safe)
                .collect(),
            Some(FixSafety::Unsafe) => self.fixes.iter().collect(),
            None => self.fixes.iter().collect(),
        };

        self.generate_preview(&fixes_to_preview)
    }

    /// Internal: Apply a set of fixes
    fn apply_fixes_internal(&self, fixes: &[&Fix]) -> Result<String> {
        // Step 1: Detect conflicts
        let resolved_fixes = self.resolve_conflicts(fixes)?;

        // Step 2: Sort by range (reverse order for easier application)
        let mut sorted_fixes = resolved_fixes.clone();
        sorted_fixes.sort_by(|a, b| b.range.start().cmp(&a.range.start()));

        // Step 3: Apply fixes
        let mut result = self.source.clone();
        for fix in sorted_fixes {
            let start = fix.range.start().into();
            let end = fix.range.end().into();
            result.replace_range(start..end, &fix.replacement);
        }

        Ok(result)
    }

    /// Detect and resolve conflicts between overlapping fixes
    fn resolve_conflicts(&self, fixes: &[&Fix]) -> Result<Vec<Fix>> {
        let mut resolved = Vec::new();
        let mut used_ranges: Vec<TextRange> = Vec::new();

        // Sort by priority (highest first)
        let mut sorted_fixes = fixes.to_vec();
        sorted_fixes.sort_by(|a, b| b.priority.cmp(&a.priority));

        for fix in sorted_fixes {
            // Check if this fix overlaps with any already selected fix
            let overlaps = used_ranges.iter()
                .any(|range| ranges_overlap(fix.range, *range));

            if overlaps {
                // Skip this fix due to conflict
                eprintln!(
                    "Warning: Skipping fix '{}' due to conflict",
                    fix.description
                );
                continue;
            }

            resolved.push((*fix).clone());
            used_ranges.push(fix.range);
        }

        Ok(resolved)
    }

    /// Generate preview showing what will change
    fn generate_preview(&self, fixes: &[&Fix]) -> String {
        let mut output = String::new();

        writeln!(&mut output, "Autofix Preview for: {}", self.file_path.display()).unwrap();
        writeln!(&mut output, "{}", "=".repeat(80)).unwrap();
        writeln!(&mut output).unwrap();

        for (i, fix) in fixes.iter().enumerate() {
            writeln!(&mut output, "Fix {}: {}", i + 1, fix.description).unwrap();
            writeln!(&mut output, "  Safety: {}", fix.safety.description()).unwrap();
            writeln!(&mut output, "  Range: {:?}", fix.range).unwrap();
            writeln!(&mut output).unwrap();

            // Show diff
            let original = &self.source[fix.range.start().into()..fix.range.end().into()];
            writeln!(&mut output, "  - {}", original).unwrap();
            writeln!(&mut output, "  + {}", fix.replacement).unwrap();
            writeln!(&mut output).unwrap();
        }

        writeln!(
            &mut output,
            "Total fixes: {} (Safe: {}, Unsafe: {})",
            fixes.len(),
            fixes.iter().filter(|f| f.safety == FixSafety::Safe).count(),
            fixes.iter().filter(|f| f.safety == FixSafety::Unsafe).count()
        ).unwrap();

        output
    }
}

fn ranges_overlap(r1: TextRange, r2: TextRange) -> bool {
    r1.start() < r2.end() && r2.start() < r1.end()
}
```

### CLI Integration

```rust
// In maki-cli/src/commands/lint.rs

#[derive(Parser)]
pub struct LintCommand {
    /// Apply safe fixes automatically
    #[arg(long)]
    fix: bool,

    /// Apply all fixes (including unsafe)
    #[arg(long)]
    fix_unsafe: bool,

    /// Preview fixes without applying
    #[arg(long)]
    dry_run: bool,

    // ... other flags
}

impl LintCommand {
    pub fn execute(&self) -> Result<()> {
        // Run linter and collect diagnostics
        let diagnostics = lint_files(&self.paths)?;

        // Extract fixes from diagnostics
        let mut engine = AutofixEngine::new(source, file_path);
        for diagnostic in &diagnostics {
            if let Some(fix) = diagnostic.fix() {
                engine.add_fix(fix.clone());
            }
        }

        // Handle fix modes
        if self.dry_run {
            // Show preview only
            let preview = if self.fix_unsafe {
                engine.preview_fixes(Some(FixSafety::Unsafe))
            } else {
                engine.preview_fixes(Some(FixSafety::Safe))
            };
            println!("{}", preview);
        } else if self.fix || self.fix_unsafe {
            // Apply fixes
            let fixed_source = if self.fix_unsafe {
                engine.apply_all_fixes()?
            } else {
                engine.apply_safe_fixes()?
            };

            // Write back to file
            fs::write(&file_path, fixed_source)?;

            println!("Applied {} fixes to {}", engine.fixes.len(), file_path.display());
        } else {
            // Just show diagnostics
            print_diagnostics(&diagnostics);
        }

        Ok(())
    }
}
```

## Examples of Fixes by Safety Level

### Safe Fixes

```rust
// 1. Adding missing metadata (required-id rule)
Fix {
    range: TextRange::new(10, 10),
    replacement: "\nId: my-patient-profile".to_string(),
    safety: FixSafety::Safe,
    description: "Add missing Id".to_string(),
    priority: FixPriority::Critical,
}

// 2. Fixing naming convention (naming-conventions rule)
Fix {
    range: TextRange::new(50, 70),
    replacement: "my-patient-profile".to_string(),
    safety: FixSafety::Safe,
    description: "Convert ID to kebab-case".to_string(),
    priority: FixPriority::Medium,
}

// 3. Removing unused alias (unused-alias rule)
Fix {
    range: TextRange::new(0, 35),
    replacement: "".to_string(),
    safety: FixSafety::Safe,
    description: "Remove unused alias".to_string(),
    priority: FixPriority::Low,
}
```

### Unsafe Fixes

```rust
// 1. Adding binding strength (binding-strength-required rule)
Fix {
    range: TextRange::new(100, 100),
    replacement: " (required)".to_string(),
    safety: FixSafety::Unsafe,  // Adds semantic constraint
    description: "Add binding strength (required)".to_string(),
    priority: FixPriority::Critical,
}

// 2. Adding extension context (extension-context-required rule)
Fix {
    range: TextRange::new(200, 200),
    replacement: "\n* ^context[+].type = #element\n* ^context[=].expression = \"Patient\"".to_string(),
    safety: FixSafety::Unsafe,  // Adds semantic constraint
    description: "Add extension context".to_string(),
    priority: FixPriority::Critical,
}

// 3. Swapping cardinality (valid-cardinality rule)
Fix {
    range: TextRange::new(150, 155),
    replacement: "3..5".to_string(),
    safety: FixSafety::Unsafe,  // Changes cardinality
    description: "Swap min and max".to_string(),
    priority: FixPriority::Critical,
}
```

## Implementation Location

**Primary File**: `crates/maki-core/src/autofix.rs` (extend existing)

**Additional Files**:
- `crates/maki-cli/src/commands/lint.rs` - CLI integration
- Update all rule files to include Fix objects in diagnostics

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_apply_safe_fixes_only() {
    let source = "Profile: my_profile\nId: MyProfile";
    let mut engine = AutofixEngine::new(source.to_string(), PathBuf::from("test.fsh"));

    // Add safe fix
    engine.add_fix(Fix {
        range: TextRange::new(9, 19),
        replacement: "MyProfile".to_string(),
        safety: FixSafety::Safe,
        description: "Fix naming".to_string(),
        priority: FixPriority::Medium,
    });

    // Add unsafe fix
    engine.add_fix(Fix {
        range: TextRange::new(29, 29),
        replacement: " (required)".to_string(),
        safety: FixSafety::Unsafe,
        description: "Add binding".to_string(),
        priority: FixPriority::Critical,
    });

    let result = engine.apply_safe_fixes().unwrap();
    assert!(result.contains("Profile: MyProfile"));
    assert!(!result.contains("(required)"));
}

#[test]
fn test_conflict_detection() {
    let source = "Profile: Test\n* name 1..*";
    let mut engine = AutofixEngine::new(source.to_string(), PathBuf::from("test.fsh"));

    // Two fixes that overlap
    engine.add_fix(Fix {
        range: TextRange::new(16, 21),
        replacement: "0..1".to_string(),
        safety: FixSafety::Unsafe,
        description: "Fix 1".to_string(),
        priority: FixPriority::High,
    });

    engine.add_fix(Fix {
        range: TextRange::new(18, 21),
        replacement: "5".to_string(),
        safety: FixSafety::Safe,
        description: "Fix 2".to_string(),
        priority: FixPriority::Low,
    });

    // Higher priority fix should win
    let result = engine.apply_all_fixes().unwrap();
    assert!(result.contains("0..1"));
}

#[test]
fn test_preview_generation() {
    let source = "Profile: Test";
    let mut engine = AutofixEngine::new(source.to_string(), PathBuf::from("test.fsh"));

    engine.add_fix(Fix {
        range: TextRange::new(9, 13),
        replacement: "MyTest".to_string(),
        safety: FixSafety::Safe,
        description: "Rename".to_string(),
        priority: FixPriority::Medium,
    });

    let preview = engine.preview_fixes(None);
    assert!(preview.contains("Autofix Preview"));
    assert!(preview.contains("Rename"));
    assert!(preview.contains("- Test"));
    assert!(preview.contains("+ MyTest"));
}
```

### Integration Tests

```bash
# Test safe fixes only
maki lint examples/ --fix
# Should only apply safe fixes

# Test all fixes
maki lint examples/ --fix-unsafe
# Should apply all fixes

# Test dry-run
maki lint examples/ --fix --dry-run
# Should show preview without modifying files

# Test that files are actually modified
maki lint examples/bad-names.fsh --fix
diff examples/bad-names.fsh examples/bad-names.fsh.expected
```

## Configuration

```toml
[autofix]
# Default safety level for --fix flag
default_safety = "safe"  # Options: safe, unsafe

# Show preview before applying (interactive mode)
interactive = false

# Create backup files before modifying
create_backups = true
backup_extension = ".bak"

# Maximum number of fixes to apply in one run
max_fixes_per_file = 100
```

## CLI Usage

```bash
# Apply safe fixes only
maki lint --fix input/fsh/

# Apply all fixes (safe + unsafe)
maki lint --fix-unsafe input/fsh/

# Preview without applying
maki lint --fix --dry-run input/fsh/

# Preview unsafe fixes
maki lint --fix-unsafe --dry-run input/fsh/

# Specific file
maki lint --fix profiles/Patient.fsh
```

## Dependencies

### Required Components
- **All Phase 2 Rules** (Tasks 30-36): Provide Fix objects
- **Diagnostic System** (Task 06): Fix attachment to diagnostics
- **File I/O**: Reading and writing fixed files

## Acceptance Criteria

- [ ] `FixSafety` enum correctly classifies fixes
- [ ] `Fix` struct contains all necessary information
- [ ] `AutofixEngine` applies safe fixes only with `--fix`
- [ ] `AutofixEngine` applies all fixes with `--fix-unsafe`
- [ ] Conflict detection identifies overlapping ranges
- [ ] Priority-based conflict resolution works correctly
- [ ] Preview mode shows clear diff output
- [ ] Dry-run mode doesn't modify files
- [ ] All Phase 2 rules integrated with autofix
- [ ] CLI flags work as expected
- [ ] Configuration file supports autofix options
- [ ] Unit tests cover conflict scenarios
- [ ] Integration tests verify file modifications
- [ ] Performance: <100ms for 50 fixes

## Future Enhancements

1. **Interactive mode**: Prompt user for each fix
2. **Batch fixing**: Apply same fix across multiple files
3. **Undo/rollback**: Revert applied fixes
4. **Fix statistics**: Show success rate and common fixes
5. **Custom fix plugins**: Allow users to define custom fixes

## Resources

- **Rust Error Handling**: https://doc.rust-lang.org/book/ch09-00-error-handling.html
- **Text Range Manipulation**: Rowan library documentation

## Related Tasks

- **All Tasks 30-36**: Provide fixes for autofix engine
- **Task 38: Formatter**: Formatting is a type of safe autofix
- **Task 47: Code Actions** (LSP): Uses same Fix infrastructure

---

**Status**: Ready for implementation (extends existing autofix.rs)
**Estimated Complexity**: High (requires careful conflict handling)
**Priority**: High (enables all lint rules to be actionable)
**Updated**: 2025-11-03
