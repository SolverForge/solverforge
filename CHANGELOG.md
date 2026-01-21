# Changelog

All notable changes to this project will be documented in this file. See [commit-and-tag-version](https://github.com/absolute-version/commit-and-tag-version) for commit guidelines.

## [0.5.1](///compare/v0.5.0...v0.5.1) (2026-01-16)


### Bug Fixes

* remove filter_with_solution() - use shadow variables on entities instead 431e503

## [0.5.1](///compare/v0.5.0...v0.5.1) (2026-01-16)


### Bug Fixes

* remove filter_with_solution() - use shadow variables on entities instead 431e503

## [0.5.0](///compare/v0.4.0...v0.5.0) (2026-01-15)


### Features

* **api:** add Solvable and Analyzable traits for public solver API 0578ebb
* **api:** re-export run_solver from umbrella crate cf8eb0a
* **config:** add SolverConfig::load() without fallback f58cc94
* **config:** add SolverConfig::time_limit() convenience method 1bf0e41
* **console:** add auto-init for tracing subscriber d9a29c2
* **console:** add SolverForge banner on init 47e276a
* **console:** add verbose step logging at TRACE level 2755637
* **core:** add ListVariableSolution trait for list-based planning 9e94edb
* **deploy:** fix CI 5fc03a7
* **lib:** export stats module and types 97cf39d
* **macros:** add BasicVariableConfig struct and parser b0a811b
* **macros:** add shadow variable attribute parsing cadf410
* **macros:** generate entity_count and list operation methods 5a9061e
* **macros:** generate helper methods for basic variables 1b95593
* **macros:** generate solve() method for basic variable problems 8810993
* **macros:** implement SolvableSolution trait in planning_solution macro 68ee735
* **macros:** parse constraints attribute and embed path in solve() a3c085e
* **manager:** add with_phase_factory() to SolverManagerBuilder d351daa
* **scope:** add PhaseStats to PhaseScope f051c24
* **scope:** add SolverStats to SolverScope faf6c85
* **scoring:** add into_working_solution to TypedScoreDirector 84e580d
* **scoring:** add shadow-aware after_variable_changed and do_change methods 3487d49
* **scoring:** add ShadowVariableSupport and ShadowAwareScoreDirector 239573c
* **scoring:** add ShadowVariableSupport::update_all_shadows() with default impl 520e142
* **scoring:** add solution-aware filter traits 45dc938
* **scoring:** add SolvableSolution trait for fluent API 74fbaf6
* **scoring:** add typed entity_counter to TypedScoreDirector 2c1155f
* **scoring:** add TypedScoreDirector::take_solution() for solution extraction c9c01d0
* **scoring:** make UniConstraintStream use solution-aware filters db71909
* **scoring:** solution-aware filter traits (BREAKING) 4c6ce03
* **solver:** add BasicConstructionPhaseBuilder for basic variables 59689ea
* **solver:** add BasicLocalSearchPhaseBuilder for basic variables 625909b
* **solver:** add create_solver() and solve_with_director() 258280c
* **solver:** add KOptPhaseBuilder fluent API for k-opt local search 9948014
* **solver:** add KOptPhaseBuilder for tour optimization 3876d17
* **solver:** add ListChangeMoveSelector for element relocation eec2cf3
* **solver:** add ListConstructionPhaseBuilder with change notification c44d3d5
* **solver:** add run_solver for basic variable problems f4a8084
* **solver:** add SolverEvent and solve_with_events for real-time feedback 978c819
* **solver:** add SolverManager::builder() static method b8ad45f
* **solver:** add termination flag to run_solver_with_events de55f5f
* **solver:** add with_phase_factory, with_config, Result-returning build f205b6f
* **solver:** add zero-erasure fluent phase functions (construction, 2-opt) e1704fe
* **solver:** export BasicConstructionPhaseBuilder and BasicLocalSearchPhaseBuilder aa36fbb
* **solver:** export ListConstructionPhaseBuilder fffc8d6
* **solverforge:** add verbose-logging feature for debug output ad5e2fc
* **stats:** add zero-erasure SolverStats and PhaseStats 901753a
* **termination:** export all termination types from fluent API aea6778
* **termination:** export MoveCountTermination and ScoreCalculationCountTermination 17a6d39
* **termination:** restore MoveCountTermination using stats API a1ed57a
* **termination:** restore ScoreCalculationCountTermination using stats API ebdd0dc


### Bug Fixes

* **benchmark:** wire SolveResult through Solvable trait for zero-erasure stats 3b98e75
* **clippy:** resolve collapsible_if, unnecessary_map_or, and boxed_local warnings 85c8d63
* **console:** flush stdout after banner print 5b9a036
* **doctest:** correct import paths for KOptPhaseBuilder and ListConstructionPhaseBuilder ae8b843
* **export:** add derive macros at root level and fix __internal imports 2fc6743
* **k-opt:** use popped position in NearbyCutIterator::backtrack 2ab6ed0
* **list-change:** filter out no-op intra-list moves cf6b1ee
* **localsearch:** add explicit type annotations in forager tests 97d2b8f
* **macros:** move console feature gate to library 4b45e65
* **macros:** update internal type references for __internal module 2890868
* **macros:** use ScoreDirector API correctly for solve() 743cb7e
* mute type_complexity clippy warning on with_time_limit_or a891b18
* remove debug eprintln from EntitySelector fe2e893
* replace .clone() with copy for Copy types (clippy) ff8c5ff
* **scoring:** set score on solution in into_working_solution d5ecb3f
* **scoring:** update tests for solution-aware filter signatures aeca6f9
* **solver:** delete SolverBuilder::solve that violated API contract 606260c
* **solver:** restore EntitySelector in KOptPhaseBuilder 8643928
* **solver:** update cached_score after rejected move undo ed6f192
* **solver:** use incremental protocol for score updates d871e30
* **termination:** wire time limit through to SolverScope 7423443
* **test:** handle Result return type from SolverManagerBuilder::build() 9aa12a5
* **vnd:** implement Debug manually to avoid S: Debug bound bd2fceb

## 0.4.0 (2026-01-04)


### ⚠ BREAKING CHANGES

* import SolverForge from private repo

### Features

* add .github 8361f7d
* add vehicle-routing example 2fff9b3
* default to Real Roads mode in vehicle-routing UI 4870311
* implement three-tier road network caching 55d1eac
* import SolverForge from private repo 3ed55f9
* **routing:** initialize road routing when creating job 907b33b
* **solver:** add k-opt reconnection patterns 1171e27
* **solver:** add KOptMove for k-opt tour optimization 9572ce4
* **solver:** add KOptMoveSelector for k-opt move generation cf83881
* **solver:** add NearbyKOptMoveSelector for efficient k-opt e762cff
* **solverforge:** export k-opt types from umbrella crate 9e5d385


### Bug Fixes

* add diagnostic logging and read_timeout to Overpass client b226b37
* eliminate all clippy and dead_code warnings b6bc7d3
* initialize tracing subscriber in vehicle-routing a9f9ad0
* **publish:** add version specs for workspace crate publishing c1c276f
* switch reqwest to rustls-tls for async DNS resolution 8c77564
* **test:** increase timing margins in diminished_returns test affd93a
* use tokio::fs for async filesystem operations 2979e2b
* **vehicle-routing:** add fallback for missing route geometry bb2efe1
* **vehicle-routing:** disable solve button immediately on click d832335
* **vehicle-routing:** disable solve button immediately on click dd1adac
* **vehicle-routing:** fix encode_routes doctest to set up route_geometries ac03030
* **vehicle-routing:** parse geometry API response correctly 32024f0
* **vehicle-routing:** skip construction when visits already assigned 1bad62f

## 0.4.0 (2026-01-04)


### ⚠ BREAKING CHANGES

* import SolverForge from private repo

### Features

* add .github 8361f7d
* add vehicle-routing example 2fff9b3
* default to Real Roads mode in vehicle-routing UI 4870311
* implement three-tier road network caching 55d1eac
* import SolverForge from private repo 3ed55f9
* **routing:** initialize road routing when creating job 907b33b
* **solver:** add k-opt reconnection patterns 1171e27
* **solver:** add KOptMove for k-opt tour optimization 9572ce4
* **solver:** add KOptMoveSelector for k-opt move generation cf83881
* **solver:** add NearbyKOptMoveSelector for efficient k-opt e762cff
* **solverforge:** export k-opt types from umbrella crate 9e5d385


### Bug Fixes

* add diagnostic logging and read_timeout to Overpass client b226b37
* eliminate all clippy and dead_code warnings b6bc7d3
* initialize tracing subscriber in vehicle-routing a9f9ad0
* switch reqwest to rustls-tls for async DNS resolution 8c77564
* **test:** increase timing margins in diminished_returns test affd93a
* use tokio::fs for async filesystem operations 2979e2b
* **vehicle-routing:** add fallback for missing route geometry bb2efe1
* **vehicle-routing:** disable solve button immediately on click d832335
* **vehicle-routing:** disable solve button immediately on click dd1adac
* **vehicle-routing:** fix encode_routes doctest to set up route_geometries ac03030
* **vehicle-routing:** parse geometry API response correctly 32024f0
* **vehicle-routing:** skip construction when visits already assigned 1bad62f

## 0.4.0 (2026-01-04)


### ⚠ BREAKING CHANGES

* import SolverForge from private repo

### Features

* add .github 8361f7d
* add vehicle-routing example 2fff9b3
* default to Real Roads mode in vehicle-routing UI 4870311
* implement three-tier road network caching 55d1eac
* import SolverForge from private repo 3ed55f9
* **routing:** initialize road routing when creating job 907b33b
* **solver:** add k-opt reconnection patterns 1171e27
* **solver:** add KOptMove for k-opt tour optimization 9572ce4
* **solver:** add KOptMoveSelector for k-opt move generation cf83881
* **solver:** add NearbyKOptMoveSelector for efficient k-opt e762cff
* **solverforge:** export k-opt types from umbrella crate 9e5d385


### Bug Fixes

* add diagnostic logging and read_timeout to Overpass client b226b37
* eliminate all clippy and dead_code warnings b6bc7d3
* initialize tracing subscriber in vehicle-routing a9f9ad0
* switch reqwest to rustls-tls for async DNS resolution 8c77564
* use tokio::fs for async filesystem operations 2979e2b
* **vehicle-routing:** add fallback for missing route geometry bb2efe1
* **vehicle-routing:** disable solve button immediately on click d832335
* **vehicle-routing:** disable solve button immediately on click dd1adac
* **vehicle-routing:** fix encode_routes doctest to set up route_geometries ac03030
* **vehicle-routing:** parse geometry API response correctly 32024f0
* **vehicle-routing:** skip construction when visits already assigned 1bad62f
