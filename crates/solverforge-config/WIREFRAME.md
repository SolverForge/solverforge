# solverforge-config WIREFRAME

Serde-based configuration system for loading solver settings from TOML or YAML files.

**Location:** `crates/solverforge-config/`
**Workspace Release:** `0.9.2`

## Dependencies

- `solverforge-core` (path) — Core types (unused at runtime; version-aligned dependency)
- `serde` (workspace) — Serialization/deserialization
- `toml` (workspace) — TOML parsing
- `serde_yaml` (workspace) — YAML parsing
- `thiserror` (workspace) — Error derivation

## File Map

```
src/
├── lib.rs           — Private module declarations and crate-root re-exports
├── acceptor.rs      — AcceptorConfig and acceptor-specific config structs
├── director.rs      — DirectorConfig
├── error.rs         — ConfigError
├── forager.rs       — ForagerConfig and AcceptedCountForagerConfig
├── move_selector.rs — MoveSelectorConfig and selector-specific config structs
├── phase.rs         — PhaseConfig plus construction/local-search/VND/exhaustive configs
├── solver_config.rs — SolverConfig, SolverConfigOverride, environment/thread settings
├── termination.rs   — TerminationConfig
└── tests.rs         — Unit tests for TOML/YAML parsing and builder API
```

## Public Re-exports (lib.rs)

Implementation is split by configuration family. The modules stay private, and
`lib.rs` re-exports the public structs and enums listed below at crate root.

## Error Type

### `ConfigError`

```rust
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Yaml(serde_yaml::Error),
    Invalid(String),
}
```

Derives: `Debug`, `Error` (thiserror).

## Public Structs

### `SolverConfig`

Top-level solver configuration. Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Default | Note |
|-------|------|---------|------|
| `environment_mode` | `EnvironmentMode` | `NonReproducible` | `#[serde(default)]` |
| `random_seed` | `Option<u64>` | `None` | |
| `move_thread_count` | `MoveThreadCount` | `Auto` | `#[serde(default)]` |
| `termination` | `Option<TerminationConfig>` | `None` | |
| `score_director` | `Option<DirectorConfig>` | `None` | |
| `phases` | `Vec<PhaseConfig>` | `[]` | `#[serde(default)]` |

**Methods:**

| Method | Signature | Note |
|--------|-----------|------|
| `new` | `fn() -> Self` | Alias for `Default::default()` |
| `load` | `fn(path: impl AsRef<Path>) -> Result<Self, ConfigError>` | Delegates to `from_toml_file` |
| `from_toml_file` | `fn(path: impl AsRef<Path>) -> Result<Self, ConfigError>` | Reads file, parses TOML |
| `from_toml_str` | `fn(s: &str) -> Result<Self, ConfigError>` | Parses TOML string |
| `from_yaml_file` | `fn(path: impl AsRef<Path>) -> Result<Self, ConfigError>` | Reads file, parses YAML |
| `from_yaml_str` | `fn(s: &str) -> Result<Self, ConfigError>` | Parses YAML string |
| `with_termination_seconds` | `fn(self, seconds: u64) -> Self` | Builder: sets seconds_spent_limit |
| `with_random_seed` | `fn(self, seed: u64) -> Self` | Builder: sets random_seed |
| `with_phase` | `fn(self, phase: PhaseConfig) -> Self` | Builder: appends phase |
| `time_limit` | `fn(&self) -> Option<Duration>` | Convenience: delegates to termination |

### `TerminationConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Note |
|-------|------|------|
| `seconds_spent_limit` | `Option<u64>` | Max seconds |
| `minutes_spent_limit` | `Option<u64>` | Max minutes |
| `best_score_limit` | `Option<String>` | Target score as string (e.g., `"0hard/0soft"`) |
| `step_count_limit` | `Option<u64>` | Max steps |
| `unimproved_step_count_limit` | `Option<u64>` | Max unimproved steps |
| `unimproved_seconds_spent_limit` | `Option<u64>` | Max seconds without improvement |

