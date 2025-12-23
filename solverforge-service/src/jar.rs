use crate::error::{ServiceError, ServiceResult};
use crate::util::{find_java, find_maven, find_submodule_dir, get_cache_dir};
use log::{debug, info, warn};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

// Maven Central coordinates
const MAVEN_GROUP_ID: &str = "org.solverforge";
const MAVEN_ARTIFACT_ID: &str = "solverforge-wasm-service";
const MAVEN_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAVEN_CENTRAL_URL: &str = "https://repo1.maven.org/maven2";

pub struct JarManager {
    /// Optional path to the solverforge-wasm-service submodule (for local builds).
    submodule_dir: Option<PathBuf>,
    cache_dir: PathBuf,
    java_home: Option<PathBuf>,
}

impl Default for JarManager {
    fn default() -> Self {
        Self::new()
    }
}

impl JarManager {
    pub fn new() -> Self {
        // Try to find submodule, but don't fail - we can download from Maven Central
        let submodule_dir = find_submodule_dir().ok();
        let cache_dir = get_cache_dir();
        Self {
            submodule_dir,
            cache_dir,
            java_home: None,
        }
    }

    pub fn with_paths(submodule_dir: PathBuf, cache_dir: PathBuf) -> Self {
        Self {
            submodule_dir: Some(submodule_dir),
            cache_dir,
            java_home: None,
        }
    }

    pub fn with_java_home(mut self, java_home: Option<&Path>) -> Self {
        self.java_home = java_home.map(|p| p.to_path_buf());
        self
    }

    pub fn ensure_jar(&self) -> ServiceResult<PathBuf> {
        let jar_path = self.jar_path();

        // 1. Check cache first
        if jar_path.exists() {
            debug!("Using cached JAR: {}", jar_path.display());
            return Ok(jar_path);
        }

        // 2. Try local build if submodule exists (dev mode)
        if let Some(ref submodule_dir) = self.submodule_dir {
            if submodule_dir.join("pom.xml").exists() {
                info!("Building solverforge-wasm-service JAR from submodule...");
                match self.build_jar() {
                    Ok(()) => {
                        if jar_path.exists() {
                            return Ok(jar_path);
                        }
                    }
                    Err(e) => {
                        warn!("Local build failed: {}, trying Maven download...", e);
                    }
                }
            }
        }

        // 3. Download from Maven Central (production mode)
        info!("Downloading solverforge-wasm-service from Maven Central...");
        self.download_from_maven()?;

        if !jar_path.exists() {
            return Err(ServiceError::BuildFailed(
                "JAR not found after download".to_string(),
            ));
        }

        // Clean up old versions
        if let Ok(removed) = self.cleanup_old_versions() {
            if removed > 0 {
                info!("Cleaned up {} old JAR version(s) from cache", removed);
            }
        }

        Ok(jar_path)
    }

    pub fn jar_exists(&self) -> bool {
        self.jar_path().exists()
    }

    pub fn jar_path(&self) -> PathBuf {
        // Uber-jar is a single self-contained JAR
        self.cache_dir.join(format!(
            "{}-{}-runner.jar",
            MAVEN_ARTIFACT_ID, MAVEN_VERSION
        ))
    }

    pub fn rebuild(&self) -> ServiceResult<PathBuf> {
        let jar_path = self.jar_path();
        if jar_path.exists() {
            fs::remove_file(&jar_path)?;
        }
        self.ensure_jar()
    }

    fn build_jar(&self) -> ServiceResult<()> {
        let submodule_dir = self.submodule_dir.as_ref().ok_or_else(|| {
            ServiceError::SubmoduleNotFound(
                "Cannot build JAR: submodule directory not configured".to_string(),
            )
        })?;

        let mvn = find_maven()?;

        fs::create_dir_all(&self.cache_dir)?;

        // Determine JAVA_HOME for Maven - it must use the same Java version
        // that solverforge-wasm-service was compiled with (Java 24)
        let java_home = if let Some(ref home) = self.java_home {
            home.clone()
        } else {
            // Find java and derive JAVA_HOME from it
            let java = find_java(None)?;
            // java is typically at $JAVA_HOME/bin/java, so go up two levels
            java.parent()
                .and_then(|bin| bin.parent())
                .map(|home| home.to_path_buf())
                .ok_or_else(|| {
                    ServiceError::JavaNotFound("Cannot determine JAVA_HOME from java path".into())
                })?
        };

        info!(
            "Running mvn package in {} with JAVA_HOME={}",
            submodule_dir.display(),
            java_home.display()
        );

        let output = Command::new(&mvn)
            .current_dir(submodule_dir)
            .env("JAVA_HOME", &java_home)
            .args(["package", "-DskipTests", "-q"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ServiceError::BuildFailed(format!(
                "Maven build failed: {}",
                stderr
            )));
        }

        // Uber-jar is named <artifactId>-<version>-runner.jar
        let built_jar = submodule_dir.join("target").join(format!(
            "{}-{}-runner.jar",
            MAVEN_ARTIFACT_ID, MAVEN_VERSION
        ));

        if !built_jar.exists() {
            return Err(ServiceError::BuildFailed(format!(
                "Expected JAR not found at {}",
                built_jar.display()
            )));
        }

        fs::create_dir_all(&self.cache_dir)?;
        let cached_jar = self.jar_path();

        info!("Copying JAR to cache: {}", cached_jar.display());
        fs::copy(&built_jar, &cached_jar)?;

        // Clean up old versions
        if let Ok(removed) = self.cleanup_old_versions() {
            if removed > 0 {
                info!("Cleaned up {} old JAR version(s) from cache", removed);
            }
        }

        Ok(())
    }

