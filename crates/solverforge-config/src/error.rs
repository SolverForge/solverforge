use thiserror::Error;

/// Configuration error
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The configuration file could not be read.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The text is not valid TOML or does not match the configuration schema,
    /// including unknown keys.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    /// The configuration parsed but is semantically invalid.
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}
