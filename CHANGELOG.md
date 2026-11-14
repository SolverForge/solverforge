# Changelog

All notable changes to this project will be documented in this file.
## [0.28.0](https://github.com/SolverForge/solverforge/compare/v0.27.0...v0.28.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* **scoring:** Constraint analysis describes entities by type name and `Debug` rendering only. `EntityRef::as_entity` and `EntityRef::with_display` are removed and `EntityRef::new` takes any `T: Debug + ?Sized`; read the entity from your solution instead of from the match. Grouped and complemented constraints, which now report their matches (#261), require `Debug` on their entities like every other constraint. `SharedOperatorBuilder::with_explainer` and `OperatorScorerSet::score_for` are removed. Scoring streams no longer require `Clone` on entity types.

### Features

* **solver:** report the resolved solver policy in final telemetry ([dc34520](https://github.com/SolverForge/solverforge/commit/dc34520a370cc57c88770059294876c7c51880ae)), references #253


### Bug Fixes

* **scoring:** describe matched entities without copying them ([19188ac](https://github.com/SolverForge/solverforge/commit/19188acc3cbb01df9457566d204a8bdc822c00e9)), references #261
* **scoring:** report matches for grouped and complemented constraints ([ac41c31](https://github.com/SolverForge/solverforge/commit/ac41c313ecaedfc498eed425266b5afdefae691d)), closes #261


## [0.27.0](https://github.com/SolverForge/solverforge/compare/v0.26.1...v0.27.0) (2026-11-13)


### ⚠ BREAKING CHANGES

* **cvrp:** ProblemData's distance_matrix and travel_times fields are replaced by `metric`, and VrpSolution gains `type Metric: RouteMetric`. Migrate with `metric: DenseMatrix::new(distance_matrix, travel_times)` and `type Metric = DenseMatrix;`.

### Features

* **cvrp:** read leg distances through a pluggable route metric ([73d1aba](https://github.com/SolverForge/solverforge/commit/73d1abacefe6346a6528a2a2d31625a2794e2059)), references #252
* **solver:** bound Clarke-Wright savings to nearest neighbours ([f3fd5b2](https://github.com/SolverForge/solverforge/commit/f3fd5b20a6e5bb08e32ae0035110a2a5ffc2cffe)), closes #252


### Performance Improvements

* **solver:** keep Clarke-Wright owner admission incremental ([07c883d](https://github.com/SolverForge/solverforge/commit/07c883da5d34166d1fa6185ab9cb1cd309320974)), closes #254
* **solver:** open the scalar swap cursor in linear time ([4910340](https://github.com/SolverForge/solverforge/commit/4910340a2d0f96386af4a22e5d4c069d830509ba)), closes #255
* **solver:** resolve Clarke-Wright owner restrictions once per route ([d79cda5](https://github.com/SolverForge/solverforge/commit/d79cda5937fd0e69a9c2bcbe7484eda7187553ba))


## [0.26.1](https://github.com/SolverForge/solverforge/compare/v0.26.0...v0.26.1) (2026-11-11)


### Bug Fixes

* **bench:** report hardware branch misses instead of grading them ([4539f3e](https://github.com/SolverForge/solverforge/commit/4539f3e720e821ffbcac9c51ae6f53ae4ce3eb0c)), closes #250
* **bench:** rotate the timed gate trials through the code layouts ([a06e9db](https://github.com/SolverForge/solverforge/commit/a06e9dbc444162b3d95cab43dda8c56aeab123a7)), closes #245
* **solver:** late acceptance re-anchors only on a dominating level change ([e1b41b4](https://github.com/SolverForge/solverforge/commit/e1b41b4117f573e8a6345cc1807edd0f273694f6))
* **solver:** late acceptance re-anchors only on hard and medium levels ([80c675e](https://github.com/SolverForge/solverforge/commit/80c675e3a503d5aa6e271e30639f081469a5f943))


### Performance Improvements

* **solver:** keep the list ruin count choice inlined ([0773934](https://github.com/SolverForge/solverforge/commit/07739344f0501dc384895a447fb9fb0dd8f5aee5)), closes #248
* **solver:** restore only the touched ruin permutation positions ([84e50f3](https://github.com/SolverForge/solverforge/commit/84e50f3b9ede3d795d3fc55dcba182f23e215f29)), closes #244
* **solver:** simulate the ruin shuffle without a permutation ([178a125](https://github.com/SolverForge/solverforge/commit/178a1258d6ce37baa461aa2831bbdaf92addc0fc))


## [0.26.0](https://github.com/SolverForge/solverforge/compare/v0.25.1...v0.26.0) (2026-11-10)


### ⚠ BREAKING CHANGES

* **scoring:** solverforge-scoring removes ReplicaDescriptor. Build a replica-forking director with ScoreDirector::with_replicas(solution, constraints, constraint_builder, &descriptor, entity_counter) (or the _weighted/_in_mode variants) instead of with_descriptor(solution, constraints, ReplicaDescriptor::new(&descriptor, constraint_builder), entity_counter). DirectorDescriptor has no type parameters, and DirectorDescriptor::replica() is replaced by share().

### Bug Fixes

* **bench:** grade cache and branch behaviour on Cachegrind simulation ([41519c2](https://github.com/SolverForge/solverforge/commit/41519c25cceebb1511c3bb0e2ce024d53282e0b9)), closes #232
* **bench:** grade cache and branch behaviour on layout-stable counters ([ab2e99e](https://github.com/SolverForge/solverforge/commit/ab2e99e8f734e5417b124b5d3b644ab4ad1ca4a8)), closes #239
* **macros:** document or hide every generated public item ([73ead60](https://github.com/SolverForge/solverforge/commit/73ead605eee63c0e0ad7c4a422e3ab25665dd9cb)), closes #235
* **scoring:** build stream closures in narrowly generic functions ([97ef409](https://github.com/SolverForge/solverforge/commit/97ef4096b542f01699ac831ee2a316aa2037559a))


### Code Refactoring

* keep module roots to declarations and re-exports ([69d0457](https://github.com/SolverForge/solverforge/commit/69d04574a5d4aa2c06965b4fac158f29979471ef)), closes #234
* **scoring:** keep the replica constraint builder out of the descriptor type ([bc60fe5](https://github.com/SolverForge/solverforge/commit/bc60fe5b24f83a7424fa44dab57b4c603ab25a52)), closes #238


## [0.25.1](https://github.com/SolverForge/solverforge/compare/v0.25.0...v0.25.1) (2026-11-09)


### Bug Fixes

* **bench:** run both zero-regression gate sides from equal-length paths ([f8d2d10](https://github.com/SolverForge/solverforge/commit/f8d2d103bed3b46734698070016de3387ae8aa9f)), closes #229


### Performance Improvements

* **list:** keep the nearest destinations in one bounded buffer per cursor ([f593fa8](https://github.com/SolverForge/solverforge/commit/f593fa875cf76cf1de7eb88df49436dd1bdd95bf)), closes #228


## [0.25.0](https://github.com/SolverForge/solverforge/compare/v0.24.0...v0.25.0) (2026-11-09)


### ⚠ BREAKING CHANGES

* **scoring:** SharedOperatorTerminal requires its operator to implement GroupedOperator, and changed_key_count now sums changed group keys instead of counting row-changing notifications.
* **vnd:** VND phases now use `score_tie_break`, whose default is `random`, so a VND phase without `score_tie_break = "first"` may apply a different one of several equally scored best improving moves than before.
* **move_thread_count:** `MoveThreadCount::Count` holds `NonZeroUsize` and `{ count = 0 }` is a configuration error. `ThreadCount::Specific` holds `NonZeroUsize`.
* **solver:** `SearchBuilder::partitioned_phase` is removed. Register a `SolutionPartitioner` with `SearchBuilder::partitioner` instead; the framework builds and runs the phase from the `partitioned_search` configuration.
* **partitioned:** `SolutionPartitioner::recommended_partition_count` returns `Option<NonZeroUsize>`, `FunctionalPartitioner::with_recommended_count` takes a `NonZeroUsize`, and `ThreadCount::resolve` takes the recommendation as a second argument.
* **environment_mode:** `environment_mode = "fast_assert"` now panics on score drift at committed steps and best-solution updates.
* **environment_mode:** `environment_mode = "full_assert"` now panics on drift in any score-director calculation, including move evaluations and evaluator replicas. `Director` gains the provided method `set_score_assertion`.
* **environment_mode:** `environment_mode = "reproducible"`, `"fast_assert"` and `"full_assert"` without `random_seed` now solve with the fixed seed `REPRODUCIBLE_RANDOM_SEED` instead of an operating-system seed.
* **construction:** `group_candidate_limit` without `group_name` is a configuration error.
* **score_director:** `score_director.constraint_match_enabled` now defaults to `true`, and `false` disables `SolverManager::analyze_snapshot` for the job. `SolverRuntime::bind_snapshot_analysis` takes `Option<fn(&S) -> ScoreAnalysis<S::Score>>`, where `None` disables analysis. `SolverManagerError` gains the variant `SnapshotAnalysisDisabled`.
* **score_director:** `score_director.constraint_provider` must name a `constraints` or `constraint_provider` path declared on the solution, or the solve fails. `try_run_dynamic_solver_with_config_parts` takes `&[DynamicConstraintProvider]` instead of one `constraints` function. `RuntimeBuildError` gains the variant `UnknownConstraintProvider`.
* **termination:** `seconds_spent_limit = 0` or `minutes_spent_limit = 0` is now a zero time limit instead of no limit.
* **acceptor:** `simulated_annealing` with both `calibration` and `level_temperatures` is a configuration error.
* **acceptor:** `step_counting_hill_climbing.step_count_limit = 0` is a configuration error.
* **scoring:** a Projection that emits more rows for one input than its MAX_EMITS now panics; raise MAX_EMITS to the real maximum.
* **partitioned:** a `partitioned_search` phase whose `termination` sets `best_score_limit`, `unimproved_step_count_limit` or `unimproved_seconds_spent_limit` is a configuration error.
* **construction:** a list construction heuristic with `value_candidate_limit`, `group_candidate_limit` or a non-default `construction_obligation` is a configuration error.
* **acceptor:** a non-positive or non-finite `great_deluge.water_level_increase_ratio` is a configuration error.
* **termination:** a solve with `best_score_limit`, `step_count_limit` or an unimproved limit but no `seconds_spent_limit`/`minutes_spent_limit` is no longer capped at 30 or 60 seconds. `try_run_solver_with_config_and_search`, `try_run_weighted_solver_with_config_and_search` and `try_run_dynamic_solver_with_config_parts` lose their `default_time_limit_secs` parameter. The `solverforge.execution_policy` trace drops `configured_entrypoint_default_time_limit_ns`, `effective_time_limit_ns` and `time_limit_source`.
* **termination:** every configured solver-level termination criterion now applies, combined with OR; a solve with several criteria can stop earlier than before. The `solverforge.execution_policy` trace drops the `criterion` and `criterion_target` attributes and gains one attribute per criterion; `termination_composition` names every installed criterion.
* **union:** the configuration error for fixed union weights changed from "must all be positive" to "need at least one positive weight"; a union with some zero weights now compiles and runs without those children.
* **move_thread_count:** the default `move_thread_count = "auto"` may now evaluate local-search steps in parallel windows on up to four evaluators, which forks up to three working-solution replicas.

### Features

* **config:** apply a SolverConfigOverride with SolverConfig::with_override ([a73be45](https://github.com/SolverForge/solverforge/commit/a73be457c997c52577069f60132724156ca7d054))
* **construction:** accept k from 2 to 5 for list_k_opt construction ([6ea7773](https://github.com/SolverForge/solverforge/commit/6ea7773d84d1543e391a2cdd3b22ed5e0a7ce197)), closes #179
* **list_k_opt:** polish routes with k-opt for k from 3 to 5 ([161ceb1](https://github.com/SolverForge/solverforge/commit/161ceb1c2527dad65101fd9caf96bb2849c2cb88))
* **manager:** run a retained job under a SolverConfigOverride ([958303a](https://github.com/SolverForge/solverforge/commit/958303a423ff0efbcc4edf898b0e9754aaeeb859)), closes #197
* **move_thread_count:** implement the auto evaluation policy ([51e1bca](https://github.com/SolverForge/solverforge/commit/51e1bcaf69866cbfe837e235d832827747b2fdeb)), closes #173
* **partitioned:** bound partitions by the phase termination ([6a9f27c](https://github.com/SolverForge/solverforge/commit/6a9f27c92a14d0d2ce877771ddc5ee6588c02d28)), closes #199
* **score_director:** gate snapshot analysis on constraint_match_enabled ([5e33d37](https://github.com/SolverForge/solverforge/commit/5e33d374fc8545f92ae93e4fb549ae9a5e3900fe)), closes #170
* **score_director:** select the constraint provider ([1353745](https://github.com/SolverForge/solverforge/commit/135374583cccdf69361f269df4890d3fb7228f95))
* **scoring:** document ConstraintWeight and export it at the crate roots ([47b006f](https://github.com/SolverForge/solverforge/commit/47b006f03e7ed2c4afdde70f1812877d32fda7b1)), closes #214
* **scoring:** fork a director over another solution ([5f14325](https://github.com/SolverForge/solverforge/commit/5f14325537909e4cab6a40760974cfd6a995c999))
* **solver:** run partitioned_search in the framework ([11a4069](https://github.com/SolverForge/solverforge/commit/11a406913ce633e890051c0611372e2cb9ca7881))


### Bug Fixes

* **acceptor:** reject a great-deluge ratio that does not rise ([2031103](https://github.com/SolverForge/solverforge/commit/20311034e6ac7cdbdf5f89eafaf6375dd791442a))
* **acceptor:** reject a zero step-counting step_count_limit ([0f1d83d](https://github.com/SolverForge/solverforge/commit/0f1d83d8864d936d12b70ca2a1ec5fe3557fee04))
* **acceptor:** reject simulated-annealing calibration beside level_temperatures ([601f896](https://github.com/SolverForge/solverforge/commit/601f8968540d12014c3349ae5ff59815013c1c98)), closes #177
* **construction:** reject scalar-only keys on list construction heuristics ([91362a4](https://github.com/SolverForge/solverforge/commit/91362a48ad713332537affa11fb0f66b2dc092a2))
* **construction:** require group_name for group_candidate_limit ([62388fe](https://github.com/SolverForge/solverforge/commit/62388feae69eb9f01a32db6305efd6bdeda35463)), closes #193
* **environment_mode:** check committed steps against a fresh score in fast_assert ([6c8e0ae](https://github.com/SolverForge/solverforge/commit/6c8e0ae4132010fc5dc9b21a1315afb54482fc6c))
* **environment_mode:** check every score calculation in full_assert ([bd910d0](https://github.com/SolverForge/solverforge/commit/bd910d01523ff2d6988a55b6f478a28d2b3c38a4)), closes #172
* **environment_mode:** seed every mode but non_reproducible with a fixed seed ([9f3f36d](https://github.com/SolverForge/solverforge/commit/9f3f36d47fece4c5b63acfb1333232ea8b0e3d0c))
* **localsearch:** resolve move_thread_count when the phase solves ([4faa904](https://github.com/SolverForge/solverforge/commit/4faa904b191129129eecb21d598548666f31bce7)), closes #221
* **move_thread_count:** reject zero thread counts ([6bc823b](https://github.com/SolverForge/solverforge/commit/6bc823ba18b3aeba71c7215e2310ea2cb17c2b7c))
* **partitioned:** bound auto threads by the recommended partition count ([ea589bb](https://github.com/SolverForge/solverforge/commit/ea589bb0b9ac55a70a87fe9d225bf3a2b773f40b)), closes #201
* **scoring:** count changed group keys in changed_key_count ([40e95ab](https://github.com/SolverForge/solverforge/commit/40e95abc68a74592d8e032aad9cd450a93db5359)), closes #210
* **scoring:** panic when a projection emits more than MAX_EMITS ([b6cd9bf](https://github.com/SolverForge/solverforge/commit/b6cd9bf5ef1ab2d6f27c7445f162b7174e7f0891))
* **scoring:** replace count_state's unsafe usize casts with downcasts ([f22f2bd](https://github.com/SolverForge/solverforge/commit/f22f2bd135380d0ce61f3e5cc03214b9a612f943))
* **solver:** keep a runner's declaration only for partitioned search ([9f09b31](https://github.com/SolverForge/solverforge/commit/9f09b31973f35242283ec18ab81348bf3608fc9f)), closes #220
* **termination:** add no hidden time limit ([fb8a0e0](https://github.com/SolverForge/solverforge/commit/fb8a0e0e91dbbcbd8f762468cefef3fa99acf462)), closes #174
* **termination:** apply every configured solver termination criterion ([9a27c3b](https://github.com/SolverForge/solverforge/commit/9a27c3bcd87c59cbefc730a5d90a3cfd69ba426e))
* **termination:** make time_limit literal and saturating ([df48d5a](https://github.com/SolverForge/solverforge/commit/df48d5a77c02e6a5d845d31f4bdb4eb662ba46f8))
* **union:** a zero fixed union weight disables its child ([2ecd40c](https://github.com/SolverForge/solverforge/commit/2ecd40c1f6e0f0da1a2067f086743b0c157e654b)), closes #178
* **vnd:** honour score_tie_break among equally scored best moves ([e28376f](https://github.com/SolverForge/solverforge/commit/e28376fa2d43591eccccb3f420b319b45cba2a1b)), closes #175


### Performance Improvements

* **construction:** reuse candidate buffers across placements ([721f5e6](https://github.com/SolverForge/solverforge/commit/721f5e63afda57c0abdb041171c984be74d9c1e9)), closes #222
* **scoring:** size each projected row list by MAX_EMITS ([3602138](https://github.com/SolverForge/solverforge/commit/3602138242d1c54ddcc75712cdba5e51dad2eedb)), closes #208


### Code Refactoring

* **partitioned:** run a partition pass through one shared function ([0c470c0](https://github.com/SolverForge/solverforge/commit/0c470c019c88776482409f88b73ee19ba0b97d45))
* **solver:** document ResourceMoveCursor and drop its unused method ([b2f02b5](https://github.com/SolverForge/solverforge/commit/b2f02b55c1ad6936d51b9ed952f2f9fc5ee5d16a)), closes #216


## [0.24.0](https://github.com/SolverForge/solverforge/compare/v0.23.0...v0.24.0) (2026-11-01)


### ⚠ BREAKING CHANGES

* **solver:** AcceptorBuilder::build, AcceptorBuilder::build_with_seed and ForagerBuilder::build return Result<_, ConfigValueError>. An accepted_count forager with `limit = 0` is rejected instead of running with limit 1.
* **scoring:** ConstraintSet implementations must provide constraint_weight_refs and the *_weighted methods; ConstraintAnalysis replaces weight with weight_multiplier in scoring and solver analysis; IncrementalConstraint::weight() is removed; ConstraintWeightOverrides and WeightProvider are removed.
* **scoring:** IncrementalConstraintSealed is renamed SingleConstraint and is no longer #[doc(hidden)]. Hand-written constraints write `impl SingleConstraint for MyConstraint {}`.
* **solver:** ListVariableSlot has no construction_list_remove field and ListVariableSlot::new no longer takes that hook.
* **solver:** RuntimeBuildError gains the ConstraintWeights variant.
* **solver:** SearchContext::new returns RuntimeBuildResult<SearchContext> and SearchContext::try_new is removed; build_descriptor_move_selector returns Result<DescriptorSelector<S>, RuntimeBuildError>.
* **solver:** SelectorTelemetry loses its six conflict_repair_* fields, which could never be non-zero, and PhaseStats/SolverStats record_conflict_repair_* methods other than provider_generated now take a count.
* **solver:** Solvable::solve returns RuntimeBuildResult<()>; manual implementations return Ok(()) on success.
* **solver:** SolverManager::solve returns SolverEventReceiver instead of tokio's UnboundedReceiver, and MoveTabuSignature takes and returns slices instead of SmallVec.
* **solver:** SolverScope::rng() is no longer public; use SolverScope::random_below or SolverScope::random_u64.
* **cvrp:** VrpSolution::vehicle_data_ptr(&self, usize) -> *const ProblemData is replaced by vehicle_data(&self, usize) -> Option<&ProblemData>. Implementations return Some(&data) (for an Arc, Some(&self.shared)) instead of a pointer, and None instead of null.
* **solver:** `MoveArena` is no longer exported from solverforge-solver or `solverforge::heuristic`, and `MoveSelector::append_moves` is removed. A custom selector that overrode `append_moves` drops the override; candidates are taken from the selector's cursor by `CandidateId`.
* **config:** `SolverConfig::from_yaml_file`, `SolverConfig::from_yaml_str` and `ConfigError::Yaml` are removed. Convert YAML solver files to TOML and load them with `SolverConfig::from_toml_file` / `from_toml_str`.
* **facade:** `solverforge::bridge` is removed and `solverforge::console` exposes only `init()`; depend on solverforge-bridge or solverforge-console directly for anything else.
* **scoring:** a ScoreDirector built with `new`/`with_descriptor` no longer applies a solution's constraint weights; use the weighted constructors.
* **scoring:** closures, `fn` items and `vec(...)` are no longer stream sources; pass the generated accessor such as `Plan::shifts()`, and give a hand-written `CollectionExtract` a `change_source`.
* **config:** configuration with unknown keys now fails to load with ConfigError::Toml. In Rust, `AcceptorConfig::HillClimbing` and the `ForagerConfig::{BestScore, FirstAccepted, FirstBestScoreImproving, FirstLastStepScoreImproving}` variants are written with `{}`.
* **solver:** consume problem changes by ownership and drop SolverHandle
* **core:** every Score implementation must provide scale(i64).
* models without `solver_toml` no longer pick up a solver.toml from the working directory; add `solver_toml = "../../solver.toml"` (the scaffold default) to keep using a file. RuntimeBuildError gains the Configuration variant.
* **core,scoring:** score arithmetic that overflows i64 now panics in release builds instead of returning a wrapped score; multiply and divide panic for non-finite factors and zero divisors. Score gains provided overflowing_* methods.
* **core,scoring:** solverforge-core no longer exports `domain::listener`, `domain::supply`, `ListVariableSolution`, `ValueRangeProvider`, `FieldValueRangeProvider`, `ComputedValueRangeProvider`, `StaticValueRange`, `IntegerRange`, `SolverForgeError` or `error::Result`; solverforge-scoring no longer exports `ScoreExplanation`, `Indictment`, `IndictmentMap`, `DetailedConstraintEvaluation`, `constraint::shared::compute_hash`, `stream::key_extract::PhantomKey` or `impl_get_matches_nary!`. Facade users lose the same items under `solverforge::domain` and `solverforge::analysis`.
* **solver:** solverforge-solver items are reachable only at the crate root (`solverforge_solver::Name`); module paths such as `solverforge_solver::heuristic::...` and `solverforge_solver::stats::...` are gone. Host-provider contracts (`RuntimeProviderRegistry`, `RuntimeCandidateMetricRegistry`, `RuntimeHostCompoundProvider`, ...) are imported from solverforge-bridge. The deleted factories, builders, selectors, moves, terminations and phase composites listed above have no replacement; configure the solver through SolverConfig and custom search instead.
* **facade:** the `decimal` feature is removed from solverforge and solverforge-core; drop it from `features = [...]`.

### Features

* **core:** add exact Score::scale and ConstraintWeights ([b82ac39](https://github.com/SolverForge/solverforge/commit/b82ac397b6c07edfa09f4ef4345f0f3eb6ceef0a))
* **core:** serialize BendableScore under the serde feature ([7cd008a](https://github.com/SolverForge/solverforge/commit/7cd008a2a22163450738e3f4fe436600edcc97d9)), closes #93
* **facade:** export every type the facade's own signatures use ([f463ff7](https://github.com/SolverForge/solverforge/commit/f463ff7c3b2f9f04b6078fc365b1797a5a264df0)), references #84
* **macros:** generate constraint weight accessors from #[constraint_weights] ([fb6c4cf](https://github.com/SolverForge/solverforge/commit/fb6c4cfae2727413adcf926f453b8f181c183643))
* **macros:** generate solve_with_config for synchronous configured solves ([fb2b9aa](https://github.com/SolverForge/solverforge/commit/fb2b9aae2c30a92f5e711e7e070f296965371262)), references #84
* **scoring:** name the single-constraint marker SingleConstraint ([6e6e148](https://github.com/SolverForge/solverforge/commit/6e6e14866afd5c35ef13dcb55dd6a38c1accedac)), references #84
* **scoring:** publish stream::chain ([18798bf](https://github.com/SolverForge/solverforge/commit/18798bf7bb6ee16c3d68024c17897406c1d9b9b5)), references #84
* **scoring:** scale each constraint's contribution by its bound weight ([f36cbd5](https://github.com/SolverForge/solverforge/commit/f36cbd56cd713991f6def0d331e6d856b6ffd175))
* **solver:** apply queued problem changes at runtime safe points ([2900805](https://github.com/SolverForge/solverforge/commit/29008054a9994441209db415cd322fcdfca0d583))
* **solver:** fail the solve at start on invalid constraint weights ([3d003d2](https://github.com/SolverForge/solverforge/commit/3d003d226041d31fff281654b86e9553bb26e2d9))
* **solverforge:** expose real-time problem changes through the facade ([178cbdb](https://github.com/SolverForge/solverforge/commit/178cbdbce4a774b07f5cabb36dca6116d55507b0))
* **solver:** give SolverScope seeded draws and keep its generator private ([7423718](https://github.com/SolverForge/solverforge/commit/74237186e5573dcae1e5467240b63fbc5431351b)), references #84
* **solver:** validate constraint weights installed by problem changes ([55bbe3a](https://github.com/SolverForge/solverforge/commit/55bbe3a5bbae184bcf9d8c343dfc2d9db5f0162e))


### Bug Fixes

* **bench:** make the zero-regression gate pass an unchanged tree without loosening it ([81c8ed2](https://github.com/SolverForge/solverforge/commit/81c8ed273193f2528528c6949a5dfa6d264f3f17)), references #134
* **bridge:** print and parse decimal DynamicScores through the exact core grammar ([5093022](https://github.com/SolverForge/solverforge/commit/50930229a4e2552b87c3816bc956d5e6d78fce35))
* **config:** reject unknown keys in solver configuration ([677ec55](https://github.com/SolverForge/solverforge/commit/677ec55b620c18f7c599b37bcc268f3ac74c5d79)), closes #88
* **core,scoring:** panic on score arithmetic overflow instead of wrapping ([41d2a61](https://github.com/SolverForge/solverforge/commit/41d2a61f5dabe327862dd9f3e2f3b814b84e9876)), closes #95
* **core:** parse and print HardSoftDecimalScore with exact fixed-point arithmetic ([adb168f](https://github.com/SolverForge/solverforge/commit/adb168ff7e838dcf1879010cecd4540801413bac)), closes #94
* **cvrp:** return Option<&ProblemData> from VrpSolution instead of a raw pointer ([7414ab1](https://github.com/SolverForge/solverforge/commit/7414ab1a2e64e8cf0dad5d0f3063a163c5a384a6)), closes #96
* **facade:** make verbose-logging enable console; delete the empty decimal feature ([7be8892](https://github.com/SolverForge/solverforge/commit/7be889297218603947d44a3a045bb66e30fff18c)), closes #86
* **macros:** pass the user's struct through the model attributes unchanged ([977ba20](https://github.com/SolverForge/solverforge/commit/977ba20ab660bc31179762f6e414882f83f2f7b7)), closes #82
* **macros:** report invalid planning_solution paths as spanned errors ([c461401](https://github.com/SolverForge/solverforge/commit/c461401dea31591eae90056a7db20e987b73e884)), closes #100
* **scoring:** accept only planning-model collections as stream sources ([f997617](https://github.com/SolverForge/solverforge/commit/f9976175a096f845048205dca1d1d4ac70da9d50)), closes #107
* **scoring:** keep cross-stream filters when a five-source join continues ([ae0ec9f](https://github.com/SolverForge/solverforge/commit/ae0ec9fd009b970440f0a9e34c7712d6f1ca2875)), closes #105
* **scoring:** let fluent same-collection self-joins continue past two sources ([16d3952](https://github.com/SolverForge/solverforge/commit/16d395282a17c871ddeb8cbc4ff56178f09bc384)), closes #158
* **scoring:** rebuild replica constraint sets on director reset ([65da6c8](https://github.com/SolverForge/solverforge/commit/65da6c8a87e6e0d1d54fb1cf3a6e24c7ffe7428b))
* **solver:** delete MoveArena and the eager move-collection path ([290882d](https://github.com/SolverForge/solverforge/commit/290882da3ce81e1b850a47e7c72ff2f02125a4df)), closes #109
* **solver:** fail the solve on a best_score_limit that does not parse ([fbea906](https://github.com/SolverForge/solverforge/commit/fbea906543b923694ac950b9af7492fd8afeb631)), closes #127
* **solver:** make SearchContext::new and the descriptor selector fallible ([d7989bc](https://github.com/SolverForge/solverforge/commit/d7989bc55968ad71cf14a026e0b3df0cba57c258)), closes #85
* **solver:** record conflict-repair telemetry where outcomes are decided ([80852fa](https://github.com/SolverForge/solverforge/commit/80852fa8f8c54197b3d205c0aa58951a11b3cd9c))
* **solver:** reject out-of-domain move-selector and construction values ([0b09124](https://github.com/SolverForge/solverforge/commit/0b09124157efe1c529623ff8a5515c20d80b228f)), closes #148
* **solver:** return solve errors from Solvable instead of re-panicking ([9dde1b9](https://github.com/SolverForge/solverforge/commit/9dde1b98ced11c1b4dc6017165e92b18a826d1ad))
* **solver:** validate acceptor and forager values when compiling the config ([e4d7c61](https://github.com/SolverForge/solverforge/commit/e4d7c6193196e8817bdae23c5167c5f15f0b0dd7))
* stop reading solver.toml from the working directory ([e5ca007](https://github.com/SolverForge/solverforge/commit/e5ca007e5062dc42aa25a7ea870727d5605f55f3)), closes #87


### Code Refactoring

* **config:** move the list move-selector configs to move_selector/list.rs ([7ed6b57](https://github.com/SolverForge/solverforge/commit/7ed6b57e98fab765e18b1fa44a3c0affd283620a)), references #91
* **config:** remove YAML solver configuration ([582799a](https://github.com/SolverForge/solverforge/commit/582799a89dead62e6ff6a18e5c45b3c3d6368eca)), closes #89
* **console:** move the solve-end summary formatter to format/solve_end.rs ([98fc4d1](https://github.com/SolverForge/solverforge/commit/98fc4d115172680fafe508bd2ce9629630bfd180)), references #91
* **core,scoring:** delete public types nothing produces or consumes ([824c54e](https://github.com/SolverForge/solverforge/commit/824c54e36398e236a013e189259451c5a7bf23e8)), references #83
* **core:** move the dynamic slot adapters out of domain/dynamic.rs ([d190a27](https://github.com/SolverForge/solverforge/commit/d190a2702b43b1b8541d08868f5ea26421fa09df)), references #91
* **facade:** stop re-exporting solverforge-bridge and solverforge-console ([677c875](https://github.com/SolverForge/solverforge/commit/677c875b11e74d6932725d6b67f87e69b77c15e2)), closes #151
* **macros:** extract the scalar hook wiring from generate_support_impl ([0a9f637](https://github.com/SolverForge/solverforge/commit/0a9f637af7c7b8126d35af825db4ed2f909f6ca7)), references #91
* **scoring:** make the constraint weight mode a ScoreDirector type ([c4a6336](https://github.com/SolverForge/solverforge/commit/c4a6336cd0cba954dd7d390801ef41f548cec40a))
* **scoring:** move the scored cross tri/quad/penta operators into child files ([28c8f5f](https://github.com/SolverForge/solverforge/commit/28c8f5f9c0241cb66f1638367418596adbee5483)), references #91
* **scoring:** split list_precedence.rs and move its tests out of line ([cf755c9](https://github.com/SolverForge/solverforge/commit/cf755c983b72fdba8469da454a114f1e1c9e4dbe)), references #91
* **scoring:** split operator scorers from the shared terminal ([5a6ce7a](https://github.com/SolverForge/solverforge/commit/5a6ce7a4473b9b75dbf5c7c1f52501abfbda800e))
* **solver:** consume problem changes by ownership and drop SolverHandle ([6e82289](https://github.com/SolverForge/solverforge/commit/6e822896186f1782436df97a9f2d12546f1cf62b))
* **solver:** delete the unused construction_list_remove list hook ([7cc7f52](https://github.com/SolverForge/solverforge/commit/7cc7f52ba08839e119e2dd4c0ea1ca4b83b24ec9)), closes #161
* **solver:** export the supported solver surface once and delete what nothing reaches ([2d93015](https://github.com/SolverForge/solverforge/commit/2d930159cc7bec1cf185b12c156318b6f5237d98)), references #83
* **solver:** extract Clarke-Wright savings generation from the kernel ([13ebdac](https://github.com/SolverForge/solverforge/commit/13ebdacd2d89cbe56010a6f971e8b6246416c55a)), references #91
* **solver:** hide tokio and smallvec types behind SolverForge API types ([edd275a](https://github.com/SolverForge/solverforge/commit/edd275a4ea3e1adf17271988951a068aa18bf5c2)), closes #150
* **solver:** move RuntimeListSlot's position meter to runtime_list/meter.rs ([e6a2476](https://github.com/SolverForge/solverforge/commit/e6a2476dc54b62241e0a8cf31e1dbd0047f540f7)), references #91
* **solver:** move configured termination assembly to run/termination.rs ([5c92618](https://github.com/SolverForge/solverforge/commit/5c9261838b5d2d2bd3a482a65085d8da14928430)), references #91
* **solver:** move grouped/provider selector compilation to selector_tree/grouped.rs ([3351a36](https://github.com/SolverForge/solverforge/commit/3351a36ad9400d924d969eb9345431209aebdad0)), references #91
* **solver:** move list-slot cursor opening to cursor/slot/open.rs ([44c676f](https://github.com/SolverForge/solverforge/commit/44c676ff00dc0dabc6950507f4084705889e470a)), references #91
* **solver:** move precedence route deltas and cycle checks to precedence_route/delta.rs ([b66885f](https://github.com/SolverForge/solverforge/commit/b66885f9807afda7468c8d925ab4d55559b501da)), references #91
* **solver:** move runtime list-move tabu and trace identities to recipe_identity.rs ([44085a0](https://github.com/SolverForge/solverforge/commit/44085a01901682d7a01269475d8a2c5ea70900a9)), references #91
* **solver:** move strength-ordered construction selection to forager_step/strength.rs ([cb81c19](https://github.com/SolverForge/solverforge/commit/cb81c197cc63397dd91af1fbdffa9cc1280b4c70)), references #91
* **solver:** move the candidate store and arena cursor out of borrowed.rs ([8fd0898](https://github.com/SolverForge/solverforge/commit/8fd08989fc4848bf797b99f17fad82b59efeaf82)), references #91
* **solver:** move the cartesian product cursor into its own file ([d0302d0](https://github.com/SolverForge/solverforge/commit/d0302d0ca61127eae181417bb1d51f7ac6e8f6dc)), references #91
* **solver:** move the compiled provider plan types to graph/provider.rs ([9589289](https://github.com/SolverForge/solverforge/commit/958928983d7bbb8e09c0346612883a6432532b87)), references #91
* **solver:** move the descriptor nearby selectors out of change_swap.rs ([2e094d8](https://github.com/SolverForge/solverforge/commit/2e094d8cf9e7608a8c31c0a9543d9187b861cd7a)), references #91
* **solver:** move the global construction iterations out of global.rs ([c9f7050](https://github.com/SolverForge/solverforge/commit/c9f7050e6443a55996dad3315f6531354491bf3d)), references #91
* **solver:** split stats/candidate_trace.rs by trace value ([4b760c8](https://github.com/SolverForge/solverforge/commit/4b760c87bc216055e6d76be424a5d9cb24a3e470)), references #91
* **solver:** split the solver scope's included source files ([4503b57](https://github.com/SolverForge/solverforge/commit/4503b574a85b69e6216681f5b32c3778d96ed0c2)), references #91


## [0.23.0](https://github.com/SolverForge/solverforge/compare/v0.22.0...v0.23.0) (2026-10-27)


### ⚠ BREAKING CHANGES

* **solver:** SolverTelemetry.phase is replaced by phases, one
* **solver:** lend tabu signatures through one Cow-returning method
* **bridge:** make DynamicScore a plain three-level value
* **scoring:** score list-shadow writes as element mutations

### Features

* **core:** let solutions report the elements their list shadows write ([6070255](https://github.com/SolverForge/solverforge/commit/60702556995dae614455b48130cc7d3fcb26cc28))
* **scoring:** fork evaluator replicas from ScoreDirector ([04f0c25](https://github.com/SolverForge/solverforge/commit/04f0c2545961270074f9046388ffaf456541f270))
* **solver:** build constraint sets from a constructor at every solve entry ([f3e5286](https://github.com/SolverForge/solverforge/commit/f3e52863c71b0f6ebae5b90bc80e6fe9142b5a0d))
* **solver:** drive acceptor-forager evaluation from move_thread_count ([8296c15](https://github.com/SolverForge/solverforge/commit/8296c15e0afc74a4234312ee3d516010f27cd7dd))
* **solver:** parallel evaluation windows for acceptor-forager local search ([6deae36](https://github.com/SolverForge/solverforge/commit/6deae36d47ffd253c29cb4e7cbe97720ee37cc47))
* **solver:** report speculative evaluation in telemetry and candidate traces ([c644bbe](https://github.com/SolverForge/solverforge/commit/c644bbed12a590992719908a16855828621effbe))
* **solver:** size parallel evaluation windows from the step's consumption ([cca8334](https://github.com/SolverForge/solverforge/commit/cca8334979283c968f2f3e113c62da83c8f841cf))


### Bug Fixes

* **bridge:** make DynamicScore a plain three-level value ([b27c793](https://github.com/SolverForge/solverforge/commit/b27c79344aff4f662ebc7ceb8ea96abaf341ffa7))
* **ci:** keep UI fixture builds inside the workspace ([f898f77](https://github.com/SolverForge/solverforge/commit/f898f7785b2e263b551daef2fb3aa8197ebfe7f6))
* **ci:** override runner target directory per Cargo process ([6885b12](https://github.com/SolverForge/solverforge/commit/6885b12aa5c44b440c7a247fdd51f0c3ef991bf9))
* **scoring:** score list-shadow writes as element mutations ([8e20b1f](https://github.com/SolverForge/solverforge/commit/8e20b1f9d9ab2d3b620b760daa8c140233fe3714))
* **solver:** cap the evaluator pool at one full window ([e94bc3f](https://github.com/SolverForge/solverforge/commit/e94bc3f499c3500b4e05fe707eb2b01b5fd5142e))
* **solver:** read the selector gate's generated-move count through its method ([a5c181a](https://github.com/SolverForge/solverforge/commit/a5c181a47dc42d5fbedba455e6643824436dc9d9))
* **solver:** record solve statistics once, per phase ([8ae3bd9](https://github.com/SolverForge/solverforge/commit/8ae3bd9ca8677f2dc8634031809fb20eae22f8d8))
* **solver:** recycle K-opt undo journals through the compiled runtime carrier ([0ddb843](https://github.com/SolverForge/solverforge/commit/0ddb843d428bd09f1d26f7fbe32f19e41a421097))


## [0.22.0](https://github.com/SolverForge/solverforge/compare/v0.21.3...v0.22.0) (2026-10-18)


### Bug Fixes

* **bench:** skip fixups that do not apply to the baseline ([9a8e2a4](https://github.com/SolverForge/solverforge/commit/9a8e2a4d88df6a59b07c308e4163d6614c2f60dd))
* **scoring:** drop stale key Clone bounds in complement weights ([cbf6e81](https://github.com/SolverForge/solverforge/commit/cbf6e81da37860e8ed0ad4a99364a2f9e0a65f44))


## [0.21.3](https://github.com/SolverForge/solverforge/compare/v0.21.2...v0.21.3) (2026-10-14)


### Bug Fixes

* **scoring:** preserve net route deltas during precedence trials ([770b358](https://github.com/SolverForge/solverforge/commit/770b3583fff66dc9fb10ebf7fd61a42e3e8472e0))


## [0.21.2](https://github.com/SolverForge/solverforge/compare/v0.21.1...v0.21.2) (2026-10-14)


## [0.21.1](https://github.com/SolverForge/solverforge/compare/v0.21.0...v0.21.1) (2026-10-14)


### Bug Fixes

* **solver:** rebase interior composite edges when merging selector subgraphs ([e958244](https://github.com/SolverForge/solverforge/commit/e9582449e936a0f0372c36603803a946dc66737b))


## [0.21.0](https://github.com/SolverForge/solverforge/compare/v0.20.4...v0.21.0) (2026-10-14)


### Features

* **bench:** add BENCH_OWN_SOURCES gate mode for cross-API-generation baselines ([6673eeb](https://github.com/SolverForge/solverforge/commit/6673eeb9f73ac7df29ffd715c3c535e6aeb246e3))
* **scoring:** expose group_by on tri quad and penta joins (#74) ([cd7ab13](https://github.com/SolverForge/solverforge/commit/cd7ab133501d66c1682c30cbd159839de42fba58)), closes #73


### Bug Fixes

* **bench:** make the identity chain data-independent in the measured loop ([9f79919](https://github.com/SolverForge/solverforge/commit/9f79919f4fd265f090304a0a6a2f2ab132008b04))
* **bench:** make the selector gate semantic check encoding-insensitive ([7b92f32](https://github.com/SolverForge/solverforge/commit/7b92f32655cf181ed2aa3dfdcaa94746c3fbdb94))
* **bench:** repair the release-era construction gate benches and own-sources comparator reporting ([4e031c3](https://github.com/SolverForge/solverforge/commit/4e031c31c01449a554a08e0037ed72e89fd22b92))
* **release:** correct csemver regex groups and rewrite dependency versions in postbump ([655f192](https://github.com/SolverForge/solverforge/commit/655f1923da24683f83e3ba75428fb46fcc643677))
* **solver:** forward live preview through borrowed candidates ([f727776](https://github.com/SolverForge/solverforge/commit/f72777680e8ee4477f1e34b6a9674678b2ef1d20))
* **solver:** lend Cartesian row previews to lazy right pulls ([629a868](https://github.com/SolverForge/solverforge/commit/629a868dd06122681624f5b3eaf58e5586d4d869))
* **solver:** make pooled cartesian rows a true slab indexed per open ([d931e11](https://github.com/SolverForge/solverforge/commit/d931e11b08c57c5eef2d52d8b4e44e2d2a590295))
* **solver:** pull the cartesian gate case through the director ([35014a8](https://github.com/SolverForge/solverforge/commit/35014a820fe5bbe054fa92e5d269fdc7c9da61fe))
* **solver:** record the real identity for self-undoing tabu moves; drop per-call key clones in count state ([59b25f0](https://github.com/SolverForge/solverforge/commit/59b25f0a2ef3d391df4d72f626ec97d839cb08cf))
* **solver:** retain full descriptor pillar swap affected scopes ([02cad36](https://github.com/SolverForge/solverforge/commit/02cad36b0bafc28717c590206c52db218f222806))
* **solver:** retain lazy assignment membership index construction ([49d066f](https://github.com/SolverForge/solverforge/commit/49d066f4b663ea06f9b2c9760a1000a0b02697b8))
* **solver:** reuse detached runtime slots and drop stale interior-mutability comments ([b339a84](https://github.com/SolverForge/solverforge/commit/b339a84e466f02299a04efdaff1ec36aa1fd4ec3))


## [0.20.4](https://github.com/SolverForge/solverforge/compare/v0.20.3...v0.20.4) (2026-10-13)

### Features

* **solver:** steered enumeration budgets for grouped-scalar families d2b3c2d

### Bug Fixes

* **solver:** acceptors match their published algorithms c084b5c
* **solver:** exact DLAS bookkeeping and delta-local assignment validation 3992802

## [0.20.3](https://github.com/SolverForge/solverforge/compare/v0.20.2...v0.20.3) (2026-10-12)

### Features

* **solver:** model-declared guided ordering for scalar search 149eb50

### Bug Fixes

* **solver:** reuse the rule verdict blockers already computed 575f2af

## [0.20.2](https://github.com/SolverForge/solverforge/compare/v0.20.1...v0.20.2) (2026-10-12)

### Bug Fixes

* **solver:** batch construction candidate stop polls ac97d1d
* **solver:** batch the wall-clock stop checks in search control loops 8c0a701
* **solver:** pool grouped-scalar assignment index rows 65c1cfc
* **solver:** short-circuit fully-forced assignment rule edges f193f67

## [0.20.1](https://github.com/SolverForge/solverforge/compare/v0.20.0...v0.20.1) (2026-10-12)

### Bug Fixes

* **scoring,solver:** drop per-candidate allocation from precedence scoring and cycle checks b286c8f
* **solver:** pool grouped-scalar construction feasibility scratch a0f825f

## [0.20.0](https://github.com/SolverForge/solverforge/compare/v0.19.14...v0.20.0) (2026-10-12)

### Features

* **solver:** accepted-count foraging for grouped scalar models a901a2b

## [0.19.14](https://github.com/SolverForge/solverforge/compare/v0.19.13...v0.19.14) (2026-10-12)

## [0.19.13](https://github.com/SolverForge/solverforge/compare/v0.19.12...v0.19.13) (2026-10-11)

## [0.19.12](https://github.com/SolverForge/solverforge/compare/v0.19.11...v0.19.12) (2026-10-10)

### Bug Fixes

* **scoring:** avoid repeated retained relationship bookkeeping 8c0e4bc

## [0.19.11](https://github.com/SolverForge/solverforge/compare/v0.19.10...v0.19.11) (2026-10-10)

### Bug Fixes

* **scoring:** avoid repeating exact indexed equality extraction abacb7f
* **scoring:** reuse relational buffers across incremental updates dec3f35
* **solver:** retain complete buffered Clarke-Wright merges on interruption 75925ba

## [0.19.10](https://github.com/SolverForge/solverforge/compare/v0.19.9...v0.19.10) (2026-10-09)

### Bug Fixes

* **bench:** make the zero-regression gate satisfiable and layout-fair cab2e1e
* **bench:** stage both gate sides into equal-length paths 21f7879
* **docs:** synchronize documentation and release generation 51edad1
* **scoring:** filter unary sources before keyed join initialization b0e97d0
* **solver:** advance optional construction past completed roots b7ab523

## [0.19.9](https://github.com/SolverForge/solverforge/compare/v0.19.8...v0.19.9) (2026-10-08)

### Features

* **scoring:** add an arity-free fluent join chain past Penta 0d634da
* **scoring:** add an arity-generic self-join uniqueness operator 4c07663
* **scoring:** add chained relational join with row-aware keys and triple terminal bccb4b3
* **scoring:** add relational binary equi-join operator and generic terminal 30bddbe
* **scoring:** add row-aware chained fluent join with independent key types 1a6783d
* **scoring:** compose indexed semi and anti row producers 35bbad3
* **scoring:** compose membership filters on either join input 8e620c6
* **scoring:** compose target-domain complements with owned defaults e26d80f
* **scoring:** dispatch comparison, overlap, and composed join conditions 48a7771
* **scoring:** flatten borrowed children from arbitrary owned rows c86d784
* **scoring:** fluent join chaining to four and five sources ac3cfc8
* **scoring:** let grouped results join a second collection 7d7e9b0
* **scoring:** own fresh derived values during relational evaluation 9382b76
* **scoring:** publish owned grouped rows through composable operators 7835656
* **scoring:** retain branch identities in generic bag unions 9b58c2e
* **scoring:** share one operator tree across score consumers c153eee

### Bug Fixes

* **solver:** clear the pause request under the pause gate 5c17b92

## [0.19.8](https://github.com/SolverForge/solverforge/compare/v0.19.7...v0.19.8) (2026-10-02)


### Features

* **scoring:** add cross-tri incremental constraint a240f3f
* **scoring:** extend cross Bi streams with a third join source 829509c


### Bug Fixes

* **build:** complete ci-local Rust checks 89ffdb1

## [0.19.7](https://github.com/SolverForge/solverforge/compare/v0.19.6...v0.19.7) (2026-09-25)

## [0.19.6](https://github.com/SolverForge/solverforge/compare/v0.19.5...v0.19.6) (2026-09-25)

### Features

* **core:** expose typed planning pin predicate 76839ed

### Bug Fixes

* **solver:** preserve pinned list owners across construction and search ca05d32
* **solver:** reject scalar edits to pinned entities 10aa801
* **solver:** retain pinned assignments in exhaustive search 4c9f2bf

## [0.19.5](https://github.com/SolverForge/solverforge/compare/v0.19.4...v0.19.5) (2026-09-15)

### Bug Fixes

* **runtime:** retain complete solution on cancellation a777f71
* **solver:** initialize score history with slice fill 978decf
* **solver:** preserve complete Clarke-Wright construction 4749130

## [0.19.4](///compare/v0.19.3...v0.19.4) (2026-08-11)
## [0.19.3](///compare/v0.19.2...v0.19.3) (2026-07-29)


### Bug Fixes

* **solver:** restore shared assignment rotation a4efa44

## [0.19.2](///compare/v0.19.1...v0.19.2) (2026-07-19)


### Bug Fixes

* **solver:** bound required preprocessing control polls 54ce892
* **solver:** commit buffered route telemetry atomically e360dfd
* **solver:** complete required assignment construction 6c468f2
* **solver:** poll construction progress by deadline 106954a
* **solver:** preserve committed score during cheapest trials ab80caa
* **solver:** propagate interrupted construction pulls 0aa1a1c
* **solver:** publish first phase progress promptly a032da4
* **solver:** stream specialized construction progress 8176f5e
* **solver:** unify construction phase lifecycle 7648481

## [0.19.1](///compare/v0.19.0...v0.19.1) (2026-07-18)


### Bug Fixes

* **solver:** fail incomplete solutions at limits db01256

## [0.19.0](///compare/v0.18.0...v0.19.0) (2026-07-17)


### ⚠ BREAKING CHANGES

* **solver:** solverforge-solver no longer exports ConstructionType, LocalSearchAcceptorType, or manager::PhaseConfig.
* **model:** Remove #[planning_variable(chained = true)], VariableType::Chained, ChainedVariableInfo, VariableDescriptor::chained, ShadowVariableKind::Anchor, AnchorSupply, and #[anchor_shadow_variable]; use planning list variables for ordered sequences and routes.

* **solver:** remove inert manager phase config d5039fd


### Features

* **model:** make lists the sole sequence representation 38b1c13


### Bug Fixes

* **bench:** compare cursor-era baselines directly c164145
* **macros:** generate list index shadows 260b51e

## [0.18.0](///compare/v0.17.2...v0.18.0) (2026-07-13)


### Features

* **config:** model resolved selector policy c83610b
* **core:** declare dynamic model capabilities 9e2d179
* **runtime:** compile immutable search graphs 8e54f48
* **solver:** freeze compound provider registries 564ef82
* **telemetry:** qualify candidate execution traces ec3eef9


### Bug Fixes

* **lifecycle:** settle control at phase boundaries f60b199

## [0.17.2](///compare/v0.17.1...v0.17.2) (2026-07-02)


### Features

* **solver:** expose dynamic construction primitives 8d49779
* **solver:** stream required assignment construction state b09c99e

## [0.17.1](///compare/v0.17.0...v0.17.1) (2026-06-15)


### Bug Fixes

* **cvrp:** handle unreachable CVRP legs safely c6ee837
* **solver:** clamp route distance arithmetic 038b729

## [0.17.1](///compare/v0.17.0...v0.17.1) (2026-06-15)


### Bug Fixes

* **cvrp:** handle unreachable CVRP legs safely

## [0.17.0](///compare/v0.16.0...v0.17.0) (2026-06-15)


### Features

* **macros:** add cvrp list domain profile d013bc0


### Bug Fixes

* **cvrp:** keep k-opt route feasibility strict 5f6acdf

## [0.16.0](///compare/v0.15.2...v0.16.0) (2026-06-12)


### Features

* **list:** split route and savings hooks 2c61fa7


### Bug Fixes

* **solver:** complete unmatched Clarke-Wright routes b0b4c7e

## [0.15.2](///compare/v0.15.1...v0.15.2) (2026-06-10)


### Features

* **scoring:** add directed projected self joins 2653a0b

## [0.15.1](///compare/v0.15.0...v0.15.1) (2026-06-02)


### Features

* **bridge:** add dynamic binding bridge crate f3513b1
* **bridge:** expose dynamic model slots from core d664655
* **config:** expose list precedence hooks f9ca1a7
* **core:** add logical descriptor identifiers ffbb922
* **scoring:** add list precedence makespan constraint 47ba8c4
* **solver:** add precedence-aware list neighborhoods f6376a6
* **solver:** honor fixed list element owners e8845d5
* **solver:** run dynamic variables in construction and search 8257945


### Bug Fixes

* **bridge:** derive dynamic score family default ce75bdc
* **console:** preserve explicit solver log filters c3c805c
* **scoring:** rebuild precedence graph for mixed cycle diffs 3ec5d5a
* **solver:** complete mandatory list construction caaf93c
* **solver:** resolve dynamic runtime slots by descriptor 3d464f0

## [0.15.1](///compare/v0.15.0...v0.15.1) (2026-05-24)


### Features

* **solver:** add fixed-owner placement for list elements


### Bug Fixes

* **solver:** score list-regret insertion by true regret delta

## [0.15.0](///compare/v0.14.1...v0.15.0) (2026-05-23)


### Features

* **constraints:** compose shared constraints through typed sets 1b81b19
* **facade:** expose complemented shared-node internals c4c7219
* **macros:** compile repeated grouped bindings into shared nodes f18b013
* **macros:** share provably identical grouped chains 98d5388
* **scoring:** add node sharing diagnostics surface 9bd9d37
* **scoring:** expose hidden shared-node compiler hooks baabdd0
* **scoring:** share complemented cross grouped state 95b5f6a
* **scoring:** share cross grouped join state ed21de8
* **scoring:** share projected complemented grouped state f075fc9
* **scoring:** share projected grouped node state 94b9345
* **scoring:** split grouped constraints into shared node state 5a9cbae
* **solver:** add assignment value-pattern neighborhoods 9f226ec


### Bug Fixes

* **macros:** count ordered-chain member spans f3c46ff
* **macros:** emit ordered same-binding grouped sharing fbbc95f
* **macros:** keep constraint compiler IR clippy-clean a8a853d
* **macros:** preserve shared tuple member order 364c76a
* **scoring:** restore zero-erasure shared grouped state 84907e6
* **solver:** complete required scalar assignment construction 507e381
* **solver:** notify compound scalar moves atomically cc2337b
* **solver:** use improving forager for assignment groups 5c4e188

## [0.14.1](///compare/v0.14.0...v0.14.1) (2026-05-16)


### Features

* **cvrp:** expose route metric classes for shared data bda7882
* **solver:** group Clarke-Wright savings by route metric class e633242


### Bug Fixes

* **solver:** match shared metric-class routes by owner feasibility 47edcf5

## [0.14.0](///compare/v0.13.1...v0.14.0) (2026-05-16)


### Features

* **scoring:** complement direct cross-join groups 088f19b, closes #60
* **solver:** unify owner-aware route hooks e4a9c0c


### Bug Fixes

* **scoring:** allow filtered join source streams bd9e227
* **scoring:** merge issue 60 and 61 stream fixes 36e3668, closes #60 #61
* **scoring:** preserve custom keyed join extractors 1a090ae
* **scoring:** preserve filtered flattened join targets ([#65](undefined/undefined/undefined/issues/65)) 5e260b4
* **solver:** bind Clarke-Wright merges to scored owner 338807d
* **solver:** preserve owner matching during route merges 7e71bc1

## [0.13.1](///compare/v0.13.0...v0.13.1) (2026-05-14)


### Features

* **scoring:** generalize grouped collectors 8e7cce3


### Bug Fixes

* **scoring:** preserve joined filter source indexes e70880f

## [0.13.0](///compare/v0.12.1...v0.13.0) (2026-05-12)


### ⚠ BREAKING CHANGES

* **scoring:** constraint streams no longer expose penalize_with, penalize_hard_with, penalize_hard, penalize_soft, reward_with, reward_hard_with, reward_hard, or reward_soft. Use penalize(HardSoftScore::ONE_HARD), penalize(HardSoftScore::ONE_SOFT), reward(score), or typed dynamic closures such as penalize(|task: &Task| HardSoftScore::of_soft(task.cost)).

* **scoring:** collapse constraint weighting methods 214f5b5


### Features

* **config:** add fair union selector ordering 748f6a8
* **facade:** export CollectedVec in the prelude 6aaa07d
* **macros:** restore constraint stream convenience traits e8cad8a
* **scoring:** add collect_vec collector 4a1b845
* **scoring:** add indexed presence collector c27725a
* **scoring:** expose explicit score weight wrappers 5b81e5c
* **scoring:** expose fresh score calculation e3501d5
* **scoring:** own collector values with retraction tokens 0332eca
* **solver:** add typed model-aware search defaults 0f588fc
* **solver:** assert cached scores in full assert mode 2371496
* **solver:** broaden grouped scalar assignment repairs 744a9de
* **solver:** interleave grouped assignment neighborhoods daa0137
* **solver:** make grouped assignment the scalar owner 4d81ed5
* **solver:** make grouped scalar and conflict repair streams bounded 036f882
* **solver:** make scalar and list selectors lazy under stream context 09e2c3c
* **solver:** route partitioned search through typed registration 4ef9935
* **solver:** synthesize default construction phases 99c824e
* **solver:** thread typed move stream context through selectors 2c9ef0d
* **solver:** tune grouped assignment default search e4b0ae9
* **solver:** use edge-local scalar assignment rules 094d9d7


### Bug Fixes

* **construction:** stop while generating live placements 2024a9e
* **macros:** reject non-usize scalar planning variables 4baba41
* **scoring:** align n-ary score helpers with tuple indices 1af54f6
* **scoring:** maintain collector item counts incrementally a3e9a06
* **scoring:** remove extractor clone from self joins 9e92e84
* **scoring:** support non-copy collect_vec elements 1341423
* **solver:** avoid accepted-count prefix bias in scalar defaults 3b9d80e
* **solver:** honor best score limit during active phases dde96b9
* **solver:** keep assignment scalars on grouped paths d5aad15
* **solver:** keep grouped scalar cleanup monomorphic 0ba2859
* **solver:** make accepted-count forager a horizon 098475c
* **solver:** make stock default search streaming-first 49f3f4b
* **solver:** preserve strict timeout semantics in local search 37e622d
* **solver:** replay exhaustive assignment paths cb61bc8
* **solver:** route omitted selectors through stock defaults 24e6387
* **solver:** score default grouped assignment construction 0d6592e

## [0.12.1](///compare/v0.12.0...v0.12.1) (2026-05-09)


### Features

* **solver:** fold coverage into scalar assignment groups af3826f
* **solver:** unify assignment-backed grouped scalar construction 6ac3ebf

## [0.12.0](///compare/v0.11.1...v0.12.0) (2026-05-08)


### ⚠ BREAKING CHANGES

* **scoring:** pass group keys to grouped weights
* **macros:** generated public list helper methods are no longer part of the user-facing model API. Code that called list_len_static(), element_count(), assign_element(), owner-prefixed list helpers, or related generated list mutation methods must move off those hidden runtime operations and use the supported planning model, descriptor, stream source, or solver APIs instead.
* **planning:** advanced direct users of solverforge-solver runtime assembly must migrate from ModelContext, VariableContext, ScalarVariableContext, ListVariableContext, ScalarGroupContext, ScalarGroupCandidate, ScalarGroupEdit, ConflictRepairProviderEntry, ConflictRepairSpec, and related provider names to RuntimeModel, VariableSlot, ScalarVariableSlot, ListVariableSlot, ScalarGroup/ScalarGroupBinding, ScalarCandidate, ScalarEdit, ConflictRepair, RepairCandidate, and RepairLimits. Macro users must use conflict_repairs = "path" instead of conflict_repair_providers = "path".

* **macros:** remove generated list helper shims 3e99299
* **scoring:** pass group keys to grouped weights df8b7bf


### Features

* **config:** add coverage phase and repair selector config 5566af8
* **examples:** add minimal shift scheduling solver 2eee8f9
* **planning:** publish declarative scalar planning contracts 649a7a0
* **scoring:** add consecutive run collector 08d736f
* **scoring:** clean public constraint stream surface d0232ce
* **solver:** add coverage first-fit construction bb1ace8
* **solver:** add coverage repair selector fe80935
* **solver:** bind model-owned coverage groups 0d917e6

## [0.11.1](///compare/v0.11.0...v0.11.1) (2026-05-05)


### Features

* **facade:** re-export solver configuration controls c2dc672


### Bug Fixes

* **release:** publish workspace crates only 17c7e7f

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
