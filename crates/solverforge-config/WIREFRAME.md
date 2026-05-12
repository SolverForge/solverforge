# solverforge-config WIREFRAME

Serde-based configuration system for loading solver settings from TOML or YAML files.

**Location:** `crates/solverforge-config/`
**Workspace Release:** `0.13.0`

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
├── phase.rs         — PhaseConfig plus construction/local-search/partitioned/custom configs
├── solver_config.rs — SolverConfig, SolverConfigOverride, environment/thread settings
├── termination.rs   — TerminationConfig
└── tests.rs         — Test module root
    └── tests/*.rs   — Unit tests for TOML/YAML parsing, selector config, and roundtrips
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
| `construction_obligation` | `ConstructionObligation` | `PreserveUnassigned` |
| `target` | `VariableTargetConfig` | empty target |
| `k` | `usize` | `2` (for `ListKOpt`) |
| `value_candidate_limit` | `Option<usize>` | `None` |
| `group_name` | `Option<String>` | `None` |
| `group_candidate_limit` | `Option<usize>` | `None` |
| `termination` | `Option<TerminationConfig>` | `None` |

`target` is flattened in serde, so configuration files still use top-level
`entity_class = "..."` and `variable_name = "..."` keys when targeting one
planning variable family.

`construction_obligation` controls nullable scalar construction and required
assignment-backed scalar construction. The default `preserve_unassigned` allows
an optional scalar slot to remain unassigned when the current unassigned state
is legal. `assign_when_candidate_exists` forces construction to assign a doable
candidate when one exists, even if the unassigned baseline scores better.

`group_name` selects a named model-provided `ScalarGroup`. Candidate-backed
grouped construction evaluates and applies each candidate's scalar edits
atomically; assignment-backed construction generates stock nullable scalar
assignment candidates and routes them through the same grouped selection
engine. Config limits override model-owned `ScalarGroup::with_limits` values;
model-owned values apply when config omits the field. `group_candidate_limit`
is construction-only: it is passed through in the effective grouped limits and
caps normalized candidates after framework legality, duplicate, frontier, and
no-op filtering. Grouped local-search selectors do not use
`group_candidate_limit`; they use config `max_moves_per_step` first and then
fall back to model-owned `ScalarGroup::with_limits` values for the total move
cap.

When a scalar target belongs to an assignment-backed `ScalarGroup`, runtime
construction rejects explicit scalar construction targets that omit the owning
`group_name`. Use grouped scalar construction for assignment-owned scalar
slots; generic scalar construction remains single-slot and only covers
non-assignment-owned scalar variables.

### `LocalSearchConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `local_search_type` | `LocalSearchType` |
| `acceptor` | `Option<AcceptorConfig>` |
| `forager` | `Option<ForagerConfig>` |
| `move_selector` | `Option<MoveSelectorConfig>` |
| `neighborhoods` | `Vec<MoveSelectorConfig>` |
| `termination` | `Option<TerminationConfig>` |

`local_search_type` defaults to `AcceptorForager`. `AcceptorForager` uses
`acceptor`, `forager`, and `move_selector`, and rejects `neighborhoods`.
`VariableNeighborhoodDescent` uses ordered `neighborhoods`, and rejects
`acceptor`, `forager`, and `move_selector`.

When `phases` is omitted, the solver runtime uses the canonical model-aware
default profile, including construction and any selected built-in search phases.
When only `move_selector` is omitted for an explicit `AcceptorForager` local
search phase, the canonical runtime resolves model-aware selector defaults from
declared runtime capabilities. Nearby scalar
change/swap selectors are used before plain scalar fallback selectors when the
model declares nearby hooks, and plain scalar fallback remains present for every
non-assignment-owned scalar slot. Scalar groups add grouped-scalar selectors. List
models use nearby list change/swap, sublist change/swap, reverse, k-opt when
k-opt hooks exist, and list ruin when the list runtime supports ruin moves.
Conflict repair selectors are added only when repair providers are registered.
Assignment-owned scalar variables stay on their grouped scalar selector path:
plain scalar selector defaults and conflict-repair defaults exclude them.
Omitted acceptor and forager settings are selected by the same model-aware
profile. Explicit `acceptor`, `forager`, `move_selector`, and VND
`neighborhoods` remain exact user-owned configuration.

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
| `AcceptedCount` | `AcceptedCountForagerConfig` | Stop after `limit` accepted moves and pick the best among them |
| `BestScore` | — | Evaluate the full neighborhood and pick the best accepted move |
| `FirstAccepted` | — | Stop on the first accepted move |
| `FirstBestScoreImproving` | — | Stop on the first move improving the phase-best score |
| `FirstLastStepScoreImproving` | — | Stop on the first move improving the previous step score |

### `AcceptedCountForagerConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Default |
|-------|------|---------|
| `limit` | `Option<usize>` | `None` |

The accepted-count forager stops the current step after collecting `limit`
accepted moves, then selects the best candidate among that step horizon.
Use `BestScore` for a full-neighborhood greedy scan.

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
| `value_candidate_limit` | `Option<usize>` |
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
| `value_candidate_limit` | `Option<usize>` | `None` |
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
| `value_candidate_limit` | `Option<usize>` | `None` |
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

### `GroupedScalarMoveSelectorConfig`

Derives: `Debug, Clone, Deserialize, Serialize, PartialEq, Eq`.

| Field | Type | Default |
|-------|------|---------|
| `group_name` | `String` | required |
| `value_candidate_limit` | `Option<usize>` | `None` |
| `max_moves_per_step` | `Option<usize>` | `None` |
| `require_hard_improvement` | `bool` | `false` |

`value_candidate_limit` is provider-defined per assignment or value-source
work, while `max_moves_per_step` caps the total grouped local-search moves
generated for one selector step. Config values override model-owned
`ScalarGroup::with_limits` values for `value_candidate_limit` and
`max_moves_per_step`; model-owned limits apply when config omits them. When
`require_hard_improvement` is true, each emitted grouped compound move carries
the shared hard-improvement gate used by compound repair and cartesian moves.

Assignment-backed `grouped_scalar_move_selector` emits compound scalar
assignment moves from a named scalar group. It tries unassigned required
entities first, then capacity conflicts, bounded reassignments, and bounded
sequence/position rematches. When `require_hard_improvement` is true, emitted
moves carry the same hard-improvement gate used by other compound repair
selectors.

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

Enum: `Sequential` (default), `RoundRobin`, `RotatingRoundRobin`,
`StratifiedRandom`.

### `CartesianProductConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `require_hard_improvement` | `bool` |
| `selectors` | `Vec<MoveSelectorConfig>` |

Runtime note: cartesian selectors compose children in selector order. The left
child is previewed first, the right child is opened against that preview state,
and the runtime rejects left-child previews that require full score evaluation.
When `require_hard_improvement` is true, the composed candidate carries the
same hard-improvement gate used by compound repair moves.

### `ConflictRepairMoveSelectorConfig`

Derives: `Debug, Clone, Deserialize, Serialize, PartialEq, Eq`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `constraints` | `Vec<String>` | `[]` |
| `max_matches_per_step` | `usize` | `16` |
| `max_repairs_per_match` | `usize` | `32` |
| `max_moves_per_step` | `usize` | `256` |
| `require_hard_improvement` | `bool` | `false` |
| `include_soft_matches` | `bool` | `false` |

Runtime note: configured constraints must match scoring constraint metadata
before providers are invoked. With `include_soft_matches = false`, non-hard
scoring constraints are rejected; setting it to `true` explicitly allows soft
repair providers. Conflict repair moves operate only on
non-assignment-owned scalar variables; assignment-backed scalar slots must be
repaired through their owning grouped scalar selector.

### `CompoundConflictRepairMoveSelectorConfig`

Derives: `Debug, Clone, Deserialize, Serialize, PartialEq, Eq`. Manual `Default`.

| Field | Type | Default |
|-------|------|---------|
| `constraints` | `Vec<String>` | `[]` |
| `max_matches_per_step` | `usize` | `16` |
| `max_repairs_per_match` | `usize` | `32` |
| `max_moves_per_step` | `usize` | `256` |
| `require_hard_improvement` | `bool` | `true` |
| `include_soft_matches` | `bool` | `false` |

### `PartitionedSearchConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type | Default |
|-------|------|---------|
| `partitioner` | `Option<String>` | `None` |
| `thread_count` | `MoveThreadCount` | `Auto` |
| `log_progress` | `bool` | `false` |
| `child_phases` | `Vec<PhaseConfig>` | `[]` |
| `termination` | `Option<TerminationConfig>` | `None` |

### `CustomPhaseConfig`

Derives: `Debug, Clone, Default, Deserialize, Serialize`.

| Field | Type |
|-------|------|
| `name` | `String` |

`name` selects a custom phase that was compiled into the solution through its
typed `search = "path::to::search"` function. Missing `name` is a TOML parse
error. SolverForge does not load arbitrary runtime classes or maintain a
string-to-erased-plugin registry.

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
| `PartitionedSearch` | `PartitionedSearchConfig` |
| `Custom` | `CustomPhaseConfig` |

### `ConstructionHeuristicType`

Derives: `Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize`.

| Variant | Note |
|---------|------|
| `FirstFit` | **Default.** Generic first-fit construction over mixed or list-bearing runtime plans; scalar-only targets reuse the descriptor path |
| `FirstFitDecreasing` | Specialized scalar-only first fit by entity difficulty; validates `construction_entity_order_key` |
| `WeakestFit` | Specialized scalar-only weakest-fit heuristic; validates `construction_value_order_key` |
| `WeakestFitDecreasing` | Specialized scalar-only weakest-fit-by-difficulty heuristic; validates both scalar order-key hooks |
| `StrongestFit` | Specialized scalar-only strongest-fit heuristic; validates `construction_value_order_key` |
| `StrongestFitDecreasing` | Specialized scalar-only strongest-fit-by-difficulty heuristic; validates both scalar order-key hooks |
| `CheapestInsertion` | Generic best-score construction over mixed or list-bearing runtime plans; scalar-only targets reuse the descriptor path |
| `AllocateEntityFromQueue` | Specialized scalar-only queue-driven allocation; validates `construction_entity_order_key` |
| `AllocateToValueFromQueue` | Specialized scalar-only value-queue allocation; validates `construction_value_order_key` |
| `ListRoundRobin` | Specialized list-only even distribution; validates the targeted list variable exists before phase build |
| `ListCheapestInsertion` | Specialized list-only score-minimizing insertion; validates the targeted list variable exists before phase build |
| `ListRegretInsertion` | Specialized list-only highest-regret insertion; validates the targeted list variable exists before phase build |
| `ListClarkeWright` | Specialized list-only greedy route merging by savings; validates required `cw_*` hooks before phase build |
| `ListKOpt` | Specialized list-only per-route k-opt polishing (k=2 = 2-opt); validates required `k_opt_*` hooks before phase build |

When `group_name` is set, grouped scalar construction supports
`FirstFit`, `FirstFitDecreasing`, `CheapestInsertion`, `WeakestFit`,
`WeakestFitDecreasing`, `StrongestFit`, and `StrongestFitDecreasing`.
Candidate-backed groups provide ordering metadata on `ScalarCandidate` values.
Assignment-backed groups use `ScalarGroup::with_entity_order` for decreasing
variants and `ScalarGroup::with_value_order` for weakest/strongest variants;
missing required hooks are rejected before phase execution.
Grouped queue construction with `AllocateEntityFromQueue` or
`AllocateToValueFromQueue` is rejected until the grouped queue contract is
explicit.

### `ConstructionObligation`

Derives: `Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize`.

| Variant | Note |
|---------|------|
| `PreserveUnassigned` | **Default.** Nullable scalar construction may keep the current unassigned value when legal |
| `AssignWhenCandidateExists` | Nullable scalar construction and required scalar assignment construction must assign a doable candidate when one exists |

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
| `GroupedScalarMoveSelector` | `GroupedScalarMoveSelectorConfig` |
| `ListChangeMoveSelector` | `ListChangeMoveConfig` |
| `NearbyListChangeMoveSelector` | `NearbyListChangeMoveConfig` |
| `ListSwapMoveSelector` | `ListSwapMoveConfig` |
| `NearbyListSwapMoveSelector` | `NearbyListSwapMoveConfig` |
| `SublistChangeMoveSelector` | `SublistChangeMoveConfig` |
| `SublistSwapMoveSelector` | `SublistSwapMoveConfig` |
| `ListReverseMoveSelector` | `ListReverseMoveConfig` |
| `KOptMoveSelector` | `KOptMoveSelectorConfig` |
| `ListRuinMoveSelector` | `ListRuinMoveSelectorConfig` |
| `ConflictRepairMoveSelector` | `ConflictRepairMoveSelectorConfig` |
| `CompoundConflictRepairMoveSelector` | `CompoundConflictRepairMoveSelectorConfig` |
| `LimitedNeighborhood` | `LimitedNeighborhoodConfig` |
| `UnionMoveSelector` | `UnionMoveSelectorConfig` |
| `CartesianProductMoveSelector` | `CartesianProductConfig` |

Decorator notes:
- `LimitedNeighborhood` wraps the child cursor at neighborhood construction time, so candidate generation stops at `selected_count_limit` while preserving the wrapped selector order.
- `CartesianProductMoveSelector` uses sequential preview, selector-order tabu ids, and selected-winner materialization rather than exposing an owned composite iterator.
- `ConflictRepairMoveSelector` validates configured constraint names and hardness through director constraint metadata before opening repair providers.
- Scalar `ChangeMoveSelector`, `NearbyChangeMoveSelector`, `PillarChangeMoveSelector`, and `RuinRecreateMoveSelector` accept `value_candidate_limit` for bounded scalar value candidate generation. Scalar `cheapest_insertion` requires either `candidate_values` on the model or this limit in config.

## Architectural Notes

- **Pure data crate.** No solver logic — only serde structs and parsing. Consumed by `solverforge-solver` which maps these configs to runtime phase/acceptor/forager/selector enums.
- **All serde `rename_all = "snake_case"`** for consistent TOML/YAML key naming.
- **Internally-tagged enums** (`#[serde(tag = "type")]`) for `PhaseConfig`, `AcceptorConfig`, `MoveSelectorConfig` — the `type` field selects the variant.
- **`SolverConfigOverride` is not serde.** It exists for runtime termination overrides only.
- **No traits defined.** This crate defines no traits — it is a config schema consumed by other crates.
- **Builder pattern** via `with_*` methods returning `Self` on `SolverConfig`.
