# Changelog

All notable changes to this project will be documented in this file. See [commit-and-tag-version](https://github.com/absolute-version/commit-and-tag-version) for commit guidelines.

## [0.11.0](///compare/v0.10.0...v0.11.0) (2026-05-05)


### Features

* **scoring:** make projected rows clone-free aa50a3f

## [0.10.0](///compare/v0.9.2...v0.10.0) (2026-05-02)


### Features

* **scoring:** allow projected row self joins 56ae571
* **scoring:** expose constraint metadata through directors a53d2b1
* **solver:** add conflict-directed scalar repair selector b746404
* **solver:** add grouped scalar construction 72814dd
* **solver:** gate cartesian composites on hard improvement f8f95e6
* **solver:** gate grouped scalar moves on hard improvement 89c4513
* **solver:** label selector telemetry from move metadata 140dee5
* **solver:** make simulated annealing score-level aware 86d41e3
* **solver:** support mandatory nullable construction assignment 0292a75
* **solver:** support round-robin union selectors 0149b28


### Bug Fixes

* **ci:** satisfy clippy termination config tests a8942ca
* **core:** keep score level access source compatible 76029c2
* **facade:** export source-aware stream helpers 933fc7b
* **macros:** simplify solution flag parsing 25b4c6f
* **scoring:** deduplicate metadata by full constraint ref 6127092
* **scoring:** preserve projected join coordinate ordering 81fa377
* **scoring:** reuse projected join row slots 16d46d8
* **solver:** apply grouped construction caps after normalization c2d0c4d
* **solver:** compare all hard levels for repair gating 3df8b14
* **solver:** enforce hard repair improvement in vnd 8fee571
* **solver:** expose grouped construction repair progress b570d78
* **solver:** honor scalar semantics in grouped construction ad98d6c
* **solver:** preserve effective fallback time limits 0720d11
* **solver:** report selector telemetry without phantom fallbacks 08e42a6
* **solver:** validate conflict repair constraints 1f3b2c6

## [0.10.0](///compare/v0.9.2...v0.10.0) (2026-04-28)


### Features

* **scoring:** allow projected row self joins 56ae571
* **solver:** make simulated annealing score-level aware 86d41e3
* **solver:** support round-robin union selectors 0149b28


### Bug Fixes

* **core:** keep score level access source compatible 76029c2
* **facade:** export source-aware stream helpers 933fc7b
* **scoring:** preserve projected join coordinate ordering 81fa377
* **scoring:** reuse projected join row slots 16d46d8
* **solver:** preserve effective fallback time limits 0720d11
* **solver:** report selector telemetry without phantom fallbacks 08e42a6

## [0.9.1](///compare/v0.9.0...v0.9.1) (2026-04-26)


### Features

* **console:** show score on local-search phase start 2b048ff

## [0.9.0](///compare/v0.8.13...v0.9.0) (2026-04-24)


### ⚠ BREAKING CHANGES

* **macros:** generated retained-runtime domains now require a planning_model! manifest for model-owned runtime metadata instead of relying on derive expansion order or proc-macro global registries.
* **solver:** direct solver APIs for scalar getter/setter callbacks, ScalarVariableContext::new, scalar move constructors, scalar selector constructors, and RuinMoveSelector now require variable-index-aware access metadata.

* **solver:** index scalar runtime and selector legality 037cf66


### Features

* **config:** expose canonical selector and forager schemas eb3334b
* **macros:** add planning_model manifest support 9e4211b
* **selector:** add scalar selector runtime and canonical pillar legality 887de6c
* **solver:** resolve scalar construction from live model hooks 47fa178
* **solver:** scope tabu signatures and normalize tabu policy 04781e4


### Bug Fixes

* **acceptor:** match undo tabu against candidate moves c226b53
* **localsearch:** thread move signatures through acceptor hooks ef08864
* **move:** derive sublist inverse identities from segment layout b85608f
* **scalar:** align nearby metadata and ruin-recreate legality a162e11
* **selector:** bound cartesian preview semantics b521bb8
* **selector:** enforce cartesian composite legality 52d13cb
* **selector:** honor seeded scalar ruin-recreate streams c416fba
* **selector:** own cartesian composites and validate tabu tenures 39c2e94
* **selector:** prune scalar nearby candidates before limiting 3b85bfc
* **tabu:** canonicalize self-inverse move identities af00d5e

## [0.8.13](///compare/v0.8.12...v0.8.13) (2026-04-22)


### Bug Fixes

* **solver:** report standard solve candidate counts ea81296

## [0.8.12](///compare/v0.8.11...v0.8.12) (2026-04-21)


### Features

* **construction:** emit engine phase telemetry 1f5cff7
* **solver:** add revision-aware optional construction semantics 0d0d9de


### Bug Fixes

* **construction:** honor optional none in first-fit 54ad31f
* **localsearch:** retain best accepted moves without early exit e66b494
* **runtime:** route pure scalar construction to descriptor path 77c5f42

## [0.8.10](///compare/v0.8.9...v0.8.10) (2026-04-20)


### Bug Fixes

* **macros:** harden generated list helper bindings d49900e
* **macros:** restore explicit list helper surface ec789e0
* **shadows:** canonicalize descriptor-aware shadow lifecycle 79cc69c
* **solver:** avoid duplicate construction best events 6c6b466
* **solver:** canonicalize partitioned search lifecycle c6bf033
* **solver:** publish tied construction solutions 527d287

## [0.8.9](///compare/v0.8.8...v0.8.9) (2026-04-20)

## [0.8.8](///compare/v0.8.7...v0.8.8) (2026-04-18)

## [0.8.7](///compare/v0.8.6...v0.8.7) (2026-04-17)


### Bug Fixes

* **scoring:** correct stateless keyed cross-bi analysis and bump workspace to 0.8.7 da9d48d

## [0.8.6](///compare/v0.8.5...v0.8.6) (2026-04-13)


### Bug Fixes

* **solver:** correct list ruin undo reinsertion tracking 5535dad

## [0.8.5](///compare/v0.8.4...v0.8.5) (2026-04-13)


### Bug Fixes

* **macros:** restore list runtime config logging compilation 7d18eb3

## [0.8.5](///compare/v0.8.4...v0.8.5) (2026-04-13)


### Bug Fixes

* **macros:** restore list runtime config logging compilation 7d18eb3

## [0.8.5](///compare/v0.8.4...v0.8.5) (2026-04-13)


### Features

* **scoring:** add tracked existence streams for direct and flattened targets 308dae7
* **solver:** add solver-manager coverage for Clarke-Wright best-solution publication b85b6c7


### Bug Fixes

* **macros:** restore list runtime config logging compilation 7d18eb3

## [0.8.4](///compare/v0.8.3...v0.8.4) (2026-04-12)

## [0.8.3](///compare/v0.8.2...v0.8.3) (2026-04-12)


### Bug Fixes

* **runtime:** support embedded solver configs and deterministic stock search a12d4db

## [0.8.2](///compare/v0.8.1...v0.8.2) (2026-04-11)


### Features

* **console:** use emerald truecolor banner output 0f08921
* **macros:** support solution trait bounds on list variables 1463cca
* **solver:** make retained phases interruptible a34d02c


### Bug Fixes

* **construction:** retry interrupted placements after pause f24a6a0
* **cvrp:** derive debug for matrix distance meters b0e4e33
* **runtime:** serialize pause lifecycle publication 4e2e3f8

## [0.8.1](///compare/v0.8.0...v0.8.1) (2026-04-07)

## [0.8.0](///compare/v0.7.1...v0.8.0) (2026-04-06)


### ⚠ BREAKING CHANGES

* **solver:** Solvable::solve now takes SolverRuntime<Self> instead of the previous terminate and sender plumbing. SolverManager::solve now returns Result<(job_id, receiver), SolverManagerError>, and the retained lifecycle API introduces the public event, status, snapshot, and error surface required by the new contract.

### Features

* **solver:** implement retained lifecycle pause and resume 1d775a5

## [0.7.1](///compare/v0.7.0...v0.7.1) (2026-04-03)


### Bug Fixes

* support solution-level value range fallback 702277e

## 0.7.0 (2026-04-03)


### ⚠ BREAKING CHANGES

* import SolverForge from private repo

### Features

* add .github 22db95a
* add ListClarkeWrightPhase and remove nqueens example b8d6066
* add ListKOptPhase, solverforge-cvrp lib, and fix doctest signatures fd9714f
* add nearby list change/swap selectors and list move infrastructure 7cbf56d
* add NearbyKOpt config support and remove Debug bound from distance meter API 3f4afc8
* add vehicle-routing example 43dc6f2
* **api:** add Solvable and Analyzable traits for public solver API 91e9e8f
* **api:** re-export run_solver from umbrella crate 483e5c0
* **cli:** implement solverforge-cli crate with full scaffolding and codegen b33b659
* **cli:** salvage generic scaffold refresh for review 80e8c57
* config-driven solver construction 3badd17
* **config:** add SolverConfig::load() without fallback 655e84d
* **config:** add SolverConfig::time_limit() convenience method b3153c2
* **console:** add auto-init for tracing subscriber 1adf876
* **console:** add console output implementation 028c732
* **console:** add SolverForge banner on init 52e73f1
* **console:** add solverforge_dynamic=info tracing directive c0bbdc6
* **console:** add verbose step logging at TRACE level 51c2e1e
* **console:** create solverforge-console crate 310bea9
* **core:** add ListVariableSolution trait for list-based planning 6399a4d
* default to Real Roads mode in vehicle-routing UI 34c023f
* **deploy:** fix CI c5d1576
* **dynamic/jit:** enhance JIT compiler with compile_n and unified JitFn 382a8ba
* **dynamic:** add benchmark comparing O(1) HashMap vs O(n) linear filter lookup af03bd0
* **dynamic:** add benchmark test and document lazy iterator performance 0241969
* **dynamic:** add best solution callback to update shared snapshot ed38356
* **dynamic:** add build_bi_self_constraint factory for self-join constraints cdecf71
* **dynamic:** add build_cross_bi_constraint factory for cross-bi-entity constraints 7d0f88e
* **dynamic:** add build_flattened_bi_constraint factory for flattened bi-entity constraints c5cbc3b
* **dynamic:** add build_penta_self_constraint factory for penta-entity self-join constraints a27c54e
* **dynamic:** add build_quad_self_constraint factory for quad-entity self-join constraints 3503e1d
* **dynamic:** add build_tri_self_constraint factory for tri-entity self-join constraints 7ec360a
* **dynamic:** add build_uni_constraint factory for unary constraints fa15404
* **dynamic:** add cross-join type aliases for incremental constraints a998dbb
* **dynamic:** add DynamicEitherMove enum wrapping Change + Swap b6d282d
* **dynamic:** add DynamicMoveIterator for lazy move generation af7b7ad
* **dynamic:** add DynamicSwapMove for value swaps between entities e981316
* **dynamic:** add eval_entity_expr for single-entity expression evaluation 34835ab
* **dynamic:** add flattened constraint type aliases 76d0b57
* **dynamic:** add id_to_location mapping to DynamicSolution 615bd2c
* **dynamic:** add make_bi_filter for bi-entity filter closure creation 371cbad
* **dynamic:** add make_bi_weight for bi-entity weight computation 1447f2b
* **dynamic:** add make_cross_extractor_a and make_cross_extractor_b for cross-join constraints 6316d05
* **dynamic:** add make_cross_filter and make_cross_weight for cross-join constraints 148c346
* **dynamic:** add make_cross_key_a and make_cross_key_b for cross-join key extraction 9d2a784
* **dynamic:** add make_extractor function for entity slice extraction a5ab04c
* **dynamic:** add make_flatten, make_c_key_fn, and make_a_lookup for flattened bi-constraints ffa76a8
* **dynamic:** add make_flattened_filter and make_flattened_weight for flattened bi-constraints f21b5af
* **dynamic:** add make_key_extractor for join key extraction a1a8e68
* **dynamic:** add make_penta_filter and make_penta_weight for penta-entity operations 7c007a1
* **dynamic:** add make_quad_filter and make_quad_weight for quad-entity operations 4ae0297
* **dynamic:** add make_tri_filter and make_tri_weight for tri-entity operations e058e6e
* **dynamic:** add runtime warnings for unsupported key expression constructs b7ec13a
* **dynamic:** add tests for cross-class constraints with same-named fields 4011689
* **dynamic:** add type aliases for boxed constraint closures 7eb3fec
* **dynamic:** change DynBiWeight to accept solution reference and indices e9e7511
* **dynamic:** change DynCrossWeight to accept solution reference and indices c30f3d1
* **dynamic:** change DynTriWeight to accept solution reference and indices 4909d1a
* **dynamic:** document key expression limitations in cross-constraint closures bef58a4
* **dynamic:** document shuffled iteration design in MoveSelector 2c17c61
* **dynamic:** implement O(1) entity location lookup via id_to_location HashMap b6ed5b8
* **dynamic:** refactor generate_moves to return lazy iterator 8dae6a1
* **dynamic:** rewrite DynamicMoveSelector to generate change + swap moves 46940a0
* **dynamic:** verify solverforge-scoring dependency in Cargo.toml e3e8e6a
* generated domain accessors for constraint streams 634e43e
* **history:** add backward compatibility aliases to nqueens module c2c1120
* **history:** add generic entity test fixtures to solverforge-test a0c6811
* **history:** add minimal TestSolution fixtures to solverforge-test dbb4cd4
* **history:** add missing ScoreDirector import to task tests 070306d
* **history:** add N-Queens test fixtures to solverforge-test 4ae2275
* **history:** add solverforge-test crate skeleton 508f9bb
* **history:** add solverforge-test lib.rs with module structure 23cf479
* **history:** add solverforge-test to workspace members and dependencies faa3b45
* **history:** add Task scheduling test fixtures to solverforge-test 2aa846e
* implement three-tier road network caching dce11b3
* import SolverForge from private repo b36e7e5
* **jit:** add Cranelift JIT compiler for Expr trees 870e010
* **jit:** zero-fallback JIT, eliminate key extractor temp buffer, add Python .pyi stub 68022f3
* **lib:** export stats module and types c0f88c8
* **macros:** add BasicVariableConfig struct and parser 429dffe
* **macros:** add shadow variable attribute parsing bd8e17d
* **macros:** generate entity_count and list operation methods ad9378f
* **macros:** generate helper methods for basic variables 51fbae6
* **macros:** generate solve() method for basic variable problems 0624317
* **macros:** implement SolvableSolution trait in planning_solution macro 5dd8af5
* **macros:** parse constraints attribute and embed path in solve() ba8b49b
* **manager:** add with_phase_factory() to SolverManagerBuilder c41724a
* **py:** init console at module import for early banner display d64911f
* **py:** release GIL during native solve loop a061b5f
* **routing:** initialize road routing when creating job 48d0607
* ruin-and-recreate ListRuinMove + construction termination fix b36b5be
* **scope:** add PhaseStats to PhaseScope 1699874
* **scope:** add SolverStats to SolverScope aaba2bb
* **score:** add score macros module 1d0f2cb
* **scoring:** add descriptor_index parameter to incremental constraint methods c3fb6ea
* **scoring:** add descriptor_index to TypedScoreDirector public API aaa7bb5
* **scoring:** add into_working_solution to TypedScoreDirector 3d407a2
* **scoring:** add shadow-aware after_variable_changed and do_change methods d737893
* **scoring:** add ShadowVariableSupport and ShadowAwareScoreDirector c71384e
* **scoring:** add ShadowVariableSupport::update_all_shadows() with default impl ca70716
* **scoring:** add shared test_utils module 01b4619
* **scoring:** add solution-aware filter traits 25e81fb
* **scoring:** add SolvableSolution trait for fluent API 4e2084a
* **scoring:** add typed entity_counter to TypedScoreDirector 3db368e
* **scoring:** add TypedScoreDirector::take_solution() for solution extraction cc6a10b
* **scoring:** make UniConstraintStream use solution-aware filters b36fa9c
* **scoring:** pass solution and indices to IncrementalBiConstraint weight function d50f091
* **scoring:** pass solution and indices to IncrementalTriConstraint weight function 8b4ccaa
* **scoring:** refactor cross-constraint weight to use solution reference e49db81
* **scoring:** solution-aware filter traits (BREAKING) 74a0cb3
* **solver:** add BasicConstructionPhaseBuilder for basic variables c9a0ee5
* **solver:** add BasicLocalSearchPhaseBuilder for basic variables 1a6e4b1
* **solver:** add best_solution_callback field to Solver struct a255208
* **solver:** add best_solution_callback field to SolverScope 314df2d
* **solver:** add create_solver() and solve_with_director() 617453f
* **solver:** add EitherChangeMoveSelector and EitherSwapMoveSelector adaptors cc9e2ae
* **solver:** add EitherMove enum for monomorphized union of ChangeMove + SwapMove 951ca78
* **solver:** add k-opt reconnection patterns 010c212
* **solver:** add KOptMove for k-opt tour optimization 6376585
* **solver:** add KOptMoveSelector for k-opt move generation fbc571d
* **solver:** add KOptPhaseBuilder fluent API for k-opt local search ec7f538
* **solver:** add KOptPhaseBuilder for tour optimization a2737ab
* **solver:** add ListChangeMoveSelector for element relocation 3d036e1
* **solver:** add ListConstructionPhaseBuilder with change notification b5a4ce8
* **solver:** add NearbyKOptMoveSelector for efficient k-opt 93855a0
* **solver:** add run_solver for basic variable problems e720bbe
* **solver:** add shared test_utils module 8229784
* **solver:** add SolverEvent and solve_with_events for real-time feedback 47911f3
* **solver:** add SolverManager::builder() static method eb10747
* **solver:** add termination flag to run_solver_with_events bdbb222
* **solver:** add with_best_solution_callback() builder method 175b879
* **solver:** add with_best_solution_callback() builder method to SolverScope 9448e24
* **solver:** add with_phase_factory, with_config, Result-returning build a24c412
* **solver:** add zero-erasure fluent phase functions (construction, 2-opt) 5640315
* **solver:** export BasicConstructionPhaseBuilder and BasicLocalSearchPhaseBuilder 29e6d74
* **solver:** export ListConstructionPhaseBuilder 51bb734
* **solverforge-py:** track source class index in ForEach and Join constraint ops 27db23e
* **solverforge:** add verbose-logging feature for debug output 75f3fff
* **solverforge:** export k-opt types from umbrella crate a7a559f
* **solver:** invoke best_solution_callback when solution improves 4721ebe
* **solver:** propagate best_solution_callback in impl_solver! solve() 11dd83d
* **solver:** return SolveResult with telemetry from Solver::solve() e737e15
* **solver:** rewrite SimulatedAnnealingAcceptor with true Boltzmann distribution 17e2150
* **solver:** stream best solutions through channel in run_solver_with_channel 29a4316
* **solver:** wire UnionMoveSelector + SimulatedAnnealing in basic.rs 22c742a
* **stats:** add zero-erasure SolverStats and PhaseStats 87b3950
* **telemetry:** wire stats recording in phases and scope 2d686eb
* **termination:** export all termination types from fluent API 043458f
* **termination:** export MoveCountTermination and ScoreCalculationCountTermination a50fe18
* **termination:** restore MoveCountTermination using stats API 17a284e
* **termination:** restore ScoreCalculationCountTermination using stats API 203817c


### Bug Fixes

* add diagnostic logging and read_timeout to Overpass client 40e76a4
* add path to all internal dependencies and move macro tests 83826e7
* add solverforge-console to publish pipeline and skip already-published crates 6197d0d
* **benchmark:** wire SolveResult through Solvable trait for zero-erasure stats 6c10672
* **ci:** add local CI support 57dfd6e
* **cli:** fail fast in generated constraint scaffolds 2a04ec7
* **cli:** gate unfinished console command c7bac29
* **cli:** hide unsupported generic list scaffold 8129ef5
* **cli:** keep console subcommand as hidden compatibility alias 2f0ea5b
* **cli:** make generated constraints compile safely faf2064
* **cli:** make salvaged scaffolds use published solverforge-ui 843d690
* **clippy:** resolve collapsible_if, unnecessary_map_or, and boxed_local warnings 9e0ee18
* **cli:** preserve list specialization guidance b78a4a4
* **cli:** replace data-loader panic stub 43036e9
* **cli:** restore scaffold smoke coverage 89b4772
* **cli:** satisfy strict clippy for test modules cfcc411
* **console:** capture moves_speed, calc_speed, acceptance_rate in EventVisitor 2fd2bc5
* **console:** cover production event payloads 0ad5ae7
* **console:** flush stdout after banner print 5e8a31f
* **console:** remove inappropriate doc comments in lib.rs 7daeb4d
* **console:** use single default directive in EnvFilter e8917f2
* **cvrp:** reject missing shared problem data 64437af
* **docs:** correct inappropriate /// doc comments on private items 209a680
* **doctest:** correct import paths for KOptPhaseBuilder and ListConstructionPhaseBuilder 0e37017
* **dynamic:** derive is_hard from weight component, not impact type 934b3b1
* **dynamic:** remove ignored tests and fix doctests 66aab07
* **dynamic:** use descriptor.value_ranges in value_count calculation 619716b
* eliminate all clippy and dead_code warnings 4febbdf
* **export:** add derive macros at root level and fix __internal imports 1e2c706
* **history): fix(scoring:** GroupedUniConstraint new-group old_score and filter propagation 02fa038
* **history:** fix serde feature flag and add consistent serde derives 88bf736
* initialize tracing subscriber in vehicle-routing 10650fc
* **k-opt:** use popped position in NearbyCutIterator::backtrack 88913fa
* **list-change:** filter out no-op intra-list moves ddfc943
* **localsearch:** add explicit type annotations in forager tests c7e429f
* **macros:** move console feature gate to library 9890039
* **macros:** remove duplicate pass fixture imports 9fa8c05
* **macros:** update internal type references for __internal module 32a5d9f
* **macros:** use ScoreDirector API correctly for solve() bfc1ae6
* mute type_complexity clippy warning on with_time_limit_or 3d3e4dd
* **nqueens:** update to new TypedScoreDirector 2-argument API 4148248
* platform call conv in JIT compiler and timing robustness in DiminishedReturnsTermination e661b61
* **publish:** add version specs for workspace crate publishing a6c1267
* **release:** address integration review regressions 02f10c0
* **release:** document staged publish dry-runs 2adca83
* **release:** port logging, score parsing, and sf-config wiring b6a639b
* remove circular dependency in solverforge-macros tests bf55201
* remove debug eprintln from EntitySelector 9ec1b9b
* remove filter_with_solution() - use shadow variables on entities instead 20aae71
* remove unused CallConv import in jit/compiler.rs f2380c0
* remove unused imports in solverforge-dynamic test files 86fc1b2
* replace .clone() with copy for Copy types (clippy) 086da7e
* replace needless range loops with iterator find in Score trait 0f70c10
* resolve clippy warnings — allow type_complexity for PhantomData fn() pattern, fix useless_conversion fc7113d
* **scoring:** add Penta weight adapter to match Quad pattern 265c55d
* **scoring:** allow too_many_arguments on GroupedUniConstraint::new b767edf
* **scoring:** correct imports in collector test module 88bb63d
* **scoring:** correct imports in constraint_set test module 6374a5c
* **scoring:** correct imports in filter test module 3468570
* **scoring:** fix incremental scoring corruption with multiple entity classes 8ef0d6e
* **scoring:** panic on i64 overflow in as_date() cfba66c
* **scoring:** panic on i64 overflow in DateOf evaluation f3b49e7
* **scoring:** panic on overflow in IntegerRange 8fe4287
* **scoring:** panic on overflow in ValueRangeDef::len() 60cb681
* **scoring:** remove inappropriate doc comments in constraint/tests/test_incremental.rs 70d983e
* **scoring:** remove inappropriate doc comments in moves/iterator.rs 3f3defa
* **scoring:** remove unused variables in complemented constraint retract 880ea84
* **scoring:** set score on solution in into_working_solution 47ae6be
* **scoring:** standardize QuadConstraint weight type to solution+indices 6408137
* **scoring:** update get_matches macros for new compute_score signature 6e3b597
* **scoring:** update Quad and Penta tests for new weight signatures 34577de
* **scoring:** update tests for solution-aware filter signatures dd74f83
* **scoring:** wire descriptor_index through TypedScoreDirector to constraints 646a037
* **solver:** delete SolverBuilder::solve that violated API contract abf3b31
* **solverforge-py:** accept Solver reference in constraint builder for_each and join methods df4ba2d
* **solver:** gate test-only k_opt re-exports with #[cfg(test)] 85d0d7a
* **solver:** initialize best_solution_callback in Solver::new() 8c2375a
* **solver:** initialize best_solution_callback in SolverScope::new() aa6dced
* **solver:** initialize best_solution_callback in SolverScope::with_seed() 5535d3d
* **solver:** initialize best_solution_callback in SolverScope::with_terminate() 4bde149
* **solver:** propagate best_solution_callback in solve_with_director() 29913b0
* **solver:** propagate best_solution_callback in with_terminate() 8c59738
* **solver:** propagate best_solution_callback in with_termination() 1aba2db
* **solver:** remove inappropriate doc comments in solve/tests/mod.rs f6fb224
* **solver:** remove inappropriate doc comments in solve/tests/test_solve.rs d825c48
* **solver:** restore EntitySelector in KOptPhaseBuilder 97c5194
* **solver:** suppress clippy type_complexity warning for callback fields 8f1b0fe
* **solver:** update cached_score after rejected move undo 0506dd0
* **solver:** use incremental protocol for score updates e9daa1e
* switch reqwest to rustls-tls for async DNS resolution 59d2c00
* **termination:** wire time limit through to SolverScope 064f8a7
* **test:** enable doctests by removing ignore annotation c8c159b
* **test:** handle Result return type from SolverManagerBuilder::build() 64256f0
* **test:** increase timing margins in diminished_returns test f8310f4
* use copy instead of clone on EitherMove::Swap (clippy clone_on_copy) baaf8e3
* use oldest reference point in DiminishedReturnsTermination 8b2522e
* use tokio::fs for async filesystem operations 032325c
* **vehicle-routing:** add fallback for missing route geometry b3cfbac
* **vehicle-routing:** disable solve button immediately on click 44b9aca
* **vehicle-routing:** disable solve button immediately on click 43b98d6
* **vehicle-routing:** fix encode_routes doctest to set up route_geometries 77b6112
* **vehicle-routing:** parse geometry API response correctly c3c5dfb
* **vehicle-routing:** skip construction when visits already assigned 0e08959
* **vnd:** implement Debug manually to avoid S: Debug bound 7a04b1e
* widen timing margins in diminished returns tests for macOS CI 90f4360
* wire entity_count into TypedScoreDirector — construction and local search were producing 0 steps 0971a5a

## [0.6.0](///compare/v0.5.18...v0.6.0) (2026-03-27)


### Features

* **cli:** implement solverforge-cli crate with full scaffolding and codegen 9f2482c
* **cli:** salvage generic scaffold refresh for review 27123e2


### Bug Fixes

* **cli:** fail fast in generated constraint scaffolds 871d654
* **cli:** gate unfinished console command adaccaf
* **cli:** hide unsupported generic list scaffold 70d8b02
* **cli:** keep console subcommand as hidden compatibility alias 8bf6fb5
* **cli:** make generated constraints compile safely f446968
* **cli:** make salvaged scaffolds use published solverforge-ui 1279239
* **cli:** preserve list specialization guidance 6986547
* **cli:** replace data-loader panic stub 498bdc1
* **cli:** restore scaffold smoke coverage afd5ac6
* **cli:** satisfy strict clippy for test modules bf07e9a
* **console:** cover production event payloads 7a25559
* **cvrp:** reject missing shared problem data d0ce288
* **macros:** remove duplicate pass fixture imports 565bffd
* **release:** address integration review regressions c0f6571
* **release:** document staged publish dry-runs f1783bb
* **release:** port logging, score parsing, and sf-config wiring dac2498

## [0.5.19](///compare/v0.5.17...v0.5.19) (2026-03-14)


### Features

* **scoring): CollectionExtract trait for ergonomic extractors + chore(release:** 0.5.19 e97fe9a

## [0.5.19](///compare/v0.5.16...v0.5.19) (2026-03-14)

## [0.5.16](///compare/v0.5.12...v0.5.16) (2026-03-14)


### Features

* add ListClarkeWrightPhase and remove nqueens example e02deb9
* add ListKOptPhase, solverforge-cvrp lib, and fix doctest signatures 79e9b38
* add NearbyKOpt config support and remove Debug bound from distance meter API 1904b56
* generated domain accessors for constraint streams 84212f4


### Bug Fixes

* replace needless range loops with iterator find in Score trait 0bf1f62

## [0.5.16](///compare/v0.5.12...v0.5.16) (2026-03-13)


### Features

* add ListClarkeWrightPhase and remove nqueens example e02deb9
* add ListKOptPhase, solverforge-cvrp lib, and fix doctest signatures 79e9b38
* add NearbyKOpt config support and remove Debug bound from distance meter API 1904b56
* generated domain accessors for constraint streams 84212f4


### Bug Fixes

* replace needless range loops with iterator find in Score trait 0bf1f62

## [0.5.15](///compare/v0.5.14...v0.5.15) (2026-03-11)

## [0.5.14](///compare/v0.5.13...v0.5.14) (2026-03-11)


### Features

* add ListKOptPhase, solverforge-cvrp lib, and fix doctest signatures 62eef42

## [0.5.13](///compare/v0.5.12...v0.5.13) (2026-03-11)


### Features

* add ListClarkeWrightPhase and remove nqueens example 5f83866
* add NearbyKOpt config support and remove Debug bound from distance meter API 2aadf31

## [0.5.12](///compare/v0.5.11...v0.5.12) (2026-03-10)

## [0.5.11](///compare/v0.5.10...v0.5.11) (2026-03-09)

## [0.5.10](///compare/v0.5.9...v0.5.10) (2026-03-09)


### Features

* config-driven solver construction 729c6f0

## [0.5.9](///compare/v0.5.8...v0.5.9) (2026-03-09)

## [0.5.8](///compare/v0.5.7...v0.5.8) (2026-03-09)

## [0.5.7](///compare/v0.5.6...v0.5.7) (2026-03-09)

## [0.5.6](///compare/v0.5.5...v0.5.6) (2026-03-08)


### Bug Fixes

* **scoring:** allow too_many_arguments on GroupedUniConstraint::new 659bd0e
* **scoring:** GroupedUniConstraint new-group old_score and filter propagation da40769

## [0.5.6](///compare/v0.5.5...v0.5.6) (2026-03-08)


### Bug Fixes

* **scoring:** GroupedUniConstraint new-group old_score and filter propagation da40769

## [0.5.5](///compare/v0.5.4...v0.5.5) (2026-03-08)

## [0.5.4](///compare/v0.5.2...v0.5.4) (2026-03-08)

## [0.5.3](///compare/v0.5.2...v0.5.3) (2026-03-05)

## 0.5.2 (2026-02-21)


### ⚠ BREAKING CHANGES

* import SolverForge from private repo

### Features

* add .github 22db95a
* add nearby list change/swap selectors and list move infrastructure 36aa89a
* add vehicle-routing example 43dc6f2
* **api:** add Solvable and Analyzable traits for public solver API 48f732f
* **api:** re-export run_solver from umbrella crate 0fd7c8e
* **config:** add SolverConfig::load() without fallback 82e89d8
* **config:** add SolverConfig::time_limit() convenience method dad5d1b
* **console:** add auto-init for tracing subscriber c35d5fb
* **console:** add console output implementation f89e8b7
* **console:** add SolverForge banner on init 717771c
* **console:** add solverforge_dynamic=info tracing directive e33e72d
* **console:** add verbose step logging at TRACE level a3795cb
* **console:** create solverforge-console crate 8237a8d
* **core:** add ListVariableSolution trait for list-based planning e78401d
* default to Real Roads mode in vehicle-routing UI 34c023f
* **deploy:** fix CI d1f35a9
* **dynamic/jit:** enhance JIT compiler with compile_n and unified JitFn 9191b78
* **dynamic:** add benchmark comparing O(1) HashMap vs O(n) linear filter lookup 49792cf
* **dynamic:** add benchmark test and document lazy iterator performance 358f4c3
* **dynamic:** add best solution callback to update shared snapshot f15bfed
* **dynamic:** add build_bi_self_constraint factory for self-join constraints e80211c
* **dynamic:** add build_cross_bi_constraint factory for cross-bi-entity constraints fee55e7
* **dynamic:** add build_flattened_bi_constraint factory for flattened bi-entity constraints 3d3c0fd
* **dynamic:** add build_penta_self_constraint factory for penta-entity self-join constraints 955ade8
* **dynamic:** add build_quad_self_constraint factory for quad-entity self-join constraints 78375fe
* **dynamic:** add build_tri_self_constraint factory for tri-entity self-join constraints 6543297
* **dynamic:** add build_uni_constraint factory for unary constraints 33e69d0
* **dynamic:** add cross-join type aliases for incremental constraints c5146ef
* **dynamic:** add DynamicEitherMove enum wrapping Change + Swap 04bfcfe
* **dynamic:** add DynamicMoveIterator for lazy move generation 9b13648
* **dynamic:** add DynamicSwapMove for value swaps between entities c5cb39c
* **dynamic:** add eval_entity_expr for single-entity expression evaluation 161c14f
* **dynamic:** add flattened constraint type aliases 59145f4
* **dynamic:** add id_to_location mapping to DynamicSolution 9e8f74a
* **dynamic:** add make_bi_filter for bi-entity filter closure creation ff03ac3
* **dynamic:** add make_bi_weight for bi-entity weight computation e4625e9
* **dynamic:** add make_cross_extractor_a and make_cross_extractor_b for cross-join constraints 6c3fc1d
* **dynamic:** add make_cross_filter and make_cross_weight for cross-join constraints 501d45a
* **dynamic:** add make_cross_key_a and make_cross_key_b for cross-join key extraction 9b23da2
* **dynamic:** add make_extractor function for entity slice extraction c5e5594
* **dynamic:** add make_flatten, make_c_key_fn, and make_a_lookup for flattened bi-constraints a06a828
* **dynamic:** add make_flattened_filter and make_flattened_weight for flattened bi-constraints c435b31
* **dynamic:** add make_key_extractor for join key extraction ac79d6d
* **dynamic:** add make_penta_filter and make_penta_weight for penta-entity operations b641979
* **dynamic:** add make_quad_filter and make_quad_weight for quad-entity operations 96f2b93
* **dynamic:** add make_tri_filter and make_tri_weight for tri-entity operations 28b474b
* **dynamic:** add runtime warnings for unsupported key expression constructs 03b676b
* **dynamic:** add tests for cross-class constraints with same-named fields e374a89
* **dynamic:** add type aliases for boxed constraint closures a108f74
* **dynamic:** change DynBiWeight to accept solution reference and indices 2f4176b
* **dynamic:** change DynCrossWeight to accept solution reference and indices 1450e23
* **dynamic:** change DynTriWeight to accept solution reference and indices d64ee4f
* **dynamic:** document key expression limitations in cross-constraint closures e4775a9
* **dynamic:** document shuffled iteration design in MoveSelector 75fffed
* **dynamic:** implement O(1) entity location lookup via id_to_location HashMap 1aceab7
* **dynamic:** refactor generate_moves to return lazy iterator e59abab
* **dynamic:** rewrite DynamicMoveSelector to generate change + swap moves a49eba2
* **dynamic:** verify solverforge-scoring dependency in Cargo.toml 073301d
* implement three-tier road network caching dce11b3
* import SolverForge from private repo b36e7e5
* **jit:** add Cranelift JIT compiler for Expr trees 4560f38
* **jit:** zero-fallback JIT, eliminate key extractor temp buffer, add Python .pyi stub 2ec55ca
* **lib:** export stats module and types 6ac4bba
* **macros:** add StandardVariableConfig struct and parser 351510f
* **macros:** add shadow variable attribute parsing d4396dd
* **macros:** generate entity_count and list operation methods eeca2e1
* **macros:** generate helper methods for basic variables 1bd8c7e
* **macros:** generate solve() method for standard variable problems ff6a1b2
* **macros:** implement SolvableSolution trait in planning_solution macro f3fd5e4
* **macros:** parse constraints attribute and embed path in solve() 7ea88d6
* **manager:** add with_phase_factory() to SolverManagerBuilder 7ab9aed
* **py:** init console at module import for early banner display e7e7786
* **py:** release GIL during native solve loop b59350f
* **routing:** initialize road routing when creating job 48d0607
* ruin-and-recreate ListRuinMove + construction termination fix 87196b8
* **scope:** add PhaseStats to PhaseScope 343e5bf
* **scope:** add SolverStats to SolverScope ad47890
* **scoring:** add descriptor_index parameter to incremental constraint methods 7daf7b3
* **scoring:** add descriptor_index to TypedScoreDirector public API 433edb9
* **scoring:** add into_working_solution to TypedScoreDirector a4d4c39
* **scoring:** add shadow-aware after_variable_changed and do_change methods 57c2074
* **scoring:** add ShadowVariableSupport and ShadowAwareScoreDirector edfbc1a
* **scoring:** add ShadowVariableSupport::update_all_shadows() with default impl b7a41fd
* **scoring:** add shared test_utils module e2962b3
* **scoring:** add solution-aware filter traits 7f1fc2b
* **scoring:** add SolvableSolution trait for fluent API fcc66e2
* **scoring:** add typed entity_counter to TypedScoreDirector 578d4bf
* **scoring:** add TypedScoreDirector::take_solution() for solution extraction 50de349
* **scoring:** make UniConstraintStream use solution-aware filters d6effea
* **scoring:** pass solution and indices to IncrementalBiConstraint weight function 1379a8b
* **scoring:** pass solution and indices to IncrementalTriConstraint weight function 7983ec8
* **scoring:** refactor cross-constraint weight to use solution reference 9f28c6c
* **scoring:** solution-aware filter traits (BREAKING) 7ac81d2
* **solver:** add BasicConstructionPhaseBuilder for basic variables 214e53f
* **solver:** add BasicLocalSearchPhaseBuilder for basic variables 040e6a6
* **solver:** add best_solution_callback field to Solver struct 3b5a1af
* **solver:** add best_solution_callback field to SolverScope 1419b0a
* **solver:** add create_solver() and solve_with_director() 57b3471
* **solver:** add EitherChangeMoveSelector and EitherSwapMoveSelector adaptors 2636a92
* **solver:** add EitherMove enum for monomorphized union of ChangeMove + SwapMove f9f8a2f
* **solver:** add k-opt reconnection patterns 010c212
* **solver:** add KOptMove for k-opt tour optimization 6376585
* **solver:** add KOptMoveSelector for k-opt move generation fbc571d
* **solver:** add KOptPhaseBuilder fluent API for k-opt local search 9222aa2
* **solver:** add KOptPhaseBuilder for tour optimization c2f3ba9
* **solver:** add ListChangeMoveSelector for element relocation 31357b3
* **solver:** add ListConstructionPhaseBuilder with change notification ea67ba9
* **solver:** add NearbyKOptMoveSelector for efficient k-opt 93855a0
* **solver:** add run_solver for standard variable problems 6c6b228
* **solver:** add shared test_utils module 12af820
* **solver:** add SolverEvent and solve_with_events for real-time feedback 45215e7
* **solver:** add SolverManager::builder() static method 1d2a063
* **solver:** add termination flag to run_solver_with_events bf593e1
* **solver:** add with_best_solution_callback() builder method 75e1b33
* **solver:** add with_best_solution_callback() builder method to SolverScope e7adef6
* **solver:** add with_phase_factory, with_config, Result-returning build 91488e2
* **solver:** add zero-erasure fluent phase functions (construction, 2-opt) 720c9a9
* **solver:** export BasicConstructionPhaseBuilder and BasicLocalSearchPhaseBuilder fd6f676
* **solver:** export ListConstructionPhaseBuilder a65ecaf
* **solverforge-py:** track source class index in ForEach and Join constraint ops 22b367a
* **solverforge:** add verbose-logging feature for debug output db4b81b
* **solverforge:** export k-opt types from umbrella crate a7a559f
* **solver:** invoke best_solution_callback when solution improves c18882c
* **solver:** propagate best_solution_callback in impl_solver! solve() 3b60d3b
* **solver:** return SolveResult with telemetry from Solver::solve() 502bca1
* **solver:** rewrite SimulatedAnnealingAcceptor with true Boltzmann distribution 26f6c60
* **solver:** wire UnionMoveSelector + SimulatedAnnealing in standard.rs 8ef7d84
* **stats:** add zero-erasure SolverStats and PhaseStats fd54200
* **telemetry:** wire stats recording in phases and scope be7ea2d
* **termination:** export all termination types from fluent API 5fceaf6
* **termination:** export MoveCountTermination and ScoreCalculationCountTermination 94b8fde
* **termination:** restore MoveCountTermination using stats API 3509840
* **termination:** restore ScoreCalculationCountTermination using stats API 46e7d0f


### Bug Fixes

* add diagnostic logging and read_timeout to Overpass client 40e76a4
* add path to all internal dependencies and move macro tests 1f23aec
* **benchmark:** wire SolveResult through Solvable trait for zero-erasure stats abd6c54
* **ci:** add local CI support 579ca12
* **clippy:** resolve collapsible_if, unnecessary_map_or, and boxed_local warnings 9207813
* **console:** capture moves_speed, calc_speed, acceptance_rate in EventVisitor ac5d1b9
* **console:** flush stdout after banner print 6b1932e
* **console:** remove inappropriate doc comments in lib.rs 0e45819
* **console:** use single default directive in EnvFilter f29ef74
* **docs:** correct inappropriate /// doc comments on private items be6b0d4
* **doctest:** correct import paths for KOptPhaseBuilder and ListConstructionPhaseBuilder 75b51e8
* **dynamic:** derive is_hard from weight component, not impact type 0350928
* **dynamic:** remove ignored tests and fix doctests e81d08b
* **dynamic:** use descriptor.value_ranges in value_count calculation 6e054db
* eliminate all clippy and dead_code warnings 3040e73
* **export:** add derive macros at root level and fix __internal imports dad0391
* initialize tracing subscriber in vehicle-routing 10650fc
* **k-opt:** use popped position in NearbyCutIterator::backtrack 858b7fb
* **list-change:** filter out no-op intra-list moves e8b4db7
* **localsearch:** add explicit type annotations in forager tests 7e283c6
* **macros:** move console feature gate to library 1fb9c8b
* **macros:** update internal type references for __internal module 56a4c59
* **macros:** use ScoreDirector API correctly for solve() 96e76ea
* mute type_complexity clippy warning on with_time_limit_or 3fba516
* **nqueens:** update to new TypedScoreDirector 2-argument API ec1cc75
* platform call conv in JIT compiler and timing robustness in DiminishedReturnsTermination 155e9b1
* **publish:** add version specs for workspace crate publishing 9723ef7
* remove circular dependency in solverforge-macros tests 4c953ab
* remove debug eprintln from EntitySelector 927d01e
* remove filter_with_solution() - use shadow variables on entities instead c96b5b7
* remove unused CallConv import in jit/compiler.rs fba0cef
* remove unused imports in solverforge-dynamic test files 69e78ed
* replace .clone() with copy for Copy types (clippy) efa42bf
* resolve clippy warnings — allow type_complexity for PhantomData fn() pattern, fix useless_conversion 1fff2f4
* **scoring:** add Penta weight adapter to match Quad pattern 392f9de
* **scoring:** correct imports in collector test module 760ec3e
* **scoring:** correct imports in constraint_set test module cab52f6
* **scoring:** correct imports in filter test module 97753c7
* **scoring:** panic on i64 overflow in as_date() 55f1500
* **scoring:** panic on i64 overflow in DateOf evaluation 5acb116
* **scoring:** panic on overflow in IntegerRange 6f24523
* **scoring:** panic on overflow in ValueRangeDef::len() e9f1b51
* **scoring:** remove inappropriate doc comments in constraint/tests/test_incremental.rs e250d26
* **scoring:** remove inappropriate doc comments in moves/iterator.rs e625bfc
* **scoring:** remove unused variables in complemented constraint retract e2e12d4
* **scoring:** set score on solution in into_working_solution 219f291
* **scoring:** standardize QuadConstraint weight type to solution+indices 7d4c880
* **scoring:** update get_matches macros for new compute_score signature e2243c6
* **scoring:** update Quad and Penta tests for new weight signatures 107ed1f
* **scoring:** update tests for solution-aware filter signatures 8c5a44e
* **scoring:** wire descriptor_index through TypedScoreDirector to constraints f0c91db
* **solver:** delete SolverBuilder::solve that violated API contract 96d261c
* **solverforge-py:** accept Solver reference in constraint builder for_each and join methods c483c14
* **solver:** gate test-only k_opt re-exports with #[cfg(test)] 3f67114
* **solver:** initialize best_solution_callback in Solver::new() ac7a0d5
* **solver:** initialize best_solution_callback in SolverScope::new() f82bc54
* **solver:** initialize best_solution_callback in SolverScope::with_seed() f70fa3e
* **solver:** initialize best_solution_callback in SolverScope::with_terminate() 640c8cd
* **solver:** propagate best_solution_callback in solve_with_director() 22cdc8c
* **solver:** propagate best_solution_callback in with_terminate() 98c9529
* **solver:** propagate best_solution_callback in with_termination() d234381
* **solver:** remove inappropriate doc comments in solve/tests/mod.rs bb19b75
* **solver:** remove inappropriate doc comments in solve/tests/test_solve.rs cc31be2
* **solver:** restore EntitySelector in KOptPhaseBuilder d8b730a
* **solver:** suppress clippy type_complexity warning for callback fields e29c91b
* **solver:** update cached_score after rejected move undo 5e7105b
* **solver:** use incremental protocol for score updates 0e4b5e1
* switch reqwest to rustls-tls for async DNS resolution 59d2c00
* **termination:** wire time limit through to SolverScope b094d94
* **test:** enable doctests by removing ignore annotation b970d73
* **test:** handle Result return type from SolverManagerBuilder::build() 2d05ae5
* **test:** increase timing margins in diminished_returns test e03028d
* use copy instead of clone on EitherMove::Swap (clippy clone_on_copy) af7cb0b
* use oldest reference point in DiminishedReturnsTermination 90d0640
* use tokio::fs for async filesystem operations 032325c
* **vehicle-routing:** add fallback for missing route geometry b3cfbac
* **vehicle-routing:** disable solve button immediately on click 44b9aca
* **vehicle-routing:** disable solve button immediately on click 43b98d6
* **vehicle-routing:** fix encode_routes doctest to set up route_geometries 77b6112
* **vehicle-routing:** parse geometry API response correctly c3c5dfb
* **vehicle-routing:** skip construction when visits already assigned 0e08959
* **vnd:** implement Debug manually to avoid S: Debug bound c0093f6
* wire entity_count into TypedScoreDirector — construction and local search were producing 0 steps 903dd43

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
* **macros:** add StandardVariableConfig struct and parser b0a811b
* **macros:** add shadow variable attribute parsing cadf410
* **macros:** generate entity_count and list operation methods 5a9061e
* **macros:** generate helper methods for basic variables 1b95593
* **macros:** generate solve() method for standard variable problems 8810993
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
* **solver:** add run_solver for standard variable problems f4a8084
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