    fn download_from_maven(&self) -> ServiceResult<()> {
        // Maven Central URL pattern: /group/artifact/version/artifact-version-classifier.jar
        let group_path = MAVEN_GROUP_ID.replace('.', "/");
        let jar_url = format!(
            "{}/{}/{}/{}/{}-{}-runner.jar",
            MAVEN_CENTRAL_URL,
            group_path,
            MAVEN_ARTIFACT_ID,
            MAVEN_VERSION,
            MAVEN_ARTIFACT_ID,
            MAVEN_VERSION
        );

        info!("Downloading from: {}", jar_url);

        let response = reqwest::blocking::get(&jar_url)
            .map_err(|e| ServiceError::DownloadFailed(format!("Failed to download JAR: {}", e)))?;

        if !response.status().is_success() {
            return Err(ServiceError::DownloadFailed(format!(
                "HTTP {}: {}",
                response.status(),
                jar_url
            )));
        }

        let bytes = response
            .bytes()
            .map_err(|e| ServiceError::DownloadFailed(format!("Failed to read response: {}", e)))?;

        fs::create_dir_all(&self.cache_dir)?;
        let jar_path = self.jar_path();

        let mut file = File::create(&jar_path)?;
        file.write_all(&bytes)?;

        info!("Downloaded JAR to: {}", jar_path.display());
        Ok(())
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Clear all cached JARs. Use when you need a fresh download.
    pub fn clear_cache(&self) -> ServiceResult<usize> {
        let mut removed = 0;

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().map(|s| s.to_string_lossy()) {
                    if name.starts_with("solverforge-wasm-service-")
                        && name.ends_with("-runner.jar")
                    {
                        info!("Removing cached JAR: {}", path.display());
                        if fs::remove_file(&path).is_ok() {
                            removed += 1;
                        }
                    }
                }
            }
        }

        // Also remove quarkus-app directory if it exists (legacy)
        let quarkus_dir = self.cache_dir.join("quarkus-app");
        if quarkus_dir.is_dir() {
            info!("Removing legacy quarkus-app directory");
            if fs::remove_dir_all(&quarkus_dir).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Clean up old JAR versions from cache, keeping only the current version.
    pub fn cleanup_old_versions(&self) -> ServiceResult<usize> {
        let current_jar = self.jar_path();
        let current_name = current_jar
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut removed = 0;

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().map(|s| s.to_string_lossy()) {
                    // Only remove old solverforge-wasm-service JARs
                    if name.starts_with("solverforge-wasm-service-")
                        && name.ends_with("-runner.jar")
                        && name != current_name
                    {
                        info!("Removing old JAR: {}", path.display());
                        if fs::remove_file(&path).is_ok() {
                            removed += 1;
                        }
                    }
                }
            }
        }

        // Also remove quarkus-app directory if it exists (legacy)
        let quarkus_dir = self.cache_dir.join("quarkus-app");
        if quarkus_dir.is_dir() {
            info!("Removing legacy quarkus-app directory");
            if fs::remove_dir_all(&quarkus_dir).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_jar_path() {
        let temp = TempDir::new().unwrap();
        let manager =
            JarManager::with_paths(PathBuf::from("/fake/submodule"), temp.path().to_path_buf());

        let jar_path = manager.jar_path();
        // Uber-jar is named <artifactId>-<version>-runner.jar
        assert!(jar_path
            .to_string_lossy()
            .contains("solverforge-wasm-service"));
        assert!(jar_path.to_string_lossy().contains("-runner.jar"));
    }

    #[test]
    fn test_jar_exists_false() {
        let temp = TempDir::new().unwrap();
        let manager =
            JarManager::with_paths(PathBuf::from("/fake/submodule"), temp.path().to_path_buf());

        assert!(!manager.jar_exists());
    }

    #[test]
    fn test_cache_dir() {
        let temp = TempDir::new().unwrap();
        let manager =
            JarManager::with_paths(PathBuf::from("/fake/submodule"), temp.path().to_path_buf());

        assert_eq!(manager.cache_dir(), temp.path());
    }
}
