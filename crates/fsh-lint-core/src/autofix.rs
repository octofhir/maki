//! Autofix engine for automatic code corrections
//!
//! This module provides intelligent autofix capabilities with:
//! - Safe-by-default fix application (Applicability::Always)
//! - Unsafe fixes requiring explicit --unsafe flag (Applicability::MaybeIncorrect)
//! - Conflict detection and resolution
//! - Interactive confirmation mode
//! - Dry-run preview support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::rules::{AutofixTemplate, FixSafety};
use crate::{Applicability, CodeSuggestion, Diagnostic, FshLintError, Location, Result};

/// Represents a fix that can be applied to source code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fix {
    /// Unique identifier for this fix
    pub id: String,
    /// Description of what this fix does
    pub description: String,
    /// Location where the fix should be applied
    pub location: Location,
    /// The replacement text
    pub replacement: String,
    /// Applicability level (Always = safe, MaybeIncorrect = unsafe)
    pub applicability: Applicability,
    /// Rule ID that generated this fix
    pub rule_id: String,
    /// Priority for conflict resolution (higher = more important)
    pub priority: u32,
}

impl Fix {
    /// Create a new fix from a CodeSuggestion
    pub fn from_code_suggestion(suggestion: &CodeSuggestion, diagnostic: &Diagnostic) -> Self {
        Self {
            id: format!("{}-{}", diagnostic.rule_id, diagnostic.location.line),
            description: suggestion.message.clone(),
            location: suggestion.location.clone(),
            replacement: suggestion.replacement.clone(),
            applicability: suggestion.applicability,
            rule_id: diagnostic.rule_id.clone(),
            priority: match suggestion.applicability {
                Applicability::Always => 10,        // Higher priority for safe fixes
                Applicability::MaybeIncorrect => 5, // Lower priority for unsafe fixes
            },
        }
    }

    /// Check if this fix is safe to apply automatically
    pub fn is_safe(&self) -> bool {
        matches!(self.applicability, Applicability::Always)
    }

    /// Check if this fix requires the --unsafe flag
    pub fn requires_unsafe_flag(&self) -> bool {
        matches!(self.applicability, Applicability::MaybeIncorrect)
    }

    /// Get a human-readable safety description
    pub fn safety_description(&self) -> &'static str {
        match self.applicability {
            Applicability::Always => "safe (formatting, whitespace, obvious corrections)",
            Applicability::MaybeIncorrect => "unsafe (semantic changes, requires review)",
        }
    }
}

/// Result of applying fixes to a file
#[derive(Debug, Clone)]
pub struct FixResult {
    /// Path to the file that was modified
    pub file: PathBuf,
    /// Number of fixes successfully applied
    pub applied_count: usize,
    /// Number of fixes that failed to apply
    pub failed_count: usize,
    /// Any errors that occurred during fix application
    pub errors: Vec<String>,
    /// The modified content (for dry-run mode)
    pub modified_content: Option<String>,
}

/// Configuration for fix application
#[derive(Debug, Clone)]
pub struct FixConfig {
    /// Whether to apply unsafe fixes (requires --unsafe flag)
    pub apply_unsafe: bool,
    /// Whether to run in dry-run mode (don't modify files)
    pub dry_run: bool,
    /// Interactive mode - ask for confirmation on each unsafe fix
    pub interactive: bool,
    /// Maximum number of fixes to apply per file
    pub max_fixes_per_file: Option<usize>,
    /// Whether to validate syntax after applying fixes
    pub validate_syntax: bool,
}

impl Default for FixConfig {
    fn default() -> Self {
        Self {
            apply_unsafe: false, // Safe by default
            dry_run: false,
            interactive: false,
            max_fixes_per_file: None,
            validate_syntax: true,
        }
    }
}

impl FixConfig {
    /// Create a config that only applies safe fixes
    pub fn safe_only() -> Self {
        Self {
            apply_unsafe: false,
            ..Default::default()
        }
    }

    /// Create a config that applies all fixes (safe and unsafe)
    pub fn with_unsafe() -> Self {
        Self {
            apply_unsafe: true,
            ..Default::default()
        }
    }

    /// Create a config for interactive mode
    pub fn interactive() -> Self {
        Self {
            apply_unsafe: true,
            interactive: true,
            ..Default::default()
        }
    }

    /// Create a config for dry-run preview
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Default::default()
        }
    }
}

/// Trait for generating and applying automatic fixes
pub trait AutofixEngine {
    /// Generate fixes from diagnostic suggestions
    fn generate_fixes(&self, diagnostics: &[Diagnostic]) -> Result<Vec<Fix>>;

