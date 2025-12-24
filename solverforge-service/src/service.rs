use crate::config::ServiceConfig;
use crate::error::{ServiceError, ServiceResult};
use crate::jar::JarManager;
use crate::util::{find_available_port, find_java};
use log::{debug, error, info, warn};
use solverforge_core::HttpSolverService;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct EmbeddedService {
    process: Option<Child>,
    port: u16,
    shutdown_flag: Arc<AtomicBool>,
}

impl EmbeddedService {
    pub fn start(config: ServiceConfig) -> ServiceResult<Self> {
        Self::start_with_retry(config, true)
    }

    fn start_with_retry(config: ServiceConfig, allow_retry: bool) -> ServiceResult<Self> {
        let port = if config.port == 0 {
            find_available_port()?
        } else {
            config.port
        };

        let java = find_java(config.java_home.as_deref())?;

        // Derive JAVA_HOME from the java path for Maven
        let java_home = java
            .parent()
            .and_then(|bin| bin.parent())
            .map(|home| home.to_path_buf());

        let jar_manager = if let Some(ref submodule_dir) = config.submodule_dir {
            let cache_dir = config
                .cache_dir
                .clone()
                .unwrap_or_else(crate::util::get_cache_dir);
            JarManager::with_paths(submodule_dir.clone(), cache_dir)
                .with_java_home(java_home.as_deref())
        } else {
            JarManager::new().with_java_home(java_home.as_deref())
        };

        let jar_path = jar_manager.ensure_jar()?;
        let working_dir = jar_manager.cache_dir();

        info!("Starting embedded solver service on port {}", port);
        debug!("Using JAR: {}", jar_path.display());
        debug!("Using Java: {}", java.display());

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_flag_clone = shutdown_flag.clone();

        let mut cmd = Command::new(&java);

        // Set JAVA_HOME for the subprocess to ensure it uses the correct Java
        if let Some(ref home) = java_home {
            cmd.env("JAVA_HOME", home);
        }

        // JVM options must come before -jar
        cmd.arg(format!("-Dquarkus.http.port={}", port));

        for opt in &config.java_opts {
            cmd.arg(opt);
        }

        cmd.arg("-jar")
            .arg(&jar_path)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut process = cmd.spawn().map_err(|e| {
            ServiceError::StartFailed(format!(
                "Failed to start Java process: {}. Is Java installed?",
                e
            ))
        })?;

        // Capture stdout (solver metrics)
        if let Some(stdout) = process.stdout.take() {
            let shutdown = shutdown_flag_clone.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(line) = line {
                        if line.contains("ERROR") {
                            error!("[solver] {}", line);
                        } else if line.contains("WARN") {
                            warn!("[solver] {}", line);
                        } else if line.contains("INFO") {
                            info!("[solver] {}", line);
                        } else {
                            debug!("[solver] {}", line);
                        }
                    }
                }
            });
        }

        // Capture stderr (JVM warnings, errors)
        if let Some(stderr) = process.stderr.take() {
            let shutdown = shutdown_flag_clone;
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(line) = line {
                        if line.contains("ERROR") {
                            error!("[solver-service] {}", line);
                        } else if line.contains("WARN") {
                            warn!("[solver-service] {}", line);
                        } else if line.contains("INFO") {
                            info!("[solver-service] {}", line);
                        } else {
                            debug!("[solver-service] {}", line);
                        }
                    }
                }
            });
        }

        let mut service = EmbeddedService {
            process: Some(process),
            port,
            shutdown_flag,
        };

        let health_url = format!("http://localhost:{}/health/ready", port);

        // Wait for service to be ready, checking if process crashes
        let start = std::time::Instant::now();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| ServiceError::Http(e.to_string()))?;

        // Give process a moment to start before checking
        std::thread::sleep(Duration::from_millis(100));

        info!("Waiting for service health check at {}", health_url);

        loop {
            // Check if process crashed (only after initial startup window)
            if start.elapsed() > Duration::from_millis(500) {
                let running = service.is_running();
                debug!("Process running check: {}", running);
                if !running {
                    // Get exit status for diagnostics
                    if let Some(ref mut proc) = service.process {
                        if let Ok(Some(status)) = proc.try_wait() {
                            error!("Java process exited with status: {:?}", status);
                        }
                    }
                    if allow_retry {
                        warn!("Java process crashed during startup, deleting cached JAR and retrying...");
                        if let Err(e) = std::fs::remove_file(&jar_path) {
                            warn!("Failed to delete cached JAR: {}", e);
                        }
                        return Self::start_with_retry(config, false);
                    }
                    return Err(ServiceError::StartFailed(
                        "Java process crashed during startup. Check logs or try: rm ~/.cache/solverforge/*.jar".to_string(),
                    ));
                }
            }

            if start.elapsed() > config.startup_timeout {
                return Err(ServiceError::Unhealthy(format!(
                    "Service did not become ready within {:?}",
                    config.startup_timeout
                )));
            }

            match client.get(&health_url).send() {
                Ok(response) if response.status().is_success() => {
                    debug!("Service is ready after {:?}", start.elapsed());
                    break;
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

        info!("Solver service is ready on port {}", port);

        Ok(service)
    }

    pub fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) => false,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub fn stop(&mut self) -> ServiceResult<()> {
        self.shutdown_flag.store(true, Ordering::Relaxed);

        if let Some(mut process) = self.process.take() {
            info!("Stopping embedded solver service");

            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(process.id() as i32, libc::SIGTERM);
                }
            }

            #[cfg(not(unix))]
            {
                process.kill().ok();
            }

            thread::sleep(Duration::from_secs(2));

            if let Ok(None) = process.try_wait() {
                warn!("Service didn't stop gracefully, forcing termination");
                process.kill().ok();
            }

            process.wait().ok();
            info!("Solver service stopped");
        }

        Ok(())
    }

    pub fn solver_service(&self) -> HttpSolverService {
        HttpSolverService::new(self.url())
    }
}

impl Drop for EmbeddedService {
    fn drop(&mut self) {
        if self.process.is_some() {
            if let Err(e) = self.stop() {
                error!("Failed to stop embedded service on drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_generation() {
        let service = EmbeddedService {
            process: None,
            port: 8080,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        };
        assert_eq!(service.url(), "http://localhost:8080");
    }

    #[test]
    fn test_port_getter() {
        let service = EmbeddedService {
            process: None,
            port: 9999,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        };
        assert_eq!(service.port(), 9999);
    }

    #[test]
    fn test_is_running_no_process() {
        let mut service = EmbeddedService {
            process: None,
            port: 8080,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        };
        assert!(!service.is_running());
    }
}
