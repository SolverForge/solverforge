use crate::error::{ServiceError, ServiceResult};
use crate::util::{find_java, find_maven, find_submodule_dir, get_cache_dir};
use log::{debug, info};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct JarManager {
    submodule_dir: PathBuf,
    cache_dir: PathBuf,
    java_home: Option<PathBuf>,
}

impl JarManager {
    pub fn new() -> ServiceResult<Self> {
        let submodule_dir = find_submodule_dir()?;
        let cache_dir = get_cache_dir();
        Ok(Self {
            submodule_dir,
            cache_dir,
            java_home: None,
        })
    }

    pub fn with_paths(submodule_dir: PathBuf, cache_dir: PathBuf) -> Self {
        Self {
            submodule_dir,
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

        if jar_path.exists() {
            debug!("Using cached JAR: {}", jar_path.display());
            return Ok(jar_path);
        }

        info!("Building timefold-wasm-service JAR...");
        self.build_jar()?;

        if !jar_path.exists() {
            return Err(ServiceError::BuildFailed(
                "JAR not found after build".to_string(),
            ));
        }

        Ok(jar_path)
    }

    pub fn jar_exists(&self) -> bool {
        self.jar_path().exists()
    }

    pub fn jar_path(&self) -> PathBuf {
        // Must be inside quarkus-app directory for relative classpath to work
        self.cache_dir.join("quarkus-app").join("quarkus-run.jar")
    }

    pub fn rebuild(&self) -> ServiceResult<PathBuf> {
        let jar_path = self.jar_path();
        if jar_path.exists() {
            fs::remove_file(&jar_path)?;
        }
        self.ensure_jar()
    }

    fn build_jar(&self) -> ServiceResult<()> {
        let mvn = find_maven()?;

        fs::create_dir_all(&self.cache_dir)?;

        // Determine JAVA_HOME for Maven - it must use the same Java version
        // that timefold-wasm-service was compiled with (Java 24)
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
            self.submodule_dir.display(),
            java_home.display()
        );

        let output = Command::new(&mvn)
            .current_dir(&self.submodule_dir)
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

        let built_jar = self
            .submodule_dir
            .join("target")
            .join("quarkus-app")
            .join("quarkus-run.jar");

        if !built_jar.exists() {
            return Err(ServiceError::BuildFailed(format!(
                "Expected JAR not found at {}",
                built_jar.display()
            )));
        }

        let quarkus_app_dir = self.submodule_dir.join("target").join("quarkus-app");
        let cache_quarkus_dir = self.cache_dir.join("quarkus-app");

        if cache_quarkus_dir.exists() {
            fs::remove_dir_all(&cache_quarkus_dir)?;
        }

        info!(
            "Copying quarkus-app to cache: {}",
            cache_quarkus_dir.display()
        );
        copy_dir_all(&quarkus_app_dir, &cache_quarkus_dir)?;

        Ok(())
    }

    pub fn quarkus_app_dir(&self) -> PathBuf {
        self.cache_dir.join("quarkus-app")
    }
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> ServiceResult<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
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
        assert!(jar_path.to_string_lossy().contains("quarkus-app"));
        assert!(jar_path.to_string_lossy().contains("quarkus-run.jar"));
    }

    #[test]
    fn test_jar_exists_false() {
        let temp = TempDir::new().unwrap();
        let manager =
            JarManager::with_paths(PathBuf::from("/fake/submodule"), temp.path().to_path_buf());

        assert!(!manager.jar_exists());
    }

    #[test]
    fn test_quarkus_app_dir() {
        let temp = TempDir::new().unwrap();
        let manager =
            JarManager::with_paths(PathBuf::from("/fake/submodule"), temp.path().to_path_buf());

        let dir = manager.quarkus_app_dir();
        assert!(dir.to_string_lossy().contains("quarkus-app"));
    }
}
