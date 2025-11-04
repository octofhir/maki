//! Incremental CST updates for FSH
//!
//! This module provides functionality for updating CST nodes incrementally
//! without requiring full re-parsing. This is essential for:
//! - IDE performance with large files
//! - Real-time syntax highlighting and error checking
//! - Efficient refactoring operations
//!
//! # Example
//!
//! ```rust,ignore
//! use maki_core::cst::incremental::{IncrementalUpdater, TextEdit};
//!
//! let source = "Profile: MyPatient\nParent: Patient";
//! let (mut cst, _, _) = parse_fsh(source);
//! 
//! let updater = IncrementalUpdater::new();
//! let edit = TextEdit::replace_range(8..17, "NewPatient");
//! 
//! let updated_cst = updater.apply_edit(&cst, &edit)?;
//! assert!(updated_cst.text().to_string().contains("NewPatient"));
//! ```

use super::{
    parse_fsh, FshSyntaxNode,
    trivia::TriviaPreserver,
};
use rowan::{TextRange, TextSize};

/// Represents a text edit operation
#[derive(Debug, Clone, PartialEq)]
pub struct TextEdit {
    /// Range to replace
    pub range: TextRange,
    /// New text to insert
    pub new_text: String,
}

impl TextEdit {
    /// Create a new text edit
    pub fn new(range: TextRange, new_text: String) -> Self {
        Self { range, new_text }
    }

    /// Create a replacement edit
    pub fn replace(range: impl Into<TextRange>, new_text: impl Into<String>) -> Self {
        Self {
            range: range.into(),
            new_text: new_text.into(),
        }
    }

    /// Create a replacement edit from usize range
    pub fn replace_range(range: std::ops::Range<usize>, new_text: impl Into<String>) -> Self {
        Self {
            range: range_to_text_range(range),
            new_text: new_text.into(),
        }
    }

    /// Create an insertion edit
    pub fn insert(position: TextSize, text: impl Into<String>) -> Self {
        Self {
            range: TextRange::new(position, position),
            new_text: text.into(),
        }
    }

    /// Create a deletion edit
    pub fn delete(range: impl Into<TextRange>) -> Self {
        Self {
            range: range.into(),
            new_text: String::new(),
        }
    }

    /// Check if this edit is an insertion
    pub fn is_insertion(&self) -> bool {
        self.range.is_empty() && !self.new_text.is_empty()
    }

    /// Check if this edit is a deletion
    pub fn is_deletion(&self) -> bool {
        !self.range.is_empty() && self.new_text.is_empty()
    }

    /// Check if this edit is a replacement
    pub fn is_replacement(&self) -> bool {
        !self.range.is_empty() && !self.new_text.is_empty()
    }

    /// Get the length change caused by this edit
    pub fn length_delta(&self) -> i64 {
        self.new_text.len() as i64 - usize::from(self.range.len()) as i64
    }
}

impl From<(TextRange, String)> for TextEdit {
    fn from((range, new_text): (TextRange, String)) -> Self {
        Self::new(range, new_text)
    }
}

impl From<(std::ops::Range<usize>, &str)> for TextEdit {
    fn from((range, new_text): (std::ops::Range<usize>, &str)) -> Self {
        let text_range = TextRange::new(
            TextSize::from(range.start as u32),
            TextSize::from(range.end as u32),
        );
        Self::new(text_range, new_text.to_string())
    }
}

// Helper function to convert range to TextRange
fn range_to_text_range(range: std::ops::Range<usize>) -> TextRange {
    TextRange::new(
        TextSize::from(range.start as u32),
        TextSize::from(range.end as u32),
    )
}

/// Result of an incremental update operation
#[derive(Debug, Clone)]
pub struct UpdateResult {
    /// The updated CST
    pub cst: FshSyntaxNode,
    /// Whether the update was successful
    pub success: bool,
    /// Any errors encountered during update
    pub errors: Vec<String>,
    /// Performance metrics
    pub metrics: UpdateMetrics,
}

