# solverforge-config WIREFRAME

Serde-based configuration system for loading solver settings from TOML or YAML files.

**Location:** `crates/solverforge-config/`
**Workspace Release:** `0.8.11`

## Dependencies

- `solverforge-core` (path) — Core types (unused at runtime; version-aligned dependency)
- `serde` (workspace) — Serialization/deserialization
- `toml` (workspace) — TOML parsing
- `serde_yaml` (workspace) — YAML parsing
- `thiserror` (workspace) — Error derivation

## File Map

```
src/
├── lib.rs    — All public types, enums, config structs, error type, SolverConfig impl
├── tests.rs  — Unit tests for TOML/YAML parsing and builder API
```

## Public Re-exports (lib.rs)

Everything is defined directly in `lib.rs` — no submodules to re-export from. All structs and enums listed below are publicly available at crate root.

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
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |
| `k` | `usize` | `2` (for `ListKOpt`) |
| `termination` | `Option<TerminationConfig>` | `None` |

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

- scalar-only models: `ChangeMoveSelector`
- list-only models: `NearbyListChangeMoveSelector(20)`,
  `NearbyListSwapMoveSelector(20)`, `ListReverseMoveSelector`
- mixed models: the list defaults first, then scalar change

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

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `accepted_count_limit` | `Option<usize>` |
| `pick_early_type` | `Option<PickEarlyType>` |

### `TabuSearchConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `entity_tabu_size` | `Option<usize>` |
| `value_tabu_size` | `Option<usize>` |
| `move_tabu_size` | `Option<usize>` |
| `undo_move_tabu_size` | `Option<usize>` |

### `SimulatedAnnealingConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `starting_temperature` | `Option<String>` |

### `LateAcceptanceConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `late_acceptance_size` | `Option<usize>` |

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

### `SubListChangeMoveConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `min_sublist_size` | `usize` | `1` |
| `max_sublist_size` | `usize` | `3` |
| `entity_class` | `Option<String>` | `None` |
| `variable_name` | `Option<String>` | `None` |

### `SubListSwapMoveConfig`

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
| `variable_name` | `Option<String>` | `None` |

### `LimitedNeighborhoodConfig`

Derives: `Debug, Clone, Deserialize, Serialize`.

| Field | Type | Default |
|-------|------|---------|
| `selected_count_limit` | `usize` | |
| `selector` | `Box<MoveSelectorConfig>` | |

### `UnionMoveSelectorConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `selectors` | `Vec<MoveSelectorConfig>` |

### `CartesianProductConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `selectors` | `Vec<MoveSelectorConfig>` |

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
| `FirstFit` | **Default.** Standard variable first fit |
| `FirstFitDecreasing` | First fit by entity difficulty |
| `WeakestFit` | |
| `WeakestFitDecreasing` | |
| `StrongestFit` | |
| `StrongestFitDecreasing` | |
| `CheapestInsertion` | Greedy, basic variables |
| `AllocateEntityFromQueue` | |
| `AllocateToValueFromQueue` | |
| `ListRoundRobin` | List variable: even distribution |
| `ListCheapestInsertion` | List variable: minimize insertion cost |
| `ListRegretInsertion` | List variable: maximize regret |
| `ListClarkeWright` | List variable: greedy route merging by savings |
| `ListKOpt` | List variable: per-route k-opt polishing (k=2 = 2-opt) |

### `AcceptorConfig`

Derives: `Debug, Clone, Deserialize, Serialize`. Tagged `#[serde(tag = "type", rename_all = "snake_case")]`.

| Variant | Payload |
|---------|---------|
| `HillClimbing` | — |
| `TabuSearch` | `TabuSearchConfig` |
| `SimulatedAnnealing` | `SimulatedAnnealingConfig` |
| `LateAcceptance` | `LateAcceptanceConfig` |
| `GreatDeluge` | `GreatDelugeConfig` |

### `PickEarlyType`

Derives: `Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize`.

| Variant | Note |
|---------|------|
| `Never` | **Default** |
| `FirstBestScoreImproving` | |
| `FirstLastStepScoreImproving` | |

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
| `ListChangeMoveSelector` | `ListChangeMoveConfig` |
| `NearbyListChangeMoveSelector` | `NearbyListChangeMoveConfig` |
| `ListSwapMoveSelector` | `ListSwapMoveConfig` |
| `NearbyListSwapMoveSelector` | `NearbyListSwapMoveConfig` |
| `SubListChangeMoveSelector` | `SubListChangeMoveConfig` |
| `SubListSwapMoveSelector` | `SubListSwapMoveConfig` |
| `ListReverseMoveSelector` | `ListReverseMoveConfig` |
| `KOptMoveSelector` | `KOptMoveSelectorConfig` |
| `ListRuinMoveSelector` | `ListRuinMoveSelectorConfig` |
| `LimitedNeighborhood` | `LimitedNeighborhoodConfig` |
| `UnionMoveSelector` | `UnionMoveSelectorConfig` |
| `CartesianProductMoveSelector` | `CartesianProductConfig` |

## Architectural Notes

- **Pure data crate.** No solver logic — only serde structs and parsing. Consumed by `solverforge-solver` which maps these configs to runtime phase/acceptor/forager/selector enums.
- **All serde `rename_all = "snake_case"`** for consistent TOML/YAML key naming.
- **Internally-tagged enums** (`#[serde(tag = "type")]`) for `PhaseConfig`, `AcceptorConfig`, `MoveSelectorConfig` — the `type` field selects the variant.
- **`SolverConfigOverride` is not serde.** It exists for runtime termination overrides only.
- **No traits defined.** This crate defines no traits — it is a config schema consumed by other crates.
- **Builder pattern** via `with_*` methods returning `Self` on `SolverConfig`.