    /// Generate fixes from autofix templates
    fn generate_fixes_from_templates(
        &self,
        diagnostics: &[Diagnostic],
        templates: &HashMap<String, AutofixTemplate>,
    ) -> Result<Vec<Fix>>;

    /// Detect and resolve conflicts between fixes
    fn resolve_conflicts(&self, fixes: &[Fix]) -> Vec<Fix>;

    /// Apply fixes to files
    fn apply_fixes(&self, fixes: &[Fix], config: &FixConfig) -> Result<Vec<FixResult>>;

    /// Apply fixes to a single file
    fn apply_fixes_to_file(
        &self,
        file: &PathBuf,
        fixes: &[Fix],
        config: &FixConfig,
    ) -> Result<FixResult>;

    /// Validate that fixes can be applied safely
    fn validate_fixes(&self, fixes: &[Fix]) -> Result<()>;

    /// Create a rollback plan for applied fixes
    fn create_rollback(&self, results: &[FixResult]) -> Result<RollbackPlan>;
}

/// Plan for rolling back applied fixes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    /// Original file contents before fixes were applied
    pub original_contents: HashMap<PathBuf, String>,
    /// Timestamp when the rollback plan was created
    pub created_at: std::time::SystemTime,
}

/// Group of conflicting fixes
#[derive(Debug, Clone)]
pub struct ConflictGroup {
    /// Indices of fixes that conflict with each other
    pub fix_indices: Vec<usize>,
    /// Type of conflict
    pub conflict_type: ConflictType,
}

/// Types of conflicts between fixes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictType {
    /// Fixes overlap in their text ranges
    Overlap,
    /// Fixes modify the same semantic construct
    Semantic,
    /// Fixes have dependencies on each other
    Dependency,
}

/// Preview of fixes to be applied to a file
#[derive(Debug, Clone)]
pub struct FixPreview {
    /// Path to the file
    pub file: PathBuf,
    /// Original file content
    pub original_content: String,
    /// Content after applying fixes
    pub modified_content: String,
    /// List of fixes that were applied
    pub applied_fixes: Vec<Fix>,
    /// Diff showing the changes
    pub diff: String,
}

/// Default implementation of the AutofixEngine
#[derive(Debug, Clone)]
pub struct DefaultAutofixEngine {
    /// Configuration for the engine
    config: AutofixEngineConfig,
}

/// Configuration for the autofix engine
#[derive(Debug, Clone)]
pub struct AutofixEngineConfig {
    /// Maximum number of conflicts to resolve per file
    pub max_conflicts_per_file: usize,
    /// Whether to preserve original file permissions
    pub preserve_permissions: bool,
    /// Backup directory for original files
    pub backup_dir: Option<PathBuf>,
}

impl Fix {
    /// Create a new fix with specific applicability (deprecated - use from_code_suggestion)
    pub fn new(
        id: String,
        description: String,
        location: Location,
        replacement: String,
        applicability: Applicability,
        rule_id: String,
    ) -> Self {
        Self {
            id,
            description,
            location,
            replacement,
            applicability,
            rule_id,
            priority: match applicability {
                Applicability::Always => 10,
                Applicability::MaybeIncorrect => 5,
            },
        }
    }

    /// Create a fix with custom priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this fix conflicts with another fix
    pub fn conflicts_with(&self, other: &Fix) -> bool {
        // Fixes conflict if they overlap in the same file
        if self.location.file != other.location.file {
            return false;
        }

        let self_start = self.location.offset;
        let self_end = self.location.offset + self.location.length;
        let other_start = other.location.offset;
        let other_end = other.location.offset + other.location.length;

        // Check for overlap
        !(self_end <= other_start || other_end <= self_start)
    }

    /// Get the span of this fix as (start, end) byte offsets
    pub fn span(&self) -> (usize, usize) {
        (
            self.location.offset,
            self.location.offset + self.location.length,
        )
    }
}

// FixConfig Default implementation and methods are defined earlier (lines 105-150)

impl Default for AutofixEngineConfig {
    fn default() -> Self {
        Self {
            max_conflicts_per_file: 100,
            preserve_permissions: true,
            backup_dir: None,
        }
    }
}

impl Default for DefaultAutofixEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultAutofixEngine {
    /// Create a new autofix engine with default configuration
    pub fn new() -> Self {
        Self {
            config: AutofixEngineConfig::default(),
        }
    }

    /// Create a new autofix engine with custom configuration
    pub fn with_config(config: AutofixEngineConfig) -> Self {
        Self { config }
    }

