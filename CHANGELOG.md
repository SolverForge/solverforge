# Changelog

All notable changes to SolverForge will be documented in this file.

## [0.2.5](///compare/v0.2.4...v0.2.5) (2025-12-17)


### Features

* **python:** add ConstraintVerifier testing framework e3f6b6e
* **python:** add flatten_last and complement stream operations 7a1465e
* **python:** add HardSoftDecimalScore and HardMediumSoftDecimalScore c83fb61
* **python:** add LoadBalance result type with unfairness() method cbc74b1
* **python:** add penalize_decimal/reward_decimal to all constraint streams bbd4bd4
* **python:** add QuadConstraintStream for 4-element constraint streams c1ea530
* **python:** add SolverJob and async solve_and_listen 399bed6
* **python:** add Timefold-compatible SolverManager and SolutionManager 7f87d89
* **python:** add Timefold-compatible submodule structure 2b00af0
* **python:** enhance lambda analyzer and constraint streams bf49d79
* **service:** add embedded service with auto-start and solution analysis 279c059


### Maintenance

* **release:** bump version to 0.2.5 010840b


### Maintenance

* **release:** bump version to 0.2.5 010840b

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
