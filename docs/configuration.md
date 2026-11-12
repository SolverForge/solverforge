# Configuration

Solver behavior lives in `solver.toml`: termination limits, phases, selectors
and score-director options. Omit `phases` and SolverForge picks defaults from
the model's shape. This guide covers loading the file, termination, constraint
providers and the defaults. Custom phases, selector routing and partitioned
search are in [Extend the solver](extend-solver.md); every key is listed in the
[`solverforge-config` wireframe](../crates/solverforge-config/WIREFRAME.md).

## Loading `solver.toml`

```toml
[termination]
seconds_spent_limit = 30
unimproved_seconds_spent_limit = 5
step_count_limit = 10000
```

Parsing is strict. A misspelled or unknown key anywhere in the file (for
example `secnds_spent_limit`) fails with a `ConfigError::Toml` that names the
key. It is never silently ignored.

Load it yourself and handle the `Result`:

```rust
let config = SolverConfig::load("solver.toml")?;
```

Every configuration type is reachable as `solverforge::config::*`. To solve on
the calling thread with exactly that config, without a `SolverManager`:

```rust
let solved = plan.solve_with_config(config)?;
```

### Config embedded in the model

Macro-generated retained solves (the `SolverManager` path) read configuration
only from `#[planning_solution(solver_toml = "...")]`. The file is embedded at
compile time, and the path is relative to the solution's source file. Without
`solver_toml` they start from `SolverConfig::default()`. Nothing is read from
the working directory. An embedded file that does not parse fails the job with
a `SolverEvent::Failed` whose error reads
`solver configuration invalid: TOML parse error ...`.

The solution module listed by `planning_model!` can add `config = "..."` to
adjust that base config per solution instead of replacing it:

```rust
#[planning_solution(
    constraints = "define_constraints",
    config = "solver_config_for_solution"
)]
pub struct Schedule {
    #[problem_fact_collection]
    pub nurses: Vec<Nurse>,

    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    pub time_limit_secs: u64,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn solver_config_for_solution(solution: &Schedule, config: SolverConfig) -> SolverConfig {
    config.with_termination_seconds(solution.time_limit_secs)
}
```

## Termination is always binding

Configured time, step, move, score-calculation and phase limits hold during
all construction work, required scalar and list construction included. A
configured solve publishes a best solution only after every mandatory list
element, required assignment row and non-optional scalar variable is assigned.
If a limit is reached first, the solve ends as `Failed`. An incomplete
construction state is never emitted as a best or completed solution.

## Choosing a constraint set

Next to `constraints`, a solution can declare further constraint sets with
`constraint_provider = "path"` (repeatable). A configuration picks one by the
path exactly as written in the attribute:

```toml
[score_director]
constraint_provider = "relaxed_constraints"
constraint_match_enabled = true
```

- Without `[score_director]`, the solve scores with `constraints`.
- A name that matches no declared provider fails the solve before it starts
  with `RuntimeBuildError::UnknownConstraintProvider`.
- `SolverManager::analyze_snapshot` analyzes a job's snapshots with the
  provider that job selected.
- `constraint_match_enabled = false` turns snapshot analysis off for the job;
  analysis then fails with `SolverManagerError::SnapshotAnalysisDisabled`.

## What runs when you omit `phases`

### Construction

Construction always runs first and is state-aware:

- list variables use the matching specialized list construction;
- assignment-backed scalar groups commit the hard-first required allocation
  through named grouped `FirstFit`, then run grouped `CheapestInsertion` for
  optional slots;
- remaining scalar variables that no assignment group owns use
  descriptor-backed single-slot construction.

The dense required batch collects independent assignments directly. Bounded
augmenting rematches stay with the following required cursor rather than being
explored inside a batch that cannot retain multi-entity edits. Keeping an
optional assignment unassigned completes that decision without ending the
optional pass. Completed optional roots are skipped before candidate-domain
reads or move generation, so the candidate budget stays available for later
rows.

An explicit scalar construction target that names an assignment-owned variable
must use the owning `group_name`.

### Local search

One streaming acceptor/forager local-search phase follows construction, but
only when top-level termination contains an effective parsed limit. Absent
termination, an empty termination object, or invalid score-only termination
leaves the solve construction-only. Variable Neighborhood Descent is never
added implicitly.