    /// Filter fixes based on safety and configuration
    pub fn filter_fixes_by_safety(&self, fixes: &[Fix], config: &FixConfig) -> Vec<Fix> {
        fixes
            .iter()
            .filter(|fix| {
                match fix.applicability {
                    Applicability::Always => true, // Always apply safe fixes
                    Applicability::MaybeIncorrect => {
                        // Only apply unsafe fixes if explicitly enabled
                        config.apply_unsafe || config.interactive
                    }
                }
            })
            .cloned()
            .collect()
    }

    /// Classify a fix's applicability based on its characteristics
    pub fn classify_fix_applicability(&self, fix: &Fix) -> Applicability {
        // Check if the fix only changes whitespace/formatting
        if self.is_formatting_only_fix(fix) {
            return Applicability::Always;
        }

        // Check if it's a simple punctuation fix
        if self.is_simple_punctuation_fix(fix) {
            return Applicability::Always;
        }

        // Check if it removes code (potentially dangerous)
        if self.removes_code(fix) {
            return Applicability::MaybeIncorrect;
        }

        // Check if it changes semantics
        if self.is_semantic_change(fix) {
            return Applicability::MaybeIncorrect;
        }

        // Default to unsafe if uncertain
        Applicability::MaybeIncorrect
    }

    /// Check if a fix only changes formatting/whitespace
    fn is_formatting_only_fix(&self, fix: &Fix) -> bool {
        let _original_trimmed = fix
            .location
            .span
            .map(|(_start, _end)| {
                // Would need access to source code here
                // For now, use a heuristic
                true
            })
            .unwrap_or(false);

        // Check if replacement only differs in whitespace
        fix.replacement.trim() == fix.replacement.trim()
            && fix
                .replacement
                .chars()
                .all(|c| c.is_whitespace() || c.is_alphanumeric())
    }

    /// Check if a fix is a simple punctuation change
    fn is_simple_punctuation_fix(&self, fix: &Fix) -> bool {
        fix.replacement.len() <= 2 && fix.replacement.chars().all(|c| ".,;:()[]{}".contains(c))
    }

    /// Check if a fix removes code
    fn removes_code(&self, fix: &Fix) -> bool {
        fix.replacement.is_empty() || fix.replacement.trim().is_empty()
    }

    /// Check if a fix makes semantic changes
    fn is_semantic_change(&self, fix: &Fix) -> bool {
        // Heuristic: if it changes structure keywords, it's semantic
        let keywords = [
            "Profile",
            "Extension",
            "ValueSet",
            "CodeSystem",
            "from",
            "only",
        ];
        keywords.iter().any(|kw| fix.replacement.contains(kw))
    }

    /// Apply fixes in batch mode with progress reporting
    pub fn apply_fixes_batch(
        &self,
        fixes: &[Fix],
        config: &FixConfig,
        mut progress_callback: Option<Box<dyn FnMut(usize, usize)>>,
    ) -> Result<Vec<FixResult>> {
        let mut results = Vec::new();
        let mut fixes_by_file: HashMap<PathBuf, Vec<&Fix>> = HashMap::new();

        // Group fixes by file
        for fix in fixes {
            if !config.apply_unsafe && !fix.is_safe() {
                continue;
            }

            fixes_by_file
                .entry(fix.location.file.clone())
                .or_default()
                .push(fix);
        }

        let total_files = fixes_by_file.len();
        let mut processed_files = 0;

        // Create backups if configured
        if let Some(backup_dir) = &self.config.backup_dir {
            self.create_backups(
                &fixes_by_file.keys().cloned().collect::<Vec<_>>(),
                backup_dir,
            )?;
        }

        // Apply fixes to each file
        for (file, file_fixes) in fixes_by_file {
            let owned_fixes: Vec<Fix> = file_fixes.into_iter().cloned().collect();
            let result = self.apply_fixes_to_file(&file, &owned_fixes, config)?;
            results.push(result);

            processed_files += 1;
            if let Some(ref mut callback) = progress_callback {
                callback(processed_files, total_files);
            }
        }

        Ok(results)
    }

