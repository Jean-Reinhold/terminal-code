use std::io;

#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: io::Error,
    },

    #[error("invalid harness input: {0}")]
    Invalid(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("YAML error: {0}")]
    Yaml(String),

    #[error("process error: {0}")]
    Process(String),

    #[error("artifact integrity error: {0}")]
    Integrity(String),
}

impl HarnessError {
    pub fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, HarnessError>;
