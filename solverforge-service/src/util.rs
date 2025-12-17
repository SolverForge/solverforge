use crate::error::{ServiceError, ServiceResult};
use log::debug;
use std::env;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub fn find_available_port() -> ServiceResult<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

pub fn find_java(java_home: Option<&Path>) -> ServiceResult<PathBuf> {
    if let Some(home) = java_home {
        let java_path = home.join("bin").join("java");
        if java_path.exists() {
            return Ok(java_path);
        }
        return Err(ServiceError::JavaNotFound(format!(
            "java not found in JAVA_HOME: {}",
            home.display()
        )));
    }

    if let Ok(home) = env::var("JAVA_HOME") {
        let java_path = PathBuf::from(&home).join("bin").join("java");
        if java_path.exists() {
            return Ok(java_path);
        }
    }

    if let Ok(output) = std::process::Command::new("which").arg("java").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    Err(ServiceError::JavaNotFound(
        "java not found in PATH or JAVA_HOME".to_string(),
    ))
}

pub fn find_maven() -> ServiceResult<PathBuf> {
    if let Ok(output) = std::process::Command::new("which").arg("mvn").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                // Canonicalize the path to resolve symlinks.
                // Maven's startup script uses $0 to derive MAVEN_HOME, so calling
                // it via a symlink like /bin/mvn (which points to ../share/maven/bin/mvn)
                // can fail if /bin doesn't have a /share sibling directory.
                let path_buf = PathBuf::from(&path);
                return Ok(path_buf.canonicalize().unwrap_or(path_buf));
            }
        }
    }

    Err(ServiceError::MavenNotFound(
        "mvn not found in PATH".to_string(),
    ))
}

pub fn wait_for_ready(url: &str, timeout: Duration) -> ServiceResult<()> {
    let start = Instant::now();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| ServiceError::Http(e.to_string()))?;

    debug!("Waiting for service to be ready: {}", url);

    loop {
        if start.elapsed() > timeout {
            return Err(ServiceError::Unhealthy(format!(
                "Service did not become ready within {:?}",
                timeout
            )));
        }

        match client.get(url).send() {
            Ok(response) if response.status().is_success() => {
                debug!("Service is ready after {:?}", start.elapsed());
                return Ok(());
            }
            Ok(response) => {
                debug!("Health check returned {}", response.status());
            }
            Err(e) => {
                debug!("Service not ready yet: {}", e);
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }
}

pub fn get_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("solverforge")
}

pub fn find_submodule_dir() -> ServiceResult<PathBuf> {
    // 1. Check SOLVERFORGE_SERVICE_DIR environment variable
    if let Ok(dir) = env::var("SOLVERFORGE_SERVICE_DIR") {
        let path = PathBuf::from(&dir);
        if path.is_dir() && path.join("pom.xml").exists() {
            debug!("Using SOLVERFORGE_SERVICE_DIR: {}", dir);
            return Ok(path);
        }
    }

    // 2. Walk up from current directory
    let mut current = env::current_dir()?;
    loop {
        let candidate = current.join("solverforge-wasm-service");
        if candidate.is_dir() && candidate.join("pom.xml").exists() {
            return Ok(candidate);
        }

        if !current.pop() {
            break;
        }
    }

    // 3. Check CARGO_MANIFEST_DIR (for development builds)
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let workspace_root = PathBuf::from(manifest_dir)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let candidate = workspace_root.join("solverforge-wasm-service");
        if candidate.is_dir() && candidate.join("pom.xml").exists() {
            return Ok(candidate);
        }
    }

    Err(ServiceError::SubmoduleNotFound(
        "solverforge-wasm-service submodule not found. Set SOLVERFORGE_SERVICE_DIR or ensure JAR is available on Maven Central.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_available_port() {
        let port = find_available_port().unwrap();
        assert!(port > 0);

        let port2 = find_available_port().unwrap();
        assert!(port2 > 0);
    }

    #[test]
    fn test_get_cache_dir() {
        let cache = get_cache_dir();
        assert!(cache.to_string_lossy().contains("solverforge"));
    }

    #[test]
    fn test_find_java_with_invalid_home() {
        let result = find_java(Some(Path::new("/nonexistent/path")));
        assert!(result.is_err());
    }
}