**Methods:**

| Method | Signature | Note |
|--------|-----------|------|
| `time_limit` | `fn(&self) -> Option<Duration>` | Combines seconds + minutes × 60 |
| `unimproved_time_limit` | `fn(&self) -> Option<Duration>` | Maps unimproved seconds to Duration |

### `DirectorConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Note |
|-------|------|------|
| `constraint_provider` | `Option<String>` | Fully qualified constraint provider name |
| `constraint_match_enabled` | `bool` | Default: `false` |

### `ConstructionHeuristicConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Default |
|-------|------|---------|
| `construction_heuristic_type` | `ConstructionHeuristicType` | `FirstFit` |
| `target` | `VariableTargetConfig` | empty target |
| `k` | `usize` | `2` (for `ListKOpt`) |
| `value_candidate_limit` | `Option<usize>` | `None` |
| `termination` | `Option<TerminationConfig>` | `None` |

`target` is flattened in serde, so configuration files still use top-level
`entity_class = "..."` and `variable_name = "..."` keys when targeting one
planning variable family.

### `LocalSearchConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `acceptor` | `Option<AcceptorConfig>` |
| `forager` | `Option<ForagerConfig>` |
| `move_selector` | `Option<MoveSelectorConfig>` |
| `termination` | `Option<TerminationConfig>` |

When `move_selector` is omitted, the canonical runtime resolves explicit
streaming defaults rather than broad exhaustive search:

- scalar-only models: `ChangeMoveSelector`, then `SwapMoveSelector`
- list-only models: `NearbyListChangeMoveSelector(20)`,
  `NearbyListSwapMoveSelector(20)`, `ListReverseMoveSelector`
- mixed models: the list defaults first, then the scalar defaults

When `forager` is omitted, the canonical runtime uses the accepted-count
forager with `limit = 1`.

### `VndConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `neighborhoods` | `Vec<MoveSelectorConfig>` |
| `termination` | `Option<TerminationConfig>` |

### `VariableTargetConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq`.

| Field | Type |
|-------|------|
| `entity_class` | `Option<String>` |
| `variable_name` | `Option<String>` |

### `ForagerConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Tagged `#[serde(tag = "type", rename_all = "snake_case")]`.

| Variant | Payload | Note |
|---------|---------|------|
| `AcceptedCount` | `AcceptedCountForagerConfig` | Retain up to `limit` accepted moves and pick the best |
| `BestScore` | — | Evaluate the full neighborhood and pick the best accepted move |
| `FirstAccepted` | — | Stop on the first accepted move |
| `FirstBestScoreImproving` | — | Stop on the first move improving the phase-best score |
| `FirstLastStepScoreImproving` | — | Stop on the first move improving the previous step score |

### `AcceptedCountForagerConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Default |
|-------|------|---------|
| `limit` | `Option<usize>` | `None` |

The accepted-count forager keeps up to `limit` accepted moves for final
selection. It does not imply early neighborhood exit.

### `TabuSearchConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `entity_tabu_size` | `Option<usize>` |
| `value_tabu_size` | `Option<usize>` |
| `move_tabu_size` | `Option<usize>` |
| `undo_move_tabu_size` | `Option<usize>` |
| `aspiration_enabled` | `Option<bool>` |

Normalization notes:
- `acceptor = { type = "tabu_search" }` normalizes to move-tabu-only with `move_tabu_size = 10` and `aspiration_enabled = true`.
- Any explicit `*_tabu_size = 0` is rejected during solver build.

### `StepCountingHillClimbingConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `step_count_limit` | `Option<u64>` |

### `SimulatedAnnealingConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `level_temperatures` | `Option<Vec<f64>>` |
| `decay_rate` | `Option<f64>` |
| `hill_climbing_temperature` | `Option<f64>` |
| `hard_regression_policy` | `Option<HardRegressionPolicyConfig>` |
| `calibration` | `Option<SimulatedAnnealingCalibrationConfig>` |