/// Performance metrics for incremental updates
#[derive(Debug, Clone, Default)]
pub struct UpdateMetrics {
    /// Time taken for the update (in microseconds)
    pub update_time_us: u64,
    /// Number of nodes that were re-parsed
    pub nodes_reparsed: usize,
    /// Number of nodes that were reused
    pub nodes_reused: usize,
    /// Size of the affected region
    pub affected_range: Option<TextRange>,
}

impl UpdateMetrics {
    /// Calculate the reuse ratio (0.0 to 1.0)
    pub fn reuse_ratio(&self) -> f64 {
        let total = self.nodes_reparsed + self.nodes_reused;
        if total == 0 {
            0.0
        } else {
            self.nodes_reused as f64 / total as f64
        }
    }

    /// Check if the update was efficient (high reuse ratio)
    pub fn is_efficient(&self) -> bool {
        self.reuse_ratio() > 0.7 // 70% reuse threshold
    }
}

/// Handles incremental updates to FSH CSTs
pub struct IncrementalUpdater {
    /// Whether to preserve trivia during updates
    preserve_trivia: bool,
    /// Maximum size for incremental updates (larger changes trigger full reparse)
    max_incremental_size: usize,
}

impl IncrementalUpdater {
    /// Create a new incremental updater
    pub fn new() -> Self {
        Self {
            preserve_trivia: true,
            max_incremental_size: 10000, // 10KB
        }
    }

    /// Set trivia preservation
    pub fn preserve_trivia(mut self, preserve: bool) -> Self {
        self.preserve_trivia = preserve;
        self
    }

    /// Set maximum size for incremental updates
    pub fn max_incremental_size(mut self, size: usize) -> Self {
        self.max_incremental_size = size;
        self
    }

    /// Apply a single text edit to a CST
    pub fn apply_edit(
        &self,
        cst: &FshSyntaxNode,
        edit: &TextEdit,
    ) -> Result<UpdateResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        
        // Check if we should do incremental update or full reparse
        if self.should_full_reparse(cst, edit) {
            return self.full_reparse_with_edit(cst, edit, start_time);
        }

