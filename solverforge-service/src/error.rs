use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Java not found: {0}")]
    JavaNotFound(String),

    #[error("Maven not found: {0}")]
    MavenNotFound(String),

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Service failed to start: {0}")]
    StartFailed(String),

    #[error("Service unhealthy: {0}")]
    Unhealthy(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Submodule not found: {0}")]
    SubmoduleNotFound(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ServiceError::JavaNotFound("java not in PATH".to_string());
        assert!(err.to_string().contains("Java not found"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ServiceError = io_err.into();
        assert!(matches!(err, ServiceError::Io(_)));
    }
}
