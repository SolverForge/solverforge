# Changelog

All notable changes to SolverForge will be documented in this file.

## [0.2.4](///compare/v0.2.3...v0.2.4) (2025-12-17)


### Features

* **python:** add shadow variable annotations for list planning 7eeb6b6

## [0.2.3](///compare/v0.2.2...v0.2.3) (2025-12-17)


### Features

* **python:** export solver runtime, constraints, joiners, collectors 7f2301b

## [0.2.2](///compare/v0.2.1...v0.2.2) (2025-12-17)


### Bug Fixes

* use rustls-tls instead of native-tls for manylinux compatibility fcedea3

## [0.2.1] - 2024-12-17

### Added
- `solverforge` umbrella crate for simplified installation (`cargo add solverforge`)
- Embedded service feature (default) - auto-manages Java solver process
- PyPI publishing workflow with trusted publishing
- crates.io publishing workflow

### Fixed
- macOS arm64 Python linking issues with PyO3 abi3
- CI workflows for cross-platform builds
- Integration test JAVA_HOME handling

### Changed
- Rebranded from timefold-wasm-service to solverforge-wasm-service
- Updated PyO3 to 0.27

## [0.2.0] - 2024-12-15

### Added
- Initial public release
- Core constraint solver library (solverforge-core)
- Derive macros for domain types (solverforge-derive)
- JVM lifecycle management (solverforge-service)
- Python bindings with Timefold-compatible API
- Constraint streams: forEach, filter, join, groupBy, penalize, reward
- Score types: Simple, HardSoft, HardMediumSoft, Bendable
- WASM module generation with proper memory alignment
- End-to-end solving via HTTP with embedded Java service
