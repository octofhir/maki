//! Optimization statistics tracking
//!
//! Tracks the changes made during optimization for reporting and debugging.

use std::fmt;

/// Statistics about optimization changes
///
/// Tracks how many rules were removed, modified, or added during optimization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptimizationStats {
    /// Number of rules removed
    pub rules_removed: usize,

    /// Number of rules modified
    pub rules_modified: usize,

    /// Number of rules added
    pub rules_added: usize,

    /// Number of rules that were redundant (removed because already implied)
    pub redundant_rules: usize,

    /// Number of rules that were simplified (e.g., cardinality changes)
    pub simplified_rules: usize,
}

impl OptimizationStats {
    /// Create a new empty statistics object
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes were made
    pub fn has_changes(&self) -> bool {
        self.rules_removed > 0
            || self.rules_modified > 0
            || self.rules_added > 0
            || self.redundant_rules > 0
            || self.simplified_rules > 0
    }

    /// Get total number of changes
    pub fn total_changes(&self) -> usize {
        self.rules_removed
            + self.rules_modified
            + self.rules_added
            + self.redundant_rules
            + self.simplified_rules
    }

    /// Merge statistics from another optimization pass
    pub fn merge(&mut self, other: &OptimizationStats) {
        self.rules_removed += other.rules_removed;
        self.rules_modified += other.rules_modified;
        self.rules_added += other.rules_added;
        self.redundant_rules += other.redundant_rules;
        self.simplified_rules += other.simplified_rules;
    }

    /// Record a rule removal
    pub fn record_removal(&mut self) {
        self.rules_removed += 1;
    }

    /// Record a rule modification
    pub fn record_modification(&mut self) {
        self.rules_modified += 1;
    }

    /// Record a rule addition
    pub fn record_addition(&mut self) {
        self.rules_added += 1;
    }

    /// Record a redundant rule removal
    pub fn record_redundant(&mut self) {
        self.redundant_rules += 1;
    }

    /// Record a rule simplification
    pub fn record_simplification(&mut self) {
        self.simplified_rules += 1;
    }
}

impl fmt::Display for OptimizationStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.has_changes() {
            return write!(f, "No changes");
        }

        let mut parts = Vec::new();

        if self.rules_removed > 0 {
            parts.push(format!("{} removed", self.rules_removed));
        }
        if self.rules_modified > 0 {
            parts.push(format!("{} modified", self.rules_modified));
        }
        if self.rules_added > 0 {
            parts.push(format!("{} added", self.rules_added));
        }
        if self.redundant_rules > 0 {
            parts.push(format!("{} redundant", self.redundant_rules));
        }
        if self.simplified_rules > 0 {
            parts.push(format!("{} simplified", self.simplified_rules));
        }

        write!(f, "{}", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stats() {
        let stats = OptimizationStats::new();
        assert!(!stats.has_changes());
        assert_eq!(stats.total_changes(), 0);
    }

    #[test]
    fn test_record_changes() {
        let mut stats = OptimizationStats::new();

        stats.record_removal();
        assert_eq!(stats.rules_removed, 1);
        assert!(stats.has_changes());

        stats.record_modification();
        assert_eq!(stats.rules_modified, 1);

        stats.record_addition();
        assert_eq!(stats.rules_added, 1);

        stats.record_redundant();
        assert_eq!(stats.redundant_rules, 1);

        stats.record_simplification();
        assert_eq!(stats.simplified_rules, 1);

        assert_eq!(stats.total_changes(), 5);
    }

    #[test]
    fn test_merge_stats() {
        let mut stats1 = OptimizationStats::new();
        stats1.record_removal();
        stats1.record_modification();

        let mut stats2 = OptimizationStats::new();
        stats2.record_addition();
        stats2.record_redundant();

        stats1.merge(&stats2);

        assert_eq!(stats1.rules_removed, 1);
        assert_eq!(stats1.rules_modified, 1);
        assert_eq!(stats1.rules_added, 1);
        assert_eq!(stats1.redundant_rules, 1);
        assert_eq!(stats1.total_changes(), 4);
    }

    #[test]
    fn test_display_no_changes() {
        let stats = OptimizationStats::new();
        assert_eq!(stats.to_string(), "No changes");
    }

    #[test]
    fn test_display_with_changes() {
        let mut stats = OptimizationStats::new();
        stats.record_removal();
        stats.record_removal();
        stats.record_modification();

        let display = stats.to_string();
        assert!(display.contains("2 removed"));
        assert!(display.contains("1 modified"));
    }

    #[test]
    fn test_display_all_types() {
        let mut stats = OptimizationStats::new();
        stats.record_removal();
        stats.record_modification();
        stats.record_addition();
        stats.record_redundant();
        stats.record_simplification();

        let display = stats.to_string();
        assert!(display.contains("removed"));
        assert!(display.contains("modified"));
        assert!(display.contains("added"));
        assert!(display.contains("redundant"));
        assert!(display.contains("simplified"));
    }
}
