//! Configuration file discovery and loading

use super::maki_config::MakiConfiguration;
use crate::error::MakiError;
use std::fs;
use std::path::{Path, PathBuf};
use toml;

/// Result type for configuration operations
pub type Result<T> = std::result::Result<T, MakiError>;

/// Configuration loader for discovering and loading config files
pub struct ConfigLoader;

impl ConfigLoader {
    /// Auto-discover config file by traversing upward from start_path
    ///
    /// Searches for `maki.jsonc` or `maki.json` files starting from
    /// the given directory and moving up the directory tree until a config
    /// is found or the filesystem root is reached.
    ///
    /// If a config with `root: true` is found, the search stops there.
    pub fn auto_discover(start_path: &Path) -> Result<Option<PathBuf>> {
        let mut current = start_path
            .canonicalize()
            .map_err(|e| MakiError::ConfigError {
                message: format!("Invalid path: {e}"),
            })?;

        loop {
            // Try multiple config file names (prefer dotfiles, then .jsonc)
            for filename in &[
                ".makirc.json",
                ".makirc.jsonc",
                ".makirc.toml",
                "maki.jsonc",
                "maki.json",
                "maki.toml",
            ] {
                let config_path = current.join(filename);
                if config_path.exists() && config_path.is_file() {
                    tracing::debug!("Found config: {}", config_path.display());

                    // Check if this config has root: true
                    if let Ok(config) = Self::load_from_file(&config_path)
                        && config.root == Some(true)
                    {
                        tracing::debug!("Config has root: true, stopping search");
                        return Ok(Some(config_path));
                    }

                    // If no root flag, keep searching up but remember this one
                    return Ok(Some(config_path));
                }
            }

            // Move up to parent directory
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                // Reached filesystem root
                break;
            }
        }