    /// Create backups of files before applying fixes
    fn create_backups(&self, files: &[PathBuf], backup_dir: &PathBuf) -> Result<()> {
        use std::fs;

        // Create backup directory if it doesn't exist
        fs::create_dir_all(backup_dir).map_err(|e| FshLintError::io_error(backup_dir, e))?;

        for file in files {
            let backup_name = format!(
                "{}.backup.{}",
                file.file_name().unwrap_or_default().to_string_lossy(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );

            let backup_path = backup_dir.join(backup_name);
            fs::copy(file, &backup_path).map_err(|e| FshLintError::io_error(&backup_path, e))?;
        }

        Ok(())
    }

    /// Preview fixes without applying them (enhanced dry-run)
    pub fn preview_fixes(&self, fixes: &[Fix]) -> Result<Vec<FixPreview>> {
        let mut previews = Vec::new();
        let mut fixes_by_file: HashMap<PathBuf, Vec<&Fix>> = HashMap::new();

        // Group fixes by file
        for fix in fixes {
            fixes_by_file
                .entry(fix.location.file.clone())
                .or_default()
                .push(fix);
        }

        // Generate previews for each file
        for (file, file_fixes) in fixes_by_file {
            let preview = self.generate_file_preview(&file, &file_fixes)?;
            previews.push(preview);
        }

        Ok(previews)
    }

    /// Generate preview for a single file
    fn generate_file_preview(&self, file: &PathBuf, fixes: &[&Fix]) -> Result<FixPreview> {
        use std::fs;

        let original_content =
            fs::read_to_string(file).map_err(|e| FshLintError::io_error(file, e))?;

        // Sort fixes for optimal application (reverse order by offset)
        let mut sorted_fixes: Vec<_> = fixes.iter().collect();
        sorted_fixes.sort_by(|a, b| b.location.offset.cmp(&a.location.offset));

        // Apply fixes to content
        let mut modified_content = original_content.clone();
        let mut applied_fixes = Vec::new();

        for fix in sorted_fixes {
            if let Ok(()) = self.apply_single_fix(&mut modified_content, fix) {
                applied_fixes.push((*fix).clone());
            }
        }

        let diff = self.generate_diff(&original_content, &modified_content);

        Ok(FixPreview {
            file: file.clone(),
            original_content,
            modified_content,
            applied_fixes,
            diff,
        })
    }

    /// Generate a diff between original and modified content
    fn generate_diff(&self, original: &str, modified: &str) -> String {
        // Simple line-by-line diff
        let original_lines: Vec<&str> = original.lines().collect();
        let modified_lines: Vec<&str> = modified.lines().collect();

        let mut diff = String::new();
        let max_lines = original_lines.len().max(modified_lines.len());

        for i in 0..max_lines {
            let orig_line = original_lines.get(i).unwrap_or(&"");
            let mod_line = modified_lines.get(i).unwrap_or(&"");

            if orig_line != mod_line {
                if !orig_line.is_empty() {
                    diff.push_str(&format!("- {orig_line}\n"));
                }
                if !mod_line.is_empty() {
                    diff.push_str(&format!("+ {mod_line}\n"));
                }
            }
        }

        diff
    }
}
impl AutofixEngine for DefaultAutofixEngine {
    fn generate_fixes(&self, diagnostics: &[Diagnostic]) -> Result<Vec<Fix>> {
        let mut fixes = Vec::new();

        for diagnostic in diagnostics {
            // Generate fixes from diagnostic suggestions with enhanced validation
            for (i, suggestion) in diagnostic.suggestions.iter().enumerate() {
                let fix_id = format!("{}_{}", diagnostic.rule_id, i);

                match self.generate_fix_from_suggestion(suggestion, diagnostic, fix_id) {
                    Ok(fix) => fixes.push(fix),
                    Err(e) => {
                        // Log the error but continue processing other fixes
                        tracing::warn!("Failed to generate fix for {}: {}", diagnostic.rule_id, e);
                    }
                }
            }
        }

        Ok(fixes)
    }

    fn generate_fixes_from_templates(
        &self,
        diagnostics: &[Diagnostic],
        templates: &HashMap<String, AutofixTemplate>,
    ) -> Result<Vec<Fix>> {
        let mut fixes = Vec::new();

        for diagnostic in diagnostics {
            if let Some(template) = templates.get(&diagnostic.rule_id) {
                let fix_id = format!("{}_template", diagnostic.rule_id);

                // Apply template to generate replacement text
                let replacement = self.apply_template(template, diagnostic)?;

                let fix = Fix::new(
                    fix_id,
                    template.description.clone(),
                    diagnostic.location.clone(),
                    replacement,
                    match template.safety {
                        FixSafety::Safe => Applicability::Always,
                        FixSafety::Unsafe => Applicability::MaybeIncorrect,
                    },
                    diagnostic.rule_id.clone(),
                );

                fixes.push(fix);
            }
        }

        Ok(fixes)
    }