### `HardRegressionPolicyConfig`

Enum: `TemperatureControlled` (default), `NeverAcceptHardRegression`.

### `SimulatedAnnealingCalibrationConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `sample_size` | `Option<usize>` |
| `target_acceptance_probability` | `Option<f64>` |
| `fallback_temperature` | `Option<f64>` |

### `LateAcceptanceConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `late_acceptance_size` | `Option<usize>` |

### `DiversifiedLateAcceptanceConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `late_acceptance_size` | `Option<usize>` |
| `tolerance` | `Option<f64>` |

### `GreatDelugeConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `water_level_increase_ratio` | `Option<f64>` |

### `ChangeMoveConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `entity_class` | `Option<String>` |
| `variable_name` | `Option<String>` |

### `SwapMoveConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `entity_class` | `Option<String>` |
| `variable_name` | `Option<String>` |

### `ListChangeMoveConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Note |
|-------|------|------|
| `entity_class` | `Option<String>` | Filter by entity class |
| `variable_name` | `Option<String>` | Filter by list variable name |

### `NearbyListChangeMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default` (max_nearby = 10).

| Field | Type | Default |
|-------|------|---------|
| `max_nearby` | `usize` | `10` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `ListSwapMoveConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `entity_class` | `Option<String>` |
| `variable_name` | `Option<String>` |

### `NearbyListSwapMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default` (max_nearby = 10).

| Field | Type | Default |
|-------|------|---------|
| `max_nearby` | `usize` | `10` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `SublistChangeMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `min_sublist_size` | `usize` | `1` |
| `max_sublist_size` | `usize` | `3` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `SublistSwapMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `min_sublist_size` | `usize` | `1` |
| `max_sublist_size` | `usize` | `3` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `ListReverseMoveConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `entity_class` | `Option<String>` |
| `variable_name` | `Option<String>` |

### `NearbyChangeMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `max_nearby` | `usize` | `10` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `NearbySwapMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `max_nearby` | `usize` | `10` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `PillarChangeMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `minimum_sub_pillar_size` | `usize` | `0` (`0/0` means full pillars only) |
| `maximum_sub_pillar_size` | `usize` | `0` (`0/0` means full pillars only) |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `PillarSwapMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `minimum_sub_pillar_size` | `usize` | `0` (`0/0` means full pillars only) |
| `maximum_sub_pillar_size` | `usize` | `0` (`0/0` means full pillars only) |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `KOptMoveSelectorConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `k` | `usize` | `3` |
| `min_segment_len` | `usize` | `1` |
| `max_nearby` | `usize` | `0` (full enumeration; >0 enables distance-pruned `NearbyKOptMoveSelector`) |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `ListRuinMoveSelectorConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `min_ruin_count` | `usize` | `2` |
| `max_ruin_count` | `usize` | `5` |
| `moves_per_step` | `Option<usize>` | `None` |
| `entity_class` | `Option<String>` | `None` |
| `value_candidate_limit` | `Option<usize>` | `None` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `RecreateHeuristicType`

Enum: `FirstFit` (default), `CheapestInsertion`.

### `RuinRecreateMoveSelectorConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `min_ruin_count` | `usize` | `2` |
| `max_ruin_count` | `usize` | `5` |
| `moves_per_step` | `Option<usize>` | `None` |
| `value_candidate_limit` | `Option<usize>` | `None` |
| `recreate_heuristic_type` | `RecreateHeuristicType` | `FirstFit` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `LimitedNeighborhoodConfig`

Derives: `Debug, Clone, Deserialize, Serialize`.

| Field | Type | Default |
|-------|------|---------|
| `selected_count_limit` | `usize` | |
| `selector` | `Box<MoveSelectorConfig>` | |

### `UnionMoveSelectorConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Default |
|-------|------|---------|
| `selection_order` | `UnionSelectionOrder` | `Sequential` |
| `selectors` | `Vec<MoveSelectorConfig>` |

