# Changelog

All notable changes to SolverForge will be documented in this file.

## [0.3.0](///compare/v0.2.6...v0.3.0) (2026-01-02)


### Features

* **constraints:** add NamedExpression for expression-based stream API 5e5dd0c
* **core,derive:** add entity difficulty and value strength comparators fbcdf7b
* **core:** add ifExistsOther and including-unassigned stream components 12178a5
* **core:** add ProblemChange API for real-time planning 71b871b
* **core:** add shadow variable annotations for list-based planning 00718e8
* **core:** add toSortedSet, toMap, toSortedMap collectors 13bddb5
* **core:** add VariableListener API for custom shadow variables 6e2b5de
* **derive:** add cascading_update_shadow support da4b23e
* **derive:** add shadow variable attribute support 61a0bb5
* **derive:** implement #[planning_list_variable] attribute parsing 95cb059
* **host:** implement hinsert and hremove for list planning variables a661842
* **lambda:** extend accumulation pattern for early returns and post-loop adds 320aa81
* **python:** add allows_unassigned_values to PyPlanningListVariable a0fc886
* **python:** add class constant inlining for lambda analysis af87664
* **python:** add extract assignment-based pattern for methods that assign to self.field instead of returning 9ec6189
* **python:** add method introspection helpers for lambda analysis e5d1b6a
* **python:** add PentaConstraintStream and fix join class tracking bc2eadc
* **python:** add sequential expression pattern and datetime serialization 1e12702
* **python:** add SolverManager.solve_and_listen() for async solving with callbacks b0ce3d9
* **python:** add substitute_param for method inlining eadf5f5
* **python:** add update methods for CascadingUpdateShadowVariable rields 3671e1c
* **python:** complete Timefold API parity for constraint streams f2d79ae
* **python:** generate WASM module in SolverFactory 1419a20
* **python:** inline method calls in AST analysis cd30864
* **python:** inline method calls in bytecode analysis 152fc8f
* **python:** pass constraint predicates to WASM module 6368beb
* **python:** register domain classes during stream creation 70da731
* **python:** require Python 3.13+ and add conditionals module e3c5dda
* **shadow:** cascading update expression compilation [WIP] 90a9eb9
* **solver:** add SolverManager for multi-problem solving 53141a1
* **test:** add systemic testing framework 286bb7a
* **wasm:** add float arithmetic operations (FloatAdd/Sub/Mul/Div) eaf34ef
* **wasm:** add math function support for constraint inlining d124a3f


### Bug Fixes

* **ast:** handle null comparisons in Eq/NotEq like Is/IsNot b262d72
* **core:** use correct WASM export names and update README 407fffc
* **derive:** correctly map i32 to Int and i64 to Long in WASM 274fdea
* **derive:** shadow variables specify entity type directly 7298724
* **inference:** add missing expression types to infer_expression_type f089c73
* **model:** add CascadingUpdateShadowVariable to setter generation 4f39cdc
* **model:** generate setter functions for shadow variables f75c5b8
* **model:** handle InverseRelationShadowVariable separately - resolves to owner class 063a057
* **model:** resolve element types for shadow variable annotations 103baac
* **python:** Div operator always produces FloatDiv ee5dd7a
* **python:** update deprecated PyO3 APIs to current versions 6dc4d97
* **service:** update integration tests for new annotation API 1fa5fe9
* **test:** rewrite employee_scheduling predicates with proper i64 ops ae7d406
* **types:** make Null transparent in type promotion a38fb74
* **wasm:** add IsNull64/IsNotNull64 for i64 null checks 547a594
* **wasm:** use i64 operations for datetime field arithmetic f295e05


### Refactoring

* **ast:** add ExpectedType for contextual type inference 0a65013
* **ast:** add WasmFieldType and split ast_convert into submodule 2c5e7cb
* **ast:** implement single-pass type inference 1b4db7c
* **core:** consolidate PlanningAnnotation in domain module adab859
* extract AST conversion helpers to ast_convert.rs 104527c
* **lambda_analyzer:** convert to module directory d8a301a
* **lambda_analyzer:** convert to module directory structure 91c2fa9
* **lambda_analyzer:** extract registry module f1d5a2d
* **lambda_analyzer:** split into submodules 7f799fe
* move substitute_param to solverforge-core Expression impl bf288b4
* **python:** extract conditional patterns to conditionals.rs 28362c9
* **python:** extract lambda parsing to lambda_parsing.rs a9852dd
* **python:** extract loop patterns to loops.rs module 2e20ea9
* **python:** extract method analysis to method_analysis.rs cafc345
* **python:** extract sequential pattern to sequential.rs 53c8244
* **python:** make class_hint required throughout lambda analysis f968326
* **python:** move convert_ast_to_expression to ast_convert.rs 59db08b
* **python:** remove callable from LambdaInfo, make expression mandatory 8e1afab
* remove bytecode fallback from lambda analyzer 8009eb0
* unify annotations field to match Java service format 4008157
* **wasm:** remove legacy Comparison API, complete i64 pipeline 1824d9d
* **wasm:** split generator.rs and expression.rs into modules 24689a3


### Documentation

* expand README documentation with comprehensive API examples 8b7a773
* export new API types from lib.rs 88d2c04


### Maintenance

* remove unused clear_class_registry function e0602a8
* update solverforge-wasm-service submodule 5ac1213
* update solverforge-wasm-service with trig host functions 2826b36
* update solverforge-wasm-submodule b93296b
* update submodules fa157d9
* update submodules dde09c7


### Tests

* remove test_accumulation_with_early_return 47733c9
* remove tests using py.run() lambdas, fix lambda source parsing f51dfb6

## [0.2.6](///compare/v0.2.5...v0.2.6) (2025-12-18)


### Features

* **python:** add compose/conditionally collectors, fix Python 3.12+ lambda analysis 6413db0


### Maintenance

* update solverforge-wasm-service to 0.2.6 f876355

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
