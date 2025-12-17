//! Integration tests for solverforge-service

use solverforge_service::{EmbeddedService, ServiceConfig, SolverService};
use std::path::PathBuf;
use std::time::Duration;

fn java_home() -> String {
    std::env::var("JAVA_HOME").unwrap_or_else(|_| "/usr/lib64/jvm/java-24-openjdk-24".to_string())
}

#[test]
fn test_embedded_service_lifecycle() {
    let config = ServiceConfig::new()
        .with_startup_timeout(Duration::from_secs(120))
        .with_java_home(PathBuf::from(java_home()));

    let mut service = EmbeddedService::start(config).expect("Failed to start service");

    assert!(service.is_running());
    assert!(service.port() > 0);
    assert!(service.url().starts_with("http://localhost:"));

    let solver_service = service.solver_service();
    assert!(solver_service.is_available());

    service.stop().expect("Failed to stop service");
    assert!(!service.is_running());
}

#[test]
fn test_service_auto_port_selection() {
    let config1 = ServiceConfig::new()
        .with_startup_timeout(Duration::from_secs(120))
        .with_java_home(PathBuf::from(java_home()));

    let service1 = EmbeddedService::start(config1).expect("Failed to start service 1");
    let port1 = service1.port();

    let config2 = ServiceConfig::new()
        .with_startup_timeout(Duration::from_secs(120))
        .with_java_home(PathBuf::from(java_home()));

    let service2 = EmbeddedService::start(config2).expect("Failed to start service 2");
    let port2 = service2.port();

    assert_ne!(port1, port2, "Services should use different ports");

    // Services stop on drop
}

#[test]
fn test_service_with_fixed_port() {
    let config = ServiceConfig::new()
        .with_port(18080)
        .with_startup_timeout(Duration::from_secs(120))
        .with_java_home(PathBuf::from(java_home()));

    let service = EmbeddedService::start(config).expect("Failed to start service");

    assert_eq!(service.port(), 18080);
}
