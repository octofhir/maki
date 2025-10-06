//! Terminal console utilities for rich output

use std::env;
use std::io::{self, IsTerminal};

/// Console output handler with color support and terminal width detection
pub struct Console {
    color_enabled: bool,
    max_width: usize,
}

impl Console {
    /// Create a new console with automatic color and terminal detection
    pub fn new() -> Self {
        Self {
            // Use modern Rust stdlib IsTerminal (not deprecated atty!)
            color_enabled: io::stdout().is_terminal() && env::var("NO_COLOR").is_err(),
            max_width: Self::detect_terminal_width(),
        }
    }

    /// Detect terminal width, defaulting to 100 if unavailable
    fn detect_terminal_width() -> usize {
        term_size::dimensions().map(|(w, _)| w).unwrap_or(100)
    }

    /// Check if color output is enabled
    pub fn is_color_enabled(&self) -> bool {
        self.color_enabled
    }

    /// Colorize text with the specified color
    pub fn colorize(&self, text: &str, color: Color) -> String {
        if !self.color_enabled {
            return text.to_string();
        }

        match color {
            Color::Red => format!("\x1b[31m{text}\x1b[0m"),
            Color::Yellow => format!("\x1b[33m{text}\x1b[0m"),
            Color::Blue => format!("\x1b[34m{text}\x1b[0m"),
            Color::Green => format!("\x1b[32m{text}\x1b[0m"),
            Color::Cyan => format!("\x1b[36m{text}\x1b[0m"),
            Color::Magenta => format!("\x1b[35m{text}\x1b[0m"),
            Color::Dim => format!("\x1b[2m{text}\x1b[0m"),
            Color::Bold => format!("\x1b[1m{text}\x1b[0m"),
            Color::Underline => format!("\x1b[4m{text}\x1b[0m"),
        }
    }

    /// Get maximum width for terminal output
    pub fn max_width(&self) -> usize {
        self.max_width
    }

    /// Create a console with colors disabled
    pub fn no_colors() -> Self {
        Self {
            color_enabled: false,
            max_width: Self::detect_terminal_width(),
        }
    }

    /// Create a console with a specific max width
    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = width;
        self
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

/// ANSI color codes for terminal output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Yellow,
    Blue,
    Green,
    Cyan,
    Magenta,
    Dim,
    Bold,
    Underline,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_creation() {
        let console = Console::new();
        assert!(console.max_width() > 0);
    }

    #[test]
    fn test_no_colors() {
        let console = Console::no_colors();
        assert!(!console.is_color_enabled());

        let text = console.colorize("test", Color::Red);
        assert_eq!(text, "test");
    }

    #[test]
    fn test_colorize_when_disabled() {
        let console = Console::no_colors();
        let result = console.colorize("hello", Color::Red);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_with_max_width() {
        let console = Console::new().with_max_width(80);
        assert_eq!(console.max_width(), 80);
    }

    #[test]
    fn test_color_variants() {
        let console = Console::no_colors(); // Use no colors for predictable testing

        assert_eq!(console.colorize("text", Color::Red), "text");
        assert_eq!(console.colorize("text", Color::Yellow), "text");
        assert_eq!(console.colorize("text", Color::Blue), "text");
        assert_eq!(console.colorize("text", Color::Green), "text");
        assert_eq!(console.colorize("text", Color::Cyan), "text");
        assert_eq!(console.colorize("text", Color::Magenta), "text");
        assert_eq!(console.colorize("text", Color::Dim), "text");
        assert_eq!(console.colorize("text", Color::Bold), "text");
        assert_eq!(console.colorize("text", Color::Underline), "text");
    }
}