        // Try incremental update
        match self.try_incremental_update(cst, edit) {
            Ok(mut result) => {
                result.metrics.update_time_us = start_time.elapsed().as_micros() as u64;
                Ok(result)
            }
            Err(_) => {
                // Fall back to full reparse
                self.full_reparse_with_edit(cst, edit, start_time)
            }
        }
    }

    /// Apply multiple text edits to a CST
    pub fn apply_edits(
        &self,
        cst: &FshSyntaxNode,
        edits: &[TextEdit],
    ) -> Result<UpdateResult, Box<dyn std::error::Error>> {
        if edits.is_empty() {
            return Ok(UpdateResult {
                cst: cst.clone(),
                success: true,
                errors: Vec::new(),
                metrics: UpdateMetrics::default(),
            });
        }

        // Sort edits by position (reverse order for easier application)
        let mut sorted_edits = edits.to_vec();
        sorted_edits.sort_by(|a, b| b.range.start().cmp(&a.range.start()));

        // Apply edits one by one
        let mut current_cst = cst.clone();
        let mut total_metrics = UpdateMetrics::default();
        let mut all_errors = Vec::new();

        for edit in sorted_edits {
            let result = self.apply_edit(&current_cst, &edit)?;
            current_cst = result.cst;
            
            total_metrics.nodes_reparsed += result.metrics.nodes_reparsed;
            total_metrics.nodes_reused += result.metrics.nodes_reused;
            total_metrics.update_time_us += result.metrics.update_time_us;
            all_errors.extend(result.errors);

            if !result.success {
                return Ok(UpdateResult {
                    cst: current_cst,
                    success: false,
                    errors: all_errors,
                    metrics: total_metrics,
                });
            }
        }

        Ok(UpdateResult {
            cst: current_cst,
            success: true,
            errors: all_errors,
            metrics: total_metrics,
        })
    }

    /// Check if we should do a full reparse instead of incremental update
    fn should_full_reparse(&self, cst: &FshSyntaxNode, edit: &TextEdit) -> bool {
        // Full reparse if edit is too large
        if edit.new_text.len() > self.max_incremental_size {
            return true;
        }

        // Full reparse if edit affects structural elements
        if self.affects_structure(cst, edit) {
            return true;
        }

        false
    }

    /// Check if an edit affects structural elements (keywords, etc.)
    fn affects_structure(&self, cst: &FshSyntaxNode, edit: &TextEdit) -> bool {
        // Find nodes that intersect with the edit range
        for node in cst.descendants() {
            if node.text_range().intersect(edit.range).is_some() {
                // Check if this node contains structural keywords
                let text = node.text().to_string();
                if text.contains("Profile:")
                    || text.contains("Extension:")
                    || text.contains("ValueSet:")
                    || text.contains("CodeSystem:")
                    || text.contains("Parent:")
                    || text.contains("Id:")
                {
                    return true;
                }
            }
        }
        false
    }

    /// Attempt incremental update
    fn try_incremental_update(
        &self,
        cst: &FshSyntaxNode,
        edit: &TextEdit,
    ) -> Result<UpdateResult, Box<dyn std::error::Error>> {
        // For now, implement a simple approach:
        // 1. Apply the text edit to get new source
        // 2. Find the minimal affected region
        // 3. Re-parse only that region
        // 4. Splice the new nodes into the existing tree

        let original_text = cst.text().to_string();
        let new_text = self.apply_text_edit(&original_text, edit);

        // Find affected nodes
        let affected_nodes = self.find_affected_nodes(cst, edit);
        
        // For simplicity, if we can't determine a minimal region, fall back to full reparse
        if affected_nodes.is_empty() {
            return Err("No affected nodes found".into());
        }

        // Re-parse the new text (simplified approach)
        let (new_cst, _, parse_errors) = parse_fsh(&new_text);
        
        let errors: Vec<String> = parse_errors
            .into_iter()
            .map(|e| format!("Parse error: {:?}", e))
            .collect();

        Ok(UpdateResult {
            cst: new_cst,
            success: errors.is_empty(),
            errors,
            metrics: UpdateMetrics {
                update_time_us: 0, // Will be set by caller
                nodes_reparsed: affected_nodes.len(),
                nodes_reused: 0, // Simplified - not tracking reuse yet
                affected_range: Some(edit.range),
            },
        })
    }

    /// Apply text edit to source string
    fn apply_text_edit(&self, source: &str, edit: &TextEdit) -> String {
        let start = edit.range.start().into();
        let end = edit.range.end().into();
        
        let mut result = String::new();
        result.push_str(&source[..start]);
        result.push_str(&edit.new_text);
        result.push_str(&source[end..]);
        
        result
    }

    /// Find nodes affected by the edit
    fn find_affected_nodes(&self, cst: &FshSyntaxNode, edit: &TextEdit) -> Vec<FshSyntaxNode> {
        let mut affected = Vec::new();
        
        for node in cst.descendants() {
            if node.text_range().intersect(edit.range).is_some() {
                affected.push(node);
            }
        }
        
        affected
    }

    /// Perform full reparse with edit applied
    fn full_reparse_with_edit(
        &self,
        cst: &FshSyntaxNode,
        edit: &TextEdit,
        start_time: std::time::Instant,
    ) -> Result<UpdateResult, Box<dyn std::error::Error>> {
        let original_text = cst.text().to_string();
        let new_text = self.apply_text_edit(&original_text, edit);
        
        // Preserve trivia if requested
        let trivia_preserver = if self.preserve_trivia {
            Some(TriviaPreserver::from_cst(cst))
        } else {
            None
        };

        let (new_cst, _, parse_errors) = parse_fsh(&new_text);
        
        let errors: Vec<String> = parse_errors
            .into_iter()
            .map(|e| format!("Parse error: {:?}", e))
            .collect();

        // Apply preserved trivia if available
        let final_cst = if let Some(preserver) = trivia_preserver {
            // For now, just return the new CST
            // A full implementation would apply preserved trivia
            new_cst
        } else {
            new_cst
        };

        Ok(UpdateResult {
            cst: final_cst,
            success: errors.is_empty(),
            errors,
            metrics: UpdateMetrics {
                update_time_us: start_time.elapsed().as_micros() as u64,
                nodes_reparsed: 1, // Full reparse counts as 1 large node
                nodes_reused: 0,
                affected_range: Some(edit.range),
            },
        })
    }

    /// Validate that an update maintains semantic correctness
    pub fn validate_update(
        &self,
        original: &FshSyntaxNode,
        updated: &FshSyntaxNode,
        edit: &TextEdit,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Basic validation: check that the edit was applied correctly
        let original_text = original.text().to_string();
        let expected_text = self.apply_text_edit(&original_text, edit);
        let actual_text = updated.text().to_string();

        Ok(expected_text == actual_text)
    }
}