### `UnionSelectionOrder`

Enum: `Sequential` (default), `RoundRobin`.

### `CartesianProductConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `selectors` | `Vec<MoveSelectorConfig>` |

Runtime note: cartesian selectors compose children in selector order. The left
child is previewed first, the right child is opened against that preview state,
and the runtime rejects left-child previews that require full score evaluation.

### `ExhaustiveSearchConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Default |
|-------|------|---------|
| `exhaustive_search_type` | `ExhaustiveSearchType` | `BranchAndBound` |
| `termination` | `Option<TerminationConfig>` | `None` |

### `PartitionedSearchConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `partition_count` | `Option<usize>` |
| `termination` | `Option<TerminationConfig>` |

### `CustomPhaseConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `custom_phase_class` | `Option<String>` |

### `SolverConfigOverride`

Derives: `Debug, Clone, Default`. **Not** serde — runtime-only.

| Field | Type |
|-------|------|
| `termination` | `Option<TerminationConfig>` |

**Methods:**

| Method | Signature |
|--------|-----------|
| `with_termination` | `fn(termination: TerminationConfig) -> Self` |

## Public Enums

### `EnvironmentMode`

Derives: `Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize`. Tagged `#[serde(rename_all = "snake_case")]`.

| Variant | Note |
|---------|------|
| `NonReproducible` | **Default.** Minimal overhead |
| `Reproducible` | Deterministic behavior |
| `FastAssert` | Basic assertions |
| `FullAssert` | Comprehensive assertions |

### `MoveThreadCount`

Derives: `Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize`.

| Variant | Note |
|---------|------|
| `Auto` | **Default.** Auto-detect thread count |
| `None` | Single-threaded |
| `Count(usize)` | Explicit count |

### `PhaseConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Tagged `#[serde(tag = "type", rename_all = "snake_case")]`.

| Variant | Payload |
|---------|---------|
| `ConstructionHeuristic` | `ConstructionHeuristicConfig` |
| `LocalSearch` | `LocalSearchConfig` |
| `ExhaustiveSearch` | `ExhaustiveSearchConfig` |
| `PartitionedSearch` | `PartitionedSearchConfig` |
| `Custom` | `CustomPhaseConfig` |

### `ConstructionHeuristicType`

Derives: `Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize`.

| Variant | Note |
|---------|------|
| `FirstFit` | **Default.** Generic first-fit construction over mixed or list-bearing `ModelContext` targets; pure scalar targets reuse the descriptor-scalar path |
| `FirstFitDecreasing` | Specialized scalar-only first fit by entity difficulty; validates `construction_entity_order_key` |
| `WeakestFit` | Specialized scalar-only weakest-fit heuristic; validates `construction_value_order_key` |
| `WeakestFitDecreasing` | Specialized scalar-only weakest-fit-by-difficulty heuristic; validates both scalar order-key hooks |
| `StrongestFit` | Specialized scalar-only strongest-fit heuristic; validates `construction_value_order_key` |
| `StrongestFitDecreasing` | Specialized scalar-only strongest-fit-by-difficulty heuristic; validates both scalar order-key hooks |
| `CheapestInsertion` | Generic best-score construction over mixed or list-bearing `ModelContext` targets; pure scalar targets reuse the descriptor-scalar path |
| `AllocateEntityFromQueue` | Specialized scalar-only queue-driven allocation; validates `construction_entity_order_key` |
| `AllocateToValueFromQueue` | Specialized scalar-only value-queue allocation; validates `construction_value_order_key` |
| `ListRoundRobin` | Specialized list-only even distribution; validates the targeted list variable exists before phase build |
| `ListCheapestInsertion` | Specialized list-only score-minimizing insertion; validates the targeted list variable exists before phase build |
| `ListRegretInsertion` | Specialized list-only highest-regret insertion; validates the targeted list variable exists before phase build |
| `ListClarkeWright` | Specialized list-only greedy route merging by savings; validates required `cw_*` hooks before phase build |
| `ListKOpt` | Specialized list-only per-route k-opt polishing (k=2 = 2-opt); validates required `k_opt_*` hooks before phase build |

