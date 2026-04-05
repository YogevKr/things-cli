use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ThingError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    InvalidInput(String),

    #[error("Things automation failed: {0}")]
    Automation(String),

    #[error("failed to read or write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl ThingError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::InvalidInput(_) => "invalid_input",
            Self::Io { .. } | Self::Json { .. } | Self::Automation(_) => "internal",
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::NotFound(_) => 2,
            Self::Conflict(_) => 3,
            Self::InvalidInput(_) => 4,
            Self::Io { .. } | Self::Json { .. } | Self::Automation(_) => 1,
        }
    }
}
