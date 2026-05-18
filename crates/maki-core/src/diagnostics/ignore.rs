//! SUSHI-compatible `sushi-ignoreErrors.txt` loader.
//!
//! SUSHI (v3.17.0+) lets IG authors place a `sushi-ignoreErrors.txt` file at
//! the IG input root. Each non-empty / non-comment line is a regex pattern;
//! diagnostics whose `code`, `rule_id`, or rendered message match any pattern
//! are suppressed from output. Comments are lines starting with `#`.
//!
//! This mirrors that behaviour for `maki`. Loading is best-effort — invalid
//! regexes log a warning and are skipped, so a partially broken file does not
//! abort the build.

use crate::diagnostics::Diagnostic;
use regex::Regex;
use std::path::Path;
use tracing::warn;

/// Compiled set of suppression patterns from `sushi-ignoreErrors.txt`.
#[derive(Debug, Default, Clone)]
pub struct IgnoreErrors {
    patterns: Vec<Regex>,
}

impl IgnoreErrors {
    /// Filename SUSHI looks for at the input-directory root.
    pub const FILE_NAME: &'static str = "sushi-ignoreErrors.txt";

    /// Empty rule set (matches nothing).
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to load from `<dir>/sushi-ignoreErrors.txt`. Returns an empty rule
    /// set if the file is absent or unreadable.
    pub fn from_input_dir(dir: &Path) -> Self {
        let path = dir.join(Self::FILE_NAME);
        if !path.is_file() {
            return Self::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::from_text(&text),
            Err(e) => {
                warn!("Failed to read {}: {}", path.display(), e);
                Self::new()
            }
        }
    }

    /// Parse the textual contents directly. One regex per non-empty,
    /// non-`#`-prefixed line. Invalid patterns log a warning and are dropped.
    pub fn from_text(text: &str) -> Self {
        let mut patterns = Vec::new();
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            match Regex::new(line) {
                Ok(rx) => patterns.push(rx),
                Err(e) => warn!(
                    "sushi-ignoreErrors.txt line {}: invalid regex `{}`: {}",
                    idx + 1,
                    line,
                    e
                ),
            }
        }
        Self { patterns }
    }

    /// True if no patterns are loaded.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Number of compiled patterns.
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// True if any pattern matches the given diagnostic. Compares against the
    /// diagnostic's `code`, `rule_id`, and rendered `message` so authors can
    /// suppress by stable identifier (e.g. `^FSH0\d+$`) or by message text.
    pub fn matches(&self, diag: &Diagnostic) -> bool {
        if self.patterns.is_empty() {
            return false;
        }
        let code = diag.code.as_deref().unwrap_or("");
        self.patterns.iter().any(|p| {
            p.is_match(&diag.rule_id) || p.is_match(code) || p.is_match(&diag.message)
        })
    }

    /// True if any pattern matches the raw text (used for unstructured
    /// build-stage warnings that have not been wrapped in a `Diagnostic`).
    pub fn matches_text(&self, text: &str) -> bool {
        self.patterns.iter().any(|p| p.is_match(text))
    }

    /// Drop every diagnostic whose `code`, `rule_id`, or `message` matches a
    /// configured pattern. Returns `(kept, suppressed_count)`.
    pub fn filter(&self, diagnostics: Vec<Diagnostic>) -> (Vec<Diagnostic>, usize) {
        if self.patterns.is_empty() {
            return (diagnostics, 0);
        }
        let total = diagnostics.len();
        let kept: Vec<Diagnostic> = diagnostics
            .into_iter()
            .filter(|d| !self.matches(d))
            .collect();
        let suppressed = total - kept.len();
        (kept, suppressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Location, Severity};
    use std::path::PathBuf;

    fn diag(rule_id: &str, code: Option<&str>, message: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.into(),
            severity: Severity::Warning,
            message: message.into(),
            location: Location {
                file: PathBuf::from("test.fsh"),
                line: 1,
                column: 1,
                end_line: None,
                end_column: None,
                offset: 0,
                length: 0,
                span: None,
            },
            suggestions: vec![],
            code_snippet: None,
            code: code.map(String::from),
            source: None,
            category: None,
        }
    }

    #[test]
    fn empty_file_yields_empty_set() {
        let s = IgnoreErrors::from_text("");
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn comments_and_blank_lines_skipped() {
        let s = IgnoreErrors::from_text("# header\n\n  \n# trailing\n");
        assert!(s.is_empty());
    }

    #[test]
    fn invalid_regex_dropped_with_warning() {
        // Unbalanced bracket → invalid; should be skipped silently (we only warn).
        let s = IgnoreErrors::from_text("^FSH0\\d+$\n[unclosed\n^OK$\n");
        // 2 valid patterns kept, the bad one dropped.
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn matches_by_rule_id_code_or_message() {
        let s = IgnoreErrors::from_text(
            "^FSH0001$\n\
             ^naming-convention$\n\
             cardinality .* invalid\n",
        );

        assert!(s.matches(&diag("naming-convention", None, "Identifier should be PascalCase")));
        assert!(s.matches(&diag("other", Some("FSH0001"), "anything")));
        assert!(s.matches(&diag("other", None, "cardinality 0..3 invalid here")));
        assert!(!s.matches(&diag("other", Some("FSH9999"), "untouched")));
    }

    #[test]
    fn filter_drops_matching_and_counts() {
        let s = IgnoreErrors::from_text("^drop-me$\n");
        let (kept, suppressed) = s.filter(vec![
            diag("drop-me", None, "x"),
            diag("keep", None, "y"),
            diag("drop-me", None, "z"),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(suppressed, 2);
        assert_eq!(kept[0].rule_id, "keep");
    }

    #[test]
    fn empty_set_is_no_op_filter() {
        let s = IgnoreErrors::new();
        let (kept, suppressed) = s.filter(vec![diag("any", None, "msg")]);
        assert_eq!(kept.len(), 1);
        assert_eq!(suppressed, 0);
    }
}