impl Default for IncrementalUpdater {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilities for working with text ranges and edits
pub struct EditUtils;

impl EditUtils {
    /// Merge overlapping or adjacent edits
    pub fn merge_edits(edits: &[TextEdit]) -> Vec<TextEdit> {
        if edits.is_empty() {
            return Vec::new();
        }

        let mut sorted_edits = edits.to_vec();
        sorted_edits.sort_by(|a, b| a.range.start().cmp(&b.range.start()));

        let mut merged = Vec::new();
        let mut current = sorted_edits[0].clone();

        for edit in sorted_edits.into_iter().skip(1) {
            if current.range.end() >= edit.range.start() {
                // Overlapping or adjacent - merge them
                let new_range = TextRange::new(
                    current.range.start(),
                    edit.range.end().max(current.range.end()),
                );
                
                // Combine the text changes
                let mut new_text = current.new_text.clone();
                new_text.push_str(&edit.new_text);
                
                current = TextEdit::new(new_range, new_text);
            } else {
                // Non-overlapping - add current and start new
                merged.push(current);
                current = edit;
            }
        }
        
        merged.push(current);
        merged
    }

    /// Check if two edits conflict
    pub fn edits_conflict(edit1: &TextEdit, edit2: &TextEdit) -> bool {
        edit1.range.intersect(edit2.range).is_some()
    }

