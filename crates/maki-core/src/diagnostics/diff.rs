//! Diff rendering for diagnostic suggestions

use crate::console::{Color, Console};
use similar::{ChangeTag, TextDiff};

/// Renderer for text diffs in diagnostic suggestions
pub struct DiffRenderer {
    console: Console,
}

impl DiffRenderer {
    /// Create a new diff renderer
    pub fn new() -> Self {
        Self {
            console: Console::new(),
        }
    }

    /// Create a diff renderer with colors disabled
    pub fn no_colors() -> Self {
        Self {
            console: Console::no_colors(),
        }
    }

    /// Render a unified diff between original and modified text
    ///
    /// Returns a string with colored diff output showing additions (+) and deletions (-)
    pub fn render_diff(&self, original: &str, modified: &str) -> String {
        let diff = TextDiff::from_lines(original, modified);
        let mut output = String::new();

        for (idx, change) in diff.iter_all_changes().enumerate() {
            let line_num = idx + 1;

            match change.tag() {
                ChangeTag::Delete => {
                    output.push_str(&self.console.colorize("- ", Color::Red));
                    output.push_str(&format!("{line_num:>4} │ "));
                    output.push_str(&self.console.colorize(change.value(), Color::Red));
                }
                ChangeTag::Insert => {
                    output.push_str(&self.console.colorize("+ ", Color::Green));
                    output.push_str(&format!("{line_num:>4} │ "));
                    output.push_str(&self.console.colorize(change.value(), Color::Green));
                }
                ChangeTag::Equal => {
                    output.push_str("  ");
                    output.push_str(&format!("{line_num:>4} │ "));
                    output.push_str(change.value());
                }
            }

            // Add newline if the change doesn't already end with one
            if !change.value().ends_with('\n') {
                output.push('\n');
            }
        }

        output
    }

    /// Render an inline diff showing changes within a single line
    ///
    /// Highlights specific character changes rather than whole lines
    pub fn render_inline_diff(&self, original: &str, modified: &str) -> String {
        let diff = TextDiff::from_chars(original, modified);
        let mut output = String::new();

        // Show original with deletions highlighted
        output.push_str(&self.console.colorize("- ", Color::Red));
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    output.push_str(&self.console.colorize(change.value(), Color::Red));
                }
                ChangeTag::Equal => {
                    output.push_str(change.value());
                }
                _ => {}
            }
        }
        output.push('\n');

        // Show modified with additions highlighted
        output.push_str(&self.console.colorize("+ ", Color::Green));
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Insert => {
                    output.push_str(&self.console.colorize(change.value(), Color::Green));
                }
                ChangeTag::Equal => {
                    output.push_str(change.value());
                }
                _ => {}
            }
        }
        output.push('\n');

        output
    }

    /// Render a compact diff suitable for suggestions
    ///
    /// Shows only the changed portions with minimal context
    pub fn render_suggestion_diff(
        &self,
        original: &str,
        modified: &str,
        context_lines: usize,
    ) -> String {
        let diff = TextDiff::from_lines(original, modified);
        let mut output = String::new();

        let changes: Vec<_> = diff.iter_all_changes().collect();
        let mut i = 0;

        while i < changes.len() {
            let change = changes[i];

            match change.tag() {
                ChangeTag::Delete | ChangeTag::Insert => {
                    // Show context before
                    let context_start = i.saturating_sub(context_lines);
                    #[allow(clippy::needless_range_loop)]
                    for j in context_start..i {
                        output.push_str("  ");
                        output.push_str(
                            &self
                                .console
                                .colorize(&format!("{:>4} │ ", j + 1), Color::Dim),
                        );
                        output.push_str(changes[j].value());
                        if !changes[j].value().ends_with('\n') {
                            output.push('\n');
                        }
                    }

                    // Show the change
                    let (marker, color) = match change.tag() {
                        ChangeTag::Delete => ("- ", Color::Red),
                        ChangeTag::Insert => ("+ ", Color::Green),
                        _ => unreachable!(),
                    };

                    output.push_str(&self.console.colorize(marker, color));
                    output.push_str(&format!("{:>4} │ ", i + 1));
                    output.push_str(&self.console.colorize(change.value(), color));
                    if !change.value().ends_with('\n') {
                        output.push('\n');
                    }

                    // Show context after
                    let context_end = (i + context_lines + 1).min(changes.len());
                    #[allow(clippy::needless_range_loop)]
                    for j in (i + 1)..context_end {
                        if changes[j].tag() == ChangeTag::Equal {
                            output.push_str("  ");
                            output.push_str(
                                &self
                                    .console
                                    .colorize(&format!("{:>4} │ ", j + 1), Color::Dim),
                            );
                            output.push_str(changes[j].value());
                            if !changes[j].value().ends_with('\n') {
                                output.push('\n');
                            }
                        }
                    }
                }
                _ => {}
            }

            i += 1;
        }

        output
    }
}

impl Default for DiffRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_diff() {
        let renderer = DiffRenderer::no_colors();
        let original = "Hello\nWorld\n";
        let modified = "Hello\nRust\n";

        let diff = renderer.render_diff(original, modified);

        assert!(diff.contains("Hello"));
        assert!(diff.contains("- "));
        assert!(diff.contains("World"));
        assert!(diff.contains("+ "));
        assert!(diff.contains("Rust"));
    }

    #[test]
    fn test_inline_diff() {
        let renderer = DiffRenderer::no_colors();
        let original = "Profile: OldName";
        let modified = "Profile: NewName";

        let diff = renderer.render_inline_diff(original, modified);

        assert!(diff.contains("- "));
        assert!(diff.contains("+ "));
        assert!(diff.contains("OldName") || diff.contains("NewName"));
    }

    #[test]
    fn test_empty_diff() {
        let renderer = DiffRenderer::no_colors();
        let original = "Same content";
        let modified = "Same content";

        let diff = renderer.render_diff(original, modified);

        assert!(diff.contains("Same content"));
        assert!(!diff.contains("- "));
        assert!(!diff.contains("+ "));
    }

    #[test]
    fn test_suggestion_diff() {
        let renderer = DiffRenderer::no_colors();
        let original = "Line 1\nLine 2 old\nLine 3\n";
        let modified = "Line 1\nLine 2 new\nLine 3\n";

        let diff = renderer.render_suggestion_diff(original, modified, 1);

        assert!(diff.contains("Line 1"));
        assert!(diff.contains("old"));
        assert!(diff.contains("new"));
        assert!(diff.contains("Line 3"));
    }
}