        Ok(None)
    }

    /// Load configuration from a specific file
    ///
    /// Supports both JSON and JSONC (JSON with comments and trailing commas)
    pub fn load_from_file(path: &Path) -> Result<MakiConfiguration> {
        let content = fs::read_to_string(path).map_err(|e| MakiError::ConfigError {
            message: format!("Cannot read config file '{}': {}", path.display(), e),
        })?;

        let config: MakiConfiguration = match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => toml::from_str(&content).map_err(|e| MakiError::ConfigError {
                message: format!("Invalid TOML in '{}': {}", path.display(), e),
            })?,
            _ => {
                // Use json5 for JSONC support (comments + trailing commas)
                json5::from_str(&content).map_err(|e| MakiError::ConfigError {
                    message: format!("Invalid JSON in '{}': {}", path.display(), e),
                })?
            }
        };

        tracing::info!("Loaded config from: {}", path.display());
        Ok(config)
    }

    /// Load config with extends resolution
    ///
    /// This method loads a configuration file and recursively resolves
    /// any `extends` directives, merging parent configurations.
    pub fn load_with_extends(path: &Path) -> Result<MakiConfiguration> {
        let mut config = Self::load_from_file(path)?;

        // Apply extends
        if config.extends.is_some() {
            let base_dir = path.parent().ok_or_else(|| MakiError::ConfigError {
                message: format!("Cannot determine parent directory of '{}'", path.display()),
            })?;
            Self::apply_extends(&mut config, base_dir)?;
        }

        Ok(config)
    }

    /// Recursively apply extends
    ///
    /// Loads and merges parent configurations specified in the `extends` field.
    /// Parent configurations are loaded first, then the current config is merged
    /// on top, ensuring that child configs override parent values.
    pub fn apply_extends(config: &mut MakiConfiguration, base_path: &Path) -> Result<()> {
        if let Some(extends_paths) = &config.extends.clone() {
            for extend_path in extends_paths {
                let full_path = if Path::new(extend_path).is_absolute() {
                    PathBuf::from(extend_path)
                } else {
                    base_path.join(extend_path)
                };

                if !full_path.exists() {
                    return Err(MakiError::ConfigError {
                        message: format!("Extended config not found: {}", full_path.display()),
                    });
                }

                // Load parent config
                let mut parent_config = Self::load_from_file(&full_path)?;

                // Recursively apply parent's extends
                let parent_base = full_path.parent().ok_or_else(|| MakiError::ConfigError {
                    message: format!(
                        "Cannot determine parent directory of '{}'",
                        full_path.display()
                    ),
                })?;
                Self::apply_extends(&mut parent_config, parent_base)?;

                // Merge parent into current (current takes precedence)
                config.merge_with(parent_config);
            }
        }

        Ok(())
    }

    /// Load config from path or auto-discover
    ///
    /// If a custom path is provided, loads from that path.
    /// Otherwise, attempts to auto-discover a config file starting from
    /// the current directory.
    pub fn load(custom_path: Option<&Path>) -> Result<MakiConfiguration> {
        let config_path = if let Some(path) = custom_path {
            // Use provided path
            if !path.exists() {
                return Err(MakiError::ConfigError {
                    message: format!("Config file not found: {}", path.display()),
                });
            }
            path.to_path_buf()
        } else {
            // Auto-discover
            let current_dir = std::env::current_dir().map_err(|e| MakiError::ConfigError {
                message: format!("Failed to get current directory: {e}"),
            })?;

            Self::auto_discover(&current_dir)?.ok_or_else(|| MakiError::ConfigError {
                message: "No config file found. Run 'maki init' to create one.".to_string(),
            })?
        };

        Self::load_with_extends(&config_path)
    }

    /// Load config or use default if not found
    ///
    /// Similar to `load()`, but returns a default configuration instead
    /// of an error if no config file is found.
    pub fn load_or_default(custom_path: Option<&Path>) -> MakiConfiguration {
        Self::load(custom_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}. Using defaults.", e);
            MakiConfiguration::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_temp_config(dir: &Path, filename: &str, content: &str) -> PathBuf {
        let path = dir.join(filename);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_from_file_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_temp_config(
            temp_dir.path(),
            "maki.json",
            r#"{
                "linter": {
                    "enabled": true
                }
            }"#,
        );

        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        assert!(config.linter.is_some());
        assert_eq!(config.linter.unwrap().enabled, Some(true));
    }

    #[test]
    fn test_load_from_file_jsonc() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_temp_config(
            temp_dir.path(),
            "maki.jsonc",
            r#"{
                // This is a comment
                "linter": {
                    "enabled": true, // trailing comma OK
                }
            }"#,
        );

        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        assert!(config.linter.is_some());
    }

    #[test]
    fn test_auto_discover() {
        let temp_dir = TempDir::new().unwrap();
        let nested = temp_dir.path().join("src/nested");
        fs::create_dir_all(&nested).unwrap();

        // Create config in root
        create_temp_config(temp_dir.path(), "maki.json", r#"{"root": true}"#);

        // Search from nested directory
        let found = ConfigLoader::auto_discover(&nested).unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn test_extends_resolution() {
        let temp_dir = TempDir::new().unwrap();

        // Create base config
        create_temp_config(
            temp_dir.path(),
            "base.json",
            r#"{
                "linter": {
                    "enabled": true,
                    "rules": {
                        "recommended": true
                    }
                }
            }"#,
        );

        // Create extending config
        create_temp_config(
            temp_dir.path(),
            "maki.json",
            r#"{
                "extends": ["base.json"],
                "linter": {
                    "rules": {
                        "correctness": {
                            "duplicate-definition": "error"
                        }
                    }
                }
            }"#,
        );

        let config_path = temp_dir.path().join("maki.json");
        let config = ConfigLoader::load_with_extends(&config_path).unwrap();

        assert!(config.linter.is_some());
        let linter = config.linter.unwrap();
        assert_eq!(linter.enabled, Some(true));
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = ConfigLoader::load_from_file(Path::new("nonexistent.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path =
            create_temp_config(temp_dir.path(), "invalid.json", r#"{ invalid json }"#);

        let result = ConfigLoader::load_from_file(&config_path);
        assert!(result.is_err());
    }
}