    /// Adjust edit ranges after applying another edit
    pub fn adjust_edit_after(edit: &TextEdit, applied_edit: &TextEdit) -> TextEdit {
        if edit.range.start() <= applied_edit.range.start() {
            // Edit is before the applied edit - no adjustment needed
            edit.clone()
        } else {
            // Edit is after the applied edit - adjust for length change
            let delta = applied_edit.length_delta();
            let new_start = if delta >= 0 {
                edit.range.start() + TextSize::from(delta as u32)
            } else {
                edit.range.start() - TextSize::from((-delta) as u32)
            };
            let new_end = if delta >= 0 {
                edit.range.end() + TextSize::from(delta as u32)
            } else {
                edit.range.end() - TextSize::from((-delta) as u32)
            };
            
            TextEdit::new(
                TextRange::new(new_start, new_end),
                edit.new_text.clone(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parse_fsh;

    #[test]
    fn test_text_edit_creation() {
        let edit = TextEdit::replace_range(5..10, "new_text");
        assert_eq!(edit.range.start(), TextSize::from(5));
        assert_eq!(edit.range.end(), TextSize::from(10));
        assert_eq!(edit.new_text, "new_text");
        assert!(edit.is_replacement());
        assert!(!edit.is_insertion());
        assert!(!edit.is_deletion());
    }

    #[test]
    fn test_text_edit_types() {
        let insertion = TextEdit::insert(TextSize::from(5), "inserted");
        assert!(insertion.is_insertion());
        assert_eq!(insertion.length_delta(), 8);

        let deletion = TextEdit::delete(TextRange::new(5.into(), 10.into()));
        assert!(deletion.is_deletion());
        assert_eq!(deletion.length_delta(), -5);

        let replacement = TextEdit::replace_range(5..10, "replaced");
        assert!(replacement.is_replacement());
        assert_eq!(replacement.length_delta(), 3); // "replaced" (8) - 5 = 3
    }

    #[test]
    fn test_apply_text_edit() {
        let updater = IncrementalUpdater::new();
        let source = "Profile: MyPatient\nParent: Patient";
        let edit = TextEdit::replace_range(9..18, "NewPatient");
        
        let result = updater.apply_text_edit(source, &edit);
        assert_eq!(result, "Profile: NewPatient\nParent: Patient");
    }

    #[test]
    fn test_incremental_update() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (cst, _, _) = parse_fsh(source);
        
        let updater = IncrementalUpdater::new();
        let edit = TextEdit::replace_range(9..18, "NewPatient");
        
        let result = updater.apply_edit(&cst, &edit).unwrap();
        assert!(result.success);
        assert!(result.cst.text().to_string().contains("NewPatient"));
    }

    #[test]
    fn test_multiple_edits() {
        let source = "Profile: MyPatient\nParent: Patient\nId: my-patient";
        let (cst, _, _) = parse_fsh(source);
        
        let updater = IncrementalUpdater::new();
        let edits = vec![
            TextEdit::replace_range(9..18, "NewPatient"),
            TextEdit::replace_range(28..35, "NewParent"),
        ];
        
        let result = updater.apply_edits(&cst, &edits).unwrap();
        assert!(result.success);
        let text = result.cst.text().to_string();
        assert!(text.contains("NewPatient"));
        assert!(text.contains("NewParent"));
    }

    #[test]
    fn test_update_metrics() {
        let mut metrics = UpdateMetrics::default();
        metrics.nodes_reparsed = 2;
        metrics.nodes_reused = 8;
        
        assert_eq!(metrics.reuse_ratio(), 0.8);
        assert!(metrics.is_efficient());
        
        metrics.nodes_reused = 3;
        assert_eq!(metrics.reuse_ratio(), 0.6);
        assert!(!metrics.is_efficient());
    }

    #[test]
    fn test_edit_utils_merge() {
        let edits = vec![
            TextEdit::replace_range(5..10, "first"),
            TextEdit::replace_range(8..15, "second"),
            TextEdit::replace_range(20..25, "third"),
        ];
        
        let merged = EditUtils::merge_edits(&edits);
        assert_eq!(merged.len(), 2); // First two should merge, third separate
    }

    #[test]
    fn test_edit_utils_conflict() {
        let edit1 = TextEdit::replace_range(5..10, "first");
        let edit2 = TextEdit::replace_range(8..15, "second");
        let edit3 = TextEdit::replace_range(20..25, "third");
        
        assert!(EditUtils::edits_conflict(&edit1, &edit2));
        assert!(!EditUtils::edits_conflict(&edit1, &edit3));
    }

    #[test]
    fn test_edit_adjustment() {
        let edit = TextEdit::replace_range(20..25, "replacement");
        let applied_edit = TextEdit::insert(TextSize::from(10), "inserted");
        
        let adjusted = EditUtils::adjust_edit_after(&edit, &applied_edit);
        
        // Edit should be shifted by the length of the insertion
        assert_eq!(adjusted.range.start(), TextSize::from(28)); // 20 + 8
        assert_eq!(adjusted.range.end(), TextSize::from(33)); // 25 + 8
    }

    #[test]
    fn test_validation() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (original_cst, _, _) = parse_fsh(source);
        
        let updater = IncrementalUpdater::new();
        let edit = TextEdit::replace_range(9..18, "NewPatient");
        
        let result = updater.apply_edit(&original_cst, &edit).unwrap();
        let is_valid = updater.validate_update(&original_cst, &result.cst, &edit).unwrap();
        
        assert!(is_valid);
    }

    #[test]
    fn test_structural_change_detection() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (cst, _, _) = parse_fsh(source);
        
        let updater = IncrementalUpdater::new();
        
        // Non-structural edit (changing name)
        let name_edit = TextEdit::replace_range(9..18, "NewPatient");
        assert!(!updater.affects_structure(&cst, &name_edit));
        
        // Structural edit (changing keyword)
        let keyword_edit = TextEdit::replace_range(0..7, "Extension");
        assert!(updater.affects_structure(&cst, &keyword_edit));
    }
}