//! Built-in rewrite functions for GritQL patterns
//!
//! Provides common transformation functions:
//! - capitalize: Capitalize first letter
//! - to_kebab_case: Convert to kebab-case
//! - to_pascal_case: Convert to PascalCase
//! - to_snake_case: Convert to snake_case
//! - trim: Remove whitespace
//! - replace: String replacement
//! - concat: Concatenate strings

use maki_core::Result;
use std::collections::HashMap;

/// Type alias for a rewrite function
pub type RewriteFunc = fn(&[FunctionValue]) -> Result<FunctionValue>;

/// A value that can be returned from a rewrite function
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionValue {
    String(String),
    Number(i64),
    Boolean(bool),
}

impl FunctionValue {
    /// Get the string representation
    pub fn as_string(&self) -> Result<String> {
        match self {
            FunctionValue::String(s) => Ok(s.clone()),
            FunctionValue::Number(n) => Ok(n.to_string()),
            FunctionValue::Boolean(b) => Ok(b.to_string()),
        }
    }
}

impl From<String> for FunctionValue {
    fn from(s: String) -> Self {
        FunctionValue::String(s)
    }
}

impl From<&str> for FunctionValue {
    fn from(s: &str) -> Self {
        FunctionValue::String(s.to_string())
    }
}

/// Registry of rewrite functions
pub struct RewriteFunctionRegistry {
    functions: HashMap<String, RewriteFunc>,
}

impl Default for RewriteFunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RewriteFunctionRegistry {
    /// Create a new function registry with all built-in functions
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    /// Register all built-in functions
    fn register_builtins(&mut self) {
        self.register("capitalize", RewriteFunctions::capitalize);
        self.register("to_kebab_case", RewriteFunctions::to_kebab_case);
        self.register("to_pascal_case", RewriteFunctions::to_pascal_case);
        self.register("to_snake_case", RewriteFunctions::to_snake_case);
        self.register("trim", RewriteFunctions::trim);
        self.register("replace", RewriteFunctions::replace);
        self.register("concat", RewriteFunctions::concat);
        self.register("lowercase", RewriteFunctions::lowercase);
        self.register("uppercase", RewriteFunctions::uppercase);
    }

    /// Register a custom function
    pub fn register(&mut self, name: &str, func: RewriteFunc) {
        self.functions.insert(name.to_string(), func);
    }

    /// Call a function by name
    pub fn call(&self, name: &str, args: &[FunctionValue]) -> Result<FunctionValue> {
        let func = self.functions.get(name).ok_or_else(|| {
            maki_core::MakiError::rule_error(
                "gritql-functions",
                format!("Unknown function: {}", name),
            )
        })?;
        func(args)
    }

    /// Check if a function exists
    pub fn has(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }
}

/// Built-in rewrite functions
pub struct RewriteFunctions;

impl RewriteFunctions {
    /// Capitalize first letter
    /// Example: "badName" -> "BadName"
    pub fn capitalize(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.is_empty() {
            return Err(maki_core::MakiError::rule_error(
                "capitalize",
                "capitalize() requires 1 argument",
            ));
        }

        let text = args[0].as_string()?;
        let mut chars = text.chars();
        match chars.next() {
            None => Ok(FunctionValue::String(String::new())),
            Some(first) => {
                let capitalized = first.to_uppercase().chain(chars).collect();
                Ok(FunctionValue::String(capitalized))
            }
        }
    }

