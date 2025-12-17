# solverforge-service

JVM lifecycle management for SolverForge's embedded solver service.

## Overview

This crate provides automatic management of the Timefold solver service JVM process:

- **JAR Management** - Downloads and caches the solver service JAR from Maven
- **Embedded Service** - Spawns and manages the JVM process lifecycle
- **Configuration** - Port, Java options, timeouts, and paths

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
solverforge-service = "0.1"
```

### Start Embedded Service

```rust
use solverforge_service::{EmbeddedService, ServiceConfig};

// Start with default configuration
let service = EmbeddedService::start(ServiceConfig::default())?;

// Get the solver service client
let solver_service = service.solver_service();

// Service URL for manual use
println!("Service running at: {}", service.url());

// Service is automatically stopped on drop
```

### Custom Configuration

```rust
use solverforge_service::ServiceConfig;
use std::time::Duration;

let config = ServiceConfig {
    port: 8080,                              // 0 = auto-assign
    startup_timeout: Duration::from_secs(60),
    java_home: Some("/path/to/java".into()),
    java_opts: vec!["-Xmx2g".into()],
    ..Default::default()
};

let service = EmbeddedService::start(config)?;
```

### Service Lifecycle

```rust
let mut service = EmbeddedService::start(ServiceConfig::default())?;

// Check if running
if service.is_running() {
    println!("Port: {}", service.port());
}

// Explicit stop (also called on drop)
service.stop()?;
```

## JAR Source

The solver service JAR is:
1. Downloaded from Maven Central on first use
2. Cached in `~/.cache/solverforge/`
3. Reused for subsequent runs

## Requirements

- **Java 24+** - Must be installed and in PATH (or set `java_home`)
- **Network** - Required for initial JAR download

## Documentation

- [API Reference](https://docs.solverforge.org/solverforge-service)
- [User Guide](https://solverforge.org/docs)

## License

Apache-2.0
