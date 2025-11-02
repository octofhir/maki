//! Configuration file discovery and loading

use super::unified_config::UnifiedConfig;
use crate::error::MakiError;
use std::path::{Path, PathBuf};

/// Result type for configuration operations
pub type Result<T> = std::result::Result<T, MakiError>;

/// Configuration loader for discovering and loading config files
pub struct ConfigLoader;

impl ConfigLoader {
    /// Auto-discover config file by traversing upward from start_path
    ///
    /// Searches for `maki.yaml`, `maki.yml`, or `maki.json` files starting from
    /// the given directory and moving up the directory tree until a config
    /// is found or the filesystem root is reached.
    pub fn auto_discover(start_path: &Path) -> Result<Option<PathBuf>> {
        let mut current = start_path
            .canonicalize()
            .map_err(|e| MakiError::ConfigError {
                message: format!("Invalid path: {e}"),
            })?;

        loop {
            // Try config file names: maki.yaml, maki.yml, maki.json
            for filename in &["maki.yaml", "maki.yml", "maki.json"] {
                let config_path = current.join(filename);
                if config_path.exists() && config_path.is_file() {
                    tracing::debug!("Found config: {}", config_path.display());
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
    /// Supports YAML (.yaml, .yml) and JSON (.json) formats
    pub fn load_from_file(path: &Path) -> Result<UnifiedConfig> {
        UnifiedConfig::load(path).map_err(|e| MakiError::ConfigError {
            message: format!("Failed to load config from '{}': {}", path.display(), e),
        })
    }

    /// Load config from path or auto-discover
    ///
    /// If a custom path is provided, loads from that path.
    /// Otherwise, attempts to auto-discover a config file starting from
    /// the given directory (or current directory).
    pub fn load(custom_path: Option<&Path>, start_dir: Option<&Path>) -> Result<UnifiedConfig> {
        let config_path = if let Some(path) = custom_path {
            // Use provided path
            if !path.exists() {
                return Err(MakiError::ConfigError {
                    message: format!(
                        "Config file not found: {}. Run 'maki migrate' if migrating from SUSHI.",
                        path.display()
                    ),
                });
            }
            path.to_path_buf()
        } else {
            // Auto-discover
            let search_dir = start_dir.unwrap_or_else(|| Path::new("."));
            let current_dir = search_dir
                .canonicalize()
                .map_err(|e| MakiError::ConfigError {
                    message: format!("Failed to resolve directory: {e}"),
                })?;

            Self::auto_discover(&current_dir)?.ok_or_else(|| MakiError::ConfigError {
                message: "No maki.yaml or maki.json found. Run 'maki migrate' to convert sushi-config.yaml to maki.yaml".to_string(),
            })?
        };

        Self::load_from_file(&config_path)
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
