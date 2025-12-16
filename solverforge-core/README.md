# solverforge-core

Language-agnostic core library for SolverForge, a constraint solver that bridges Rust to the Timefold JVM via WASM modules and HTTP.

## Overview

This crate provides the foundation for SolverForge's constraint solving capabilities:

- **Value types** - Language-agnostic representations (`Value`, `ObjectHandle`, `FunctionHandle`)
- **Score types** - `SimpleScore`, `HardSoftScore`, `HardMediumSoftScore`, and decimal variants
- **Domain modeling** - Planning annotations, entities, and solutions
- **Constraint streams** - Functional constraint definition API
- **WASM generation** - Compile constraints to WebAssembly modules
- **Solver service** - HTTP client for the Timefold solver backend

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
solverforge-core = "0.1"
```

### Basic Example

```rust
use solverforge_core::{Value, HardSoftScore, SolverFactory};

// Create a solver factory with configuration
let factory = SolverFactory::builder()
    .with_service_url("http://localhost:8080")
    .build();

// Create domain values
let score = HardSoftScore::of(0, -5);
println!("Score: {}", score);
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Language Bindings (Python, JS, Go)                          │
├─────────────────────────────────────────────────────────────┤
│ solverforge-core (this crate)                               │
│  ├─ Domain modeling                                         │
│  ├─ Constraint definition                                   │
│  ├─ WASM module generation                                  │
│  └─ HTTP solver client                                      │
├─────────────────────────────────────────────────────────────┤
│ Solver Service (Timefold JVM)                               │
└─────────────────────────────────────────────────────────────┘
```

## Documentation

- [API Reference](https://docs.solverforge.org/solverforge-core)
- [User Guide](https://solverforge.org/docs)
- [GitHub Repository](https://github.com/solverforge/solverforge)

## License

Apache-2.0