    /// Convert to kebab-case
    /// Example: "BadName" -> "bad-name"
    pub fn to_kebab_case(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.is_empty() {
            return Err(maki_core::MakiError::rule_error(
                "to_kebab_case",
                "to_kebab_case() requires 1 argument",
            ));
        }

        let text = args[0].as_string()?;
        let kebab = text
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    vec!['-', c.to_lowercase().next().unwrap_or(c)]
                } else {
                    vec![c.to_lowercase().next().unwrap_or(c)]
                }
            })
            .collect::<String>();
        Ok(FunctionValue::String(kebab))
    }

    /// Convert to PascalCase
    /// Example: "bad-name" -> "BadName"
    pub fn to_pascal_case(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.is_empty() {
            return Err(maki_core::MakiError::rule_error(
                "to_pascal_case",
                "to_pascal_case() requires 1 argument",
            ));
        }

        let text = args[0].as_string()?;
        let pascal = text
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<String>();
        Ok(FunctionValue::String(pascal))
    }

    /// Convert to snake_case
    /// Example: "BadName" -> "bad_name"
    pub fn to_snake_case(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.is_empty() {
            return Err(maki_core::MakiError::rule_error(
                "to_snake_case",
                "to_snake_case() requires 1 argument",
            ));
        }

        let text = args[0].as_string()?;
        let snake = text
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    vec!['_', c.to_lowercase().next().unwrap_or(c)]
                } else {
                    vec![c.to_lowercase().next().unwrap_or(c)]
                }
            })
            .collect::<String>();
        Ok(FunctionValue::String(snake))
    }

    /// Trim whitespace
    /// Example: " text " -> "text"
    pub fn trim(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.is_empty() {
            return Err(maki_core::MakiError::rule_error(
                "trim",
                "trim() requires 1 argument",
            ));
        }

        let text = args[0].as_string()?;
        Ok(FunctionValue::String(text.trim().to_string()))
    }

    /// Replace string
    /// Example: replace("bad_name", "_", "-") -> "bad-name"
    pub fn replace(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.len() < 3 {
            return Err(maki_core::MakiError::rule_error(
                "replace",
                "replace() requires 3 arguments: (text, old, new)",
            ));
        }

        let text = args[0].as_string()?;
        let old = args[1].as_string()?;
        let new = args[2].as_string()?;
        Ok(FunctionValue::String(text.replace(&old, &new)))
    }

    /// Concatenate strings
    /// Example: concat("prefix-", "suffix") -> "prefix-suffix"
    pub fn concat(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.len() < 2 {
            return Err(maki_core::MakiError::rule_error(
                "concat",
                "concat() requires at least 2 arguments",
            ));
        }

        let mut result = String::new();
        for arg in args {
            result.push_str(&arg.as_string()?);
        }
        Ok(FunctionValue::String(result))
    }

    /// Convert to lowercase
    /// Example: "BadName" -> "badname"
    pub fn lowercase(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.is_empty() {
            return Err(maki_core::MakiError::rule_error(
                "lowercase",
                "lowercase() requires 1 argument",
            ));
        }

        let text = args[0].as_string()?;
        Ok(FunctionValue::String(text.to_lowercase()))
    }

    /// Convert to uppercase
    /// Example: "badname" -> "BADNAME"
    pub fn uppercase(args: &[FunctionValue]) -> Result<FunctionValue> {
        if args.is_empty() {
            return Err(maki_core::MakiError::rule_error(
                "uppercase",
                "uppercase() requires 1 argument",
            ));
        }

        let text = args[0].as_string()?;
        Ok(FunctionValue::String(text.to_uppercase()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize() {
        let result =
            RewriteFunctions::capitalize(&[FunctionValue::String("badName".to_string())]).unwrap();
        assert_eq!(result.as_string().unwrap(), "BadName");
    }

    #[test]
    fn test_to_kebab_case() {
        let result =
            RewriteFunctions::to_kebab_case(&[FunctionValue::String("BadName".to_string())])
                .unwrap();
        assert_eq!(result.as_string().unwrap(), "bad-name");
    }

    #[test]
    fn test_to_pascal_case() {
        let result =
            RewriteFunctions::to_pascal_case(&[FunctionValue::String("bad-name".to_string())])
                .unwrap();
        assert_eq!(result.as_string().unwrap(), "BadName");
    }

    #[test]
    fn test_to_snake_case() {
        let result =
            RewriteFunctions::to_snake_case(&[FunctionValue::String("BadName".to_string())])
                .unwrap();
        assert_eq!(result.as_string().unwrap(), "bad_name");
    }

    #[test]
    fn test_trim() {
        let result =
            RewriteFunctions::trim(&[FunctionValue::String("  text  ".to_string())]).unwrap();
        assert_eq!(result.as_string().unwrap(), "text");
    }

    #[test]
    fn test_replace() {
        let result = RewriteFunctions::replace(&[
            FunctionValue::String("bad_name".to_string()),
            FunctionValue::String("_".to_string()),
            FunctionValue::String("-".to_string()),
        ])
        .unwrap();
        assert_eq!(result.as_string().unwrap(), "bad-name");
    }

    #[test]
    fn test_concat() {
        let result = RewriteFunctions::concat(&[
            FunctionValue::String("prefix-".to_string()),
            FunctionValue::String("suffix".to_string()),
        ])
        .unwrap();
        assert_eq!(result.as_string().unwrap(), "prefix-suffix");
    }

    #[test]
    fn test_lowercase() {
        let result =
            RewriteFunctions::lowercase(&[FunctionValue::String("BadName".to_string())]).unwrap();
        assert_eq!(result.as_string().unwrap(), "badname");
    }

    #[test]
    fn test_uppercase() {
        let result =
            RewriteFunctions::uppercase(&[FunctionValue::String("badname".to_string())]).unwrap();
        assert_eq!(result.as_string().unwrap(), "BADNAME");
    }

    #[test]
    fn test_registry_creation() {
        let registry = RewriteFunctionRegistry::new();
        assert!(registry.has("capitalize"));
        assert!(registry.has("to_kebab_case"));
        assert!(registry.has("trim"));
    }

    #[test]
    fn test_registry_call() {
        let registry = RewriteFunctionRegistry::new();
        let result = registry
            .call("capitalize", &[FunctionValue::String("hello".to_string())])
            .unwrap();
        assert_eq!(result.as_string().unwrap(), "Hello");
    }

    #[test]
    fn test_registry_unknown_function() {
        let registry = RewriteFunctionRegistry::new();
        let result = registry.call("unknown_function", &[]);
        assert!(result.is_err());
    }
}