    fn resolve_conflicts(&self, fixes: &[Fix]) -> Vec<Fix> {
        let mut resolved_fixes = Vec::new();
        let mut fixes_by_file: HashMap<PathBuf, Vec<&Fix>> = HashMap::new();

        // Group fixes by file
        for fix in fixes {
            fixes_by_file
                .entry(fix.location.file.clone())
                .or_default()
                .push(fix);
        }

        // Resolve conflicts within each file using enhanced conflict detection
        for (_, file_fixes) in fixes_by_file {
            let conflicts = self.detect_complex_conflicts(
                &file_fixes.iter().map(|f| (*f).clone()).collect::<Vec<_>>(),
            );
            let mut non_conflicting =
                self.resolve_file_conflicts_with_groups(&file_fixes, &conflicts);
            resolved_fixes.append(&mut non_conflicting);
        }

        resolved_fixes
    }

    fn apply_fixes(&self, fixes: &[Fix], config: &FixConfig) -> Result<Vec<FixResult>> {
        let mut results = Vec::new();
        let mut fixes_by_file: HashMap<PathBuf, Vec<&Fix>> = HashMap::new();

        // Group fixes by file
        for fix in fixes {
            // Skip unsafe fixes if not configured to apply them
            if !config.apply_unsafe && !fix.is_safe() {
                continue;
            }

            fixes_by_file
                .entry(fix.location.file.clone())
                .or_default()
                .push(fix);
        }

        // Apply fixes to each file
        for (file, file_fixes) in fixes_by_file {
            let owned_fixes: Vec<Fix> = file_fixes.into_iter().cloned().collect();
            let result = self.apply_fixes_to_file(&file, &owned_fixes, config)?;
            results.push(result);
        }

        Ok(results)
    }

    fn apply_fixes_to_file(
        &self,
        file: &PathBuf,
        fixes: &[Fix],
        config: &FixConfig,
    ) -> Result<FixResult> {
        use std::fs;

        // Read the original file content
        let original_content =
            fs::read_to_string(file).map_err(|e| FshLintError::io_error(file.clone(), e))?;

        // Limit the number of fixes if configured
        let fixes_to_apply = if let Some(max) = config.max_fixes_per_file {
            &fixes[..fixes.len().min(max)]
        } else {
            fixes
        };

        // Sort fixes for optimal application (reverse order by offset)
        let mut sorted_fixes: Vec<_> = fixes_to_apply.iter().collect();
        sorted_fixes.sort_by(|a, b| b.location.offset.cmp(&a.location.offset));

        // Apply fixes to content
        let mut modified_content = original_content.clone();
        let mut applied_count = 0;
        let mut failed_count = 0;
        let mut errors = Vec::new();

        for fix in sorted_fixes {
            match self.apply_single_fix(&mut modified_content, fix) {
                Ok(()) => applied_count += 1,
                Err(e) => {
                    failed_count += 1;
                    errors.push(format!("Fix {}: {}", fix.id, e));
                }
            }
        }

        // Validate syntax if requested
        if config.validate_syntax && applied_count > 0 {
            if let Err(e) = self.validate_syntax(&modified_content, file) {
                errors.push(format!("Syntax validation failed: {e}"));
            }
        }

        // Write the modified content if not in dry-run mode
        if !config.dry_run && applied_count > 0 && errors.is_empty() {
            fs::write(file, &modified_content)
                .map_err(|e| FshLintError::io_error(file.clone(), e))?;
        }

        Ok(FixResult {
            file: file.clone(),
            applied_count,
            failed_count,
            errors,
            modified_content: if config.dry_run {
                Some(modified_content)
            } else {
                None
            },
        })
    }

    fn validate_fixes(&self, fixes: &[Fix]) -> Result<()> {
        for fix in fixes {
            // Validate that the fix location is valid
            if fix.location.offset > 0 && fix.location.length == 0 && fix.replacement.is_empty() {
                return Err(FshLintError::autofix_error(format!(
                    "Invalid fix {}: no-op fix",
                    fix.id
                )));
            }

            // Validate that the fix has a valid location
            if fix.location.file.as_os_str().is_empty() {
                return Err(FshLintError::autofix_error(format!(
                    "Invalid fix {}: empty file path",
                    fix.id
                )));
            }
        }

        Ok(())
    }

