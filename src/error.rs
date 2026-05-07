use std::fmt;

#[derive(Debug)]
pub enum RouterError {
    Io(std::io::Error),
    Parse(String),
    Config(String),
    Embedding(String),
    NoExamples,
}

impl fmt::Display for RouterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouterError::Io(e) => write!(f, "IO error: {e}"),
            RouterError::Parse(msg) => write!(f, "Parse error: {msg}"),
            RouterError::Config(msg) => write!(f, "Config error: {msg}"),
            RouterError::Embedding(msg) => write!(f, "Embedding error: {msg}"),
            RouterError::NoExamples => write!(f, "No route examples loaded"),
        }
    }
}

impl std::error::Error for RouterError {}

impl From<std::io::Error> for RouterError {
    fn from(e: std::io::Error) -> Self {
        RouterError::Io(e)
    }
}

impl From<serde_json::Error> for RouterError {
    fn from(e: serde_json::Error) -> Self {
        RouterError::Parse(e.to_string())
    }
}

impl From<toml::de::Error> for RouterError {
    fn from(e: toml::de::Error) -> Self {
        RouterError::Config(e.to_string())
    }
}

/// Errors returned by `semrouter::testing::EvalSuite::from_dir`.
#[derive(Debug, thiserror::Error)]
pub enum EvalSuiteError {
    /// Failed to load `router.toml` from the fixture directory.
    #[error("loading router.toml: {0}")]
    ConfigLoad(#[from] RouterError),

    /// Failed to read `thresholds.toml` (file exists but unreadable).
    #[error("reading thresholds.toml: {0}")]
    ThresholdsRead(#[source] std::io::Error),

    /// Failed to parse `thresholds.toml`.
    #[error("parsing thresholds.toml: {0}")]
    ThresholdsParse(#[source] toml::de::Error),
}