### `AcceptorConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Tagged `#[serde(tag = "type", rename_all = "snake_case")]`.

| Variant | Payload |
|---------|---------|
| `HillClimbing` | — |
| `TabuSearch` | `TabuSearchConfig` |
| `StepCountingHillClimbing` | `StepCountingHillClimbingConfig` |
| `SimulatedAnnealing` | `SimulatedAnnealingConfig` |
| `LateAcceptance` | `LateAcceptanceConfig` |
| `DiversifiedLateAcceptance` | `DiversifiedLateAcceptanceConfig` |
| `GreatDeluge` | `GreatDelugeConfig` |

### `ExhaustiveSearchType`

Derives: `Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize`.

| Variant | Note |
|---------|------|
| `BranchAndBound` | **Default** |
| `BruteForce` | |

### `MoveSelectorConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Tagged `#[serde(tag = "type", rename_all = "snake_case")]`.

| Variant | Payload |
|---------|---------|
| `ChangeMoveSelector` | `ChangeMoveConfig` |
| `SwapMoveSelector` | `SwapMoveConfig` |
| `NearbyChangeMoveSelector` | `NearbyChangeMoveConfig` |
| `NearbySwapMoveSelector` | `NearbySwapMoveConfig` |
| `PillarChangeMoveSelector` | `PillarChangeMoveConfig` |
| `PillarSwapMoveSelector` | `PillarSwapMoveConfig` |
| `RuinRecreateMoveSelector` | `RuinRecreateMoveSelectorConfig` |
| `ListChangeMoveSelector` | `ListChangeMoveConfig` |
| `NearbyListChangeMoveSelector` | `NearbyListChangeMoveConfig` |
| `ListSwapMoveSelector` | `ListSwapMoveConfig` |
| `NearbyListSwapMoveSelector` | `NearbyListSwapMoveConfig` |
| `SublistChangeMoveSelector` | `SublistChangeMoveConfig` |
| `SublistSwapMoveSelector` | `SublistSwapMoveConfig` |
| `ListReverseMoveSelector` | `ListReverseMoveConfig` |
| `KOptMoveSelector` | `KOptMoveSelectorConfig` |
| `ListRuinMoveSelector` | `ListRuinMoveSelectorConfig` |
| `LimitedNeighborhood` | `LimitedNeighborhoodConfig` |
| `UnionMoveSelector` | `UnionMoveSelectorConfig` |
| `CartesianProductMoveSelector` | `CartesianProductConfig` |

Decorator notes:
- `LimitedNeighborhood` caps yielded candidates while preserving the wrapped selector order.
- `CartesianProductMoveSelector` uses sequential preview, selector-order tabu ids, and selected-winner materialization rather than exposing an owned composite iterator.
- Scalar `ChangeMoveSelector`, `NearbyChangeMoveSelector`, `PillarChangeMoveSelector`, and `RuinRecreateMoveSelector` accept `value_candidate_limit` for bounded scalar value candidate generation. Scalar `cheapest_insertion` requires either `candidate_values` on the model or this limit in config.

## Architectural Notes

- **Pure data crate.** No solver logic — only serde structs and parsing. Consumed by `solverforge-solver` which maps these configs to runtime phase/acceptor/forager/selector enums.
- **All serde `rename_all = "snake_case"`** for consistent TOML/YAML key naming.
- **Internally-tagged enums** (`#[serde(tag = "type")]`) for `PhaseConfig`, `AcceptorConfig`, `MoveSelectorConfig` — the `type` field selects the variant.
- **`SolverConfigOverride` is not serde.** It exists for runtime termination overrides only.
- **No traits defined.** This crate defines no traits — it is a config schema consumed by other crates.
- **Builder pattern** via `with_*` methods returning `Self` on `SolverConfig`.