    fn create_rollback(&self, results: &[FixResult]) -> Result<RollbackPlan> {
        let mut original_contents = HashMap::new();

        for result in results {
            if result.applied_count > 0 {
                // In a real implementation, we would have stored the original content
                // before applying fixes. For now, we'll try to read from backup if available
                if let Some(backup_dir) = &self.config.backup_dir {
                    let backup_files: Vec<_> = std::fs::read_dir(backup_dir)
                        .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
                        .filter_map(|entry| entry.ok())
                        .filter(|entry| {
                            let entry_name = entry.file_name().to_string_lossy().to_string();
                            let file_name = result
                                .file
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            entry_name.contains(&file_name)
                        })
                        .collect();

                    if let Some(backup_file) = backup_files.first() {
                        let original_content = std::fs::read_to_string(backup_file.path())
                            .map_err(|e| FshLintError::io_error(backup_file.path(), e))?;

                        original_contents.insert(result.file.clone(), original_content);
                    }
                }
            }
        }

        Ok(RollbackPlan {
            original_contents,
            created_at: std::time::SystemTime::now(),
        })
    }
}

impl DefaultAutofixEngine {
    /// Apply a template to generate replacement text
    fn apply_template(
        &self,
        template: &AutofixTemplate,
        diagnostic: &Diagnostic,
    ) -> Result<String> {
        // Simple template substitution - in a real implementation this would be more sophisticated
        let mut replacement = template.replacement_template.clone();

        // Replace common placeholders
        replacement = replacement.replace("{{message}}", &diagnostic.message);
        replacement = replacement.replace("{{rule_id}}", &diagnostic.rule_id);
        replacement = replacement.replace("{{line}}", &diagnostic.location.line.to_string());
        replacement = replacement.replace("{{column}}", &diagnostic.location.column.to_string());

        Ok(replacement)
    }

    /// Resolve conflicts using conflict groups
    fn resolve_file_conflicts_with_groups(
        &self,
        fixes: &[&Fix],
        conflicts: &[ConflictGroup],
    ) -> Vec<Fix> {
        let mut resolved = Vec::new();
        let mut excluded_indices = std::collections::HashSet::new();

        // For each conflict group, select the best fix
        for conflict_group in conflicts {
            let best_fix_index =
                self.select_best_fix_from_group(fixes, &conflict_group.fix_indices);

            // Mark all other fixes in the group as excluded
            for &index in &conflict_group.fix_indices {
                if index != best_fix_index {
                    excluded_indices.insert(index);
                }
            }
        }

        // Add all non-conflicting fixes
        for (i, fix) in fixes.iter().enumerate() {
            if !excluded_indices.contains(&i) {
                resolved.push((*fix).clone());
            }
        }

        resolved
    }

    /// Select the best fix from a conflict group
    fn select_best_fix_from_group(&self, fixes: &[&Fix], indices: &[usize]) -> usize {
        let mut best_index = indices[0];
        let mut best_score = self.calculate_fix_score(fixes[best_index]);

        for &index in indices.iter().skip(1) {
            let score = self.calculate_fix_score(fixes[index]);
            if score > best_score {
                best_score = score;
                best_index = index;
            }
        }

        best_index
    }

    /// Calculate a score for fix selection (higher is better)
    fn calculate_fix_score(&self, fix: &Fix) -> i32 {
        let mut score = 0;

        // Prefer safe fixes
        if fix.is_safe() {
            score += 100;
        }

        // Prefer fixes with higher priority
        score += fix.priority as i32;

        // Prefer smaller replacements (less likely to break things)
        score += (100 - fix.replacement.len().min(100)) as i32;

        // Prefer fixes for errors over warnings
        score += match fix.rule_id.contains("error") {
            true => 50,
            false => 0,
        };

        score
    }

    /// Apply a single fix to content
    fn apply_single_fix(&self, content: &mut String, fix: &Fix) -> Result<()> {
        let start = fix.location.offset;
        let end = start + fix.location.length;

        // Validate bounds
        if start > content.len() || end > content.len() {
            return Err(FshLintError::autofix_error(format!(
                "Fix {} has invalid bounds",
                fix.id
            )));
        }

        // Apply the replacement
        content.replace_range(start..end, &fix.replacement);

        Ok(())
    }

    /// Validate syntax of modified content
    fn validate_syntax(&self, content: &str, file: &PathBuf) -> Result<()> {
        // Basic syntax validation for FSH files
        if file.extension().and_then(|s| s.to_str()) == Some("fsh") {
            self.validate_fsh_syntax(content)?;
        }
        Ok(())
    }

    /// Validate FSH syntax
    pub fn validate_fsh_syntax(&self, content: &str) -> Result<()> {
        // Basic FSH syntax checks
        let lines: Vec<&str> = content.lines().collect();
        let mut brace_count = 0;
        let mut paren_count = 0;
        let mut bracket_count = 0;

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Count brackets for balance checking
            for ch in line.chars() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    '(' => paren_count += 1,
                    ')' => paren_count -= 1,
                    '[' => bracket_count += 1,
                    ']' => bracket_count -= 1,
                    _ => {}
                }