When `move_selector` is omitted from `acceptor_forager` local search, the
runtime uses explicit streaming defaults instead of broad exhaustive
neighborhoods:

- **Scalar slots** get nearby scalar change/swap selectors when the matching
  hooks are declared, then plain `ChangeMoveSelector` plus `SwapMoveSelector`
  for every scalar slot that no assignment group owns.
- **Assignment-backed scalar groups** are edited through their owning grouped
  scalar selector. Generic scalar selectors and conflict-repair defaults leave
  group-owned slots alone.
- **List slots** get capability-matched families in a fixed order:
  1. precedence repair plus list permutation, when the complete precedence move
     bundle exists;
  2. nearby change, plus nearby swap when set access exists, with a limit of
     20, when a cross-position metric exists;
  3. otherwise plain change, plus plain swap when set access exists;
  4. sublist, reverse and k-opt families, only when their required operations
     are declared. K-opt uses the nearby metric when available and the
     unbounded form otherwise;
  5. list ruin for every bound list slot.
- **Mixed models** declare list families first, then nearby scalar, grouped
  scalar, compound conflict-repair and ordinary scalar families.

Leaf order is seeded `Random`. A single family uses `Sequential`; a
multi-family union uses `StratifiedRandom`. The stock forager applies finite
accepted-count horizons where they apply, and `limited_neighborhood` remains
the explicit cap for configured exhaustive selectors.

[Extend the solver](extend-solver.md#canonical-selector-defaults) explains how
these defaults interact with explicit selector configuration.

### Variable Neighborhood Descent

VND is a local-search type you opt into:

```toml
[[phases]]
type = "local_search"
local_search_type = "variable_neighborhood_descent"

[[phases.neighborhoods]]
type = "change_move_selector"
```

## Available phases and moves

| Area | What ships (selectors by their `type` key in `solver.toml`) |
|------|------------|
| Construction | `FirstFit` and `CheapestInsertion` through one compiled runtime graph: scalar-only targets use its descriptor-placement schedule, mixed and list-bearing targets its declaration-order global scan. Specialized list phases: `ListRoundRobin`, `ListCheapestInsertion`, `ListRegretInsertion`, owner-aware `ListClarkeWright`, `ListKOpt` |
| Grouped scalar construction | Named `ScalarGroup` declarations, candidate-backed compound edits, assignment-backed nullable construction with required-slot handling, capacity repair and ordering hooks |
| Local search | `acceptor_forager` with Hill Climbing, Simulated Annealing (level-aware), Tabu Search, Late Acceptance, Great Deluge, Step Counting Hill Climbing or Diversified Late Acceptance; or `variable_neighborhood_descent` with ordered neighborhoods |
| Exhaustive search | Exact search over small finite scalar spaces |
| Partitioned search | Explicit user-defined decomposition with optional Rayon parallelism, see [Extend the solver](extend-solver.md#partitioned-search) |
| Scalar selectors | `change_move_selector`, `swap_move_selector`, `nearby_change_move_selector`, `nearby_swap_move_selector`, `pillar_change_move_selector`, `pillar_swap_move_selector`, `ruin_recreate_move_selector`, `grouped_scalar_move_selector`, `conflict_repair_move_selector`, `compound_conflict_repair_move_selector` |
| List selectors | `list_change_move_selector`, `nearby_list_change_move_selector`, `list_swap_move_selector`, `nearby_list_swap_move_selector`, `list_permute_move_selector`, `list_precedence_move_selector`, `sublist_change_move_selector`, `sublist_swap_move_selector`, `list_reverse_move_selector`, `k_opt_move_selector`, `list_ruin_move_selector` |
| Composition | `union_move_selector`, `cartesian_product_move_selector`, `limited_neighborhood`; native and host-backed compound scalar moves; nearby selection for scalar and list neighborhoods |

The configuration surface also covers bounded candidate tracing, per-selector
ordering and host metrics, weighted union scheduling, score tie-breaking, and
grouped scalar and conflict-repair selectors.

## Cargo features

| Feature | Description |
|---------|-------------|
| `console` | Colored console output with progress tracking |
| `verbose-logging` | Console output at DEBUG level (implies `console`): construction and local-search progress, prompt first work, then about once a second during long phases |
| `serde` | Serialization support for score types |

What the console prints is shown in [Running solves](running-solves.md#console-output).
