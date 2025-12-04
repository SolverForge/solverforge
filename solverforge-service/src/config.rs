use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub port: u16,
    pub startup_timeout: Duration,
    pub java_home: Option<PathBuf>,
    pub java_opts: Vec<String>,
    pub submodule_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            port: 0, // Auto-select
            startup_timeout: Duration::from_secs(60),
            java_home: None,
            java_opts: vec![],
            submodule_dir: None,
            cache_dir: None,
        }
    }
}

impl ServiceConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    pub fn with_java_home(mut self, path: impl Into<PathBuf>) -> Self {
        self.java_home = Some(path.into());
        self
    }

    pub fn with_java_opt(mut self, opt: impl Into<String>) -> Self {
        self.java_opts.push(opt.into());
        self
    }

    pub fn with_java_opts(mut self, opts: Vec<String>) -> Self {
        self.java_opts = opts;
        self
    }

    pub fn with_submodule_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.submodule_dir = Some(path.into());
        self
    }

    pub fn with_cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(path.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServiceConfig::default();
        assert_eq!(config.port, 0);
        assert_eq!(config.startup_timeout, Duration::from_secs(60));
        assert!(config.java_home.is_none());
        assert!(config.java_opts.is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let config = ServiceConfig::new()
            .with_port(8080)
            .with_startup_timeout(Duration::from_secs(30))
            .with_java_home("/usr/lib/jvm/java-21")
            .with_java_opt("-Xmx2g");

        assert_eq!(config.port, 8080);
        assert_eq!(config.startup_timeout, Duration::from_secs(30));
        assert_eq!(
            config.java_home,
            Some(PathBuf::from("/usr/lib/jvm/java-21"))
        );
        assert_eq!(config.java_opts, vec!["-Xmx2g".to_string()]);
    }

    #[test]
    fn test_multiple_java_opts() {
        let config = ServiceConfig::new()
            .with_java_opt("-Xmx2g")
            .with_java_opt("-Xms512m");

        assert_eq!(config.java_opts.len(), 2);
    }
}
