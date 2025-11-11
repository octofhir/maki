//! Error types for the decompiler

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the decompiler
#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("Failed to parse {file}: {message}")]
    ParseError { file: PathBuf, message: String },

    #[error("Missing base definition for StructureDefinition")]
    MissingBaseDefinition,

    #[error("Invalid FHIR version: {0}")]
    InvalidFhirVersion(String),

    #[error("Invalid package specification: {0}")]
    InvalidPackageSpec(String),

    #[error("Canonical manager error: {0}")]
    CanonicalError(String),

    #[error("No element definitions found")]
    NoElementDefinitions,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::DeError),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;