                // Check for negative counts (closing before opening)
                if brace_count < 0 || paren_count < 0 || bracket_count < 0 {
                    return Err(FshLintError::autofix_error(format!(
                        "Unmatched closing bracket at line {}",
                        line_num + 1
                    )));
                }
            }
        }

        // Check for unmatched opening brackets
        if brace_count != 0 || paren_count != 0 || bracket_count != 0 {
            return Err(FshLintError::autofix_error(
                "Unmatched brackets in modified content".to_string(),
            ));
        }

        Ok(())
    }

    /// Detect complex conflicts between fixes
    pub fn detect_complex_conflicts(&self, fixes: &[Fix]) -> Vec<ConflictGroup> {
        let mut conflicts = Vec::new();
        let mut processed = std::collections::HashSet::new();

        for (i, fix1) in fixes.iter().enumerate() {
            if processed.contains(&i) {
                continue;
            }

            let mut conflict_group = vec![i];

            for (j, fix2) in fixes.iter().enumerate().skip(i + 1) {
                if processed.contains(&j) {
                    continue;
                }

                if self.fixes_have_complex_conflict(fix1, fix2) {
                    conflict_group.push(j);
                    processed.insert(j);
                }
            }

            if conflict_group.len() > 1 {
                conflicts.push(ConflictGroup {
                    fix_indices: conflict_group,
                    conflict_type: ConflictType::Overlap,
                });
            }

            processed.insert(i);
        }

        conflicts
    }

    /// Check for complex conflicts between two fixes
    fn fixes_have_complex_conflict(&self, fix1: &Fix, fix2: &Fix) -> bool {
        // Same file check
        if fix1.location.file != fix2.location.file {
            return false;
        }

        // Direct overlap
        if fix1.conflicts_with(fix2) {
            return true;
        }

        // Semantic conflicts (e.g., both trying to modify the same logical construct)
        if fix1.rule_id == fix2.rule_id
            && (fix1.location.line == fix2.location.line
                || (fix1.location.line as i32 - fix2.location.line as i32).abs() <= 2)
        {
            return true;
        }

        false
    }

    /// Generate fix from suggestion with enhanced validation
    fn generate_fix_from_suggestion(
        &self,
        suggestion: &CodeSuggestion,
        diagnostic: &Diagnostic,
        fix_id: String,
    ) -> Result<Fix> {
        // Validate suggestion before creating fix
        if suggestion.replacement.len() > 1000 {
            return Err(FshLintError::autofix_error(
                "Fix replacement text too large".to_string(),
            ));
        }

        // Check for potentially dangerous replacements
        if self.is_dangerous_replacement(&suggestion.replacement) {
            return Err(FshLintError::autofix_error(
                "Fix contains potentially dangerous replacement".to_string(),
            ));
        }

        Ok(Fix::new(
            fix_id,
            suggestion.message.clone(),
            suggestion.location.clone(),
            suggestion.replacement.clone(),
            suggestion.applicability,
            diagnostic.rule_id.clone(),
        ))
    }

    /// Check if replacement follows safe patterns
    pub fn is_safe_replacement_pattern(&self, replacement: &str) -> bool {
        // Whitespace-only changes
        if replacement.trim().is_empty() {
            return true;
        }

        // Simple punctuation additions
        if replacement.len() <= 3 && replacement.chars().all(|c| ".,;:()[]{}".contains(c)) {
            return true;
        }

        // Simple keyword fixes
        let safe_keywords = ["true", "false", "null", "undefined"];
        if safe_keywords.contains(&replacement.trim()) {
            return true;
        }

        false
    }

    /// Check for dangerous replacement patterns
    pub fn is_dangerous_replacement(&self, replacement: &str) -> bool {
        let dangerous_patterns = [
            "eval(",
            "exec(",
            "system(",
            "shell(",
            "import os",
            "import subprocess",
            "__import__",
            "file://",
            "http://",
            "https://",
        ];

        let lower_replacement = replacement.to_lowercase();
        dangerous_patterns
            .iter()
            .any(|pattern| lower_replacement.contains(pattern))
    }
}

impl RollbackPlan {
    /// Execute the rollback plan
    pub fn execute(&self) -> Result<()> {
        use std::fs;

        for (file, content) in &self.original_contents {
            fs::write(file, content).map_err(|e| FshLintError::io_error(file, e))?;
        }

        Ok(())
    }

    /// Execute rollback for specific files only
    pub fn execute_partial(&self, files: &[PathBuf]) -> Result<()> {
        use std::fs;

        for file in files {
            if let Some(content) = self.original_contents.get(file) {
                fs::write(file, content).map_err(|e| FshLintError::io_error(file, e))?;
            }
        }

        Ok(())
    }

    /// Get the age of this rollback plan
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed().unwrap_or_default()
    }

    /// Check if rollback plan is still valid (files haven't been modified since)
    pub fn is_valid(&self) -> bool {
        use std::fs;

        for file in self.original_contents.keys() {
            if let Ok(metadata) = fs::metadata(file) {
                if let Ok(modified) = metadata.modified() {
                    if modified > self.created_at {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Save rollback plan to disk
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        use std::fs;

        let serialized = serde_json::to_string_pretty(self).map_err(|e| {
            FshLintError::autofix_error(format!("Failed to serialize rollback plan: {e}"))
        })?;

        fs::write(path, serialized).map_err(|e| FshLintError::io_error(path, e))?;

        Ok(())
    }

    /// Load rollback plan from disk
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        use std::fs;

        let content = fs::read_to_string(path).map_err(|e| FshLintError::io_error(path, e))?;

        let plan: RollbackPlan = serde_json::from_str(&content).map_err(|e| {
            FshLintError::autofix_error(format!("Failed to deserialize rollback plan: {e}"))
        })?;

        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Severity;
    use std::path::PathBuf;

    fn create_test_diagnostic() -> Diagnostic {
        let location = Location::new(PathBuf::from("test.fsh"), 1, 1, 0, 5);

        let suggestion = CodeSuggestion::safe(
            "Replace with correct syntax",
            "fixed_text",
            location.clone(),
        );

        Diagnostic::new("test-rule", Severity::Error, "Test error", location)
            .with_suggestion(suggestion)
    }

    #[test]
    fn test_fix_creation() {
        let location = Location::new(PathBuf::from("test.fsh"), 1, 1, 0, 5);
        let fix = Fix::new(
            "test-fix".to_string(),
            "Test fix".to_string(),
            location,
            "replacement".to_string(),
            Applicability::Always,
            "test-rule".to_string(),
        );

        assert_eq!(fix.id, "test-fix");
        assert!(fix.is_safe());
        assert_eq!(fix.span(), (0, 5));
    }

    #[test]
    fn test_fix_conflicts() {
        let location1 = Location::new(PathBuf::from("test.fsh"), 1, 1, 0, 5);
        let location2 = Location::new(PathBuf::from("test.fsh"), 1, 3, 2, 3);
        let location3 = Location::new(PathBuf::from("test.fsh"), 1, 10, 10, 5);

        let fix1 = Fix::new(
            "fix1".to_string(),
            "Fix 1".to_string(),
            location1,
            "text1".to_string(),
            Applicability::Always,
            "rule1".to_string(),
        );
        let fix2 = Fix::new(
            "fix2".to_string(),
            "Fix 2".to_string(),
            location2,
            "text2".to_string(),
            Applicability::Always,
            "rule2".to_string(),
        );
        let fix3 = Fix::new(
            "fix3".to_string(),
            "Fix 3".to_string(),
            location3,
            "text3".to_string(),
            Applicability::Always,
            "rule3".to_string(),
        );

        assert!(fix1.conflicts_with(&fix2)); // Overlapping ranges
        assert!(!fix1.conflicts_with(&fix3)); // Non-overlapping ranges
    }

    #[test]
    fn test_generate_fixes_from_diagnostics() {
        let engine = DefaultAutofixEngine::new();
        let diagnostic = create_test_diagnostic();
        let fixes = engine.generate_fixes(&[diagnostic]).unwrap();

        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].rule_id, "test-rule");
        assert!(fixes[0].is_safe());
    }

    #[test]
    fn test_conflict_resolution() {
        let engine = DefaultAutofixEngine::new();

        let location1 = Location::new(PathBuf::from("test.fsh"), 1, 1, 0, 5);
        let location2 = Location::new(PathBuf::from("test.fsh"), 1, 3, 2, 3);

        let fix1 = Fix::new(
            "fix1".to_string(),
            "Fix 1".to_string(),
            location1,
            "text1".to_string(),
            Applicability::Always,
            "rule1".to_string(),
        )
        .with_priority(1);
        let fix2 = Fix::new(
            "fix2".to_string(),
            "Fix 2".to_string(),
            location2,
            "text2".to_string(),
            Applicability::Always,
            "rule2".to_string(),
        )
        .with_priority(2);

        let resolved = engine.resolve_conflicts(&[fix1, fix2]);

        // Should keep the higher priority fix
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].id, "fix2");
    }

    #[test]
    fn test_fix_config() {
        let config = FixConfig {
            apply_unsafe: true,
            dry_run: true,
            max_fixes_per_file: Some(10),
            validate_syntax: false,
            ..FixConfig::default()
        };

        assert!(config.apply_unsafe);
        assert!(config.dry_run);
        assert_eq!(config.max_fixes_per_file, Some(10));
        assert!(!config.validate_syntax);
    }
}
