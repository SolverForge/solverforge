# SolverForge

<div align="center">

<img src="assets/solverforge-mascot.png" alt="SolverForge mascot" width="180">

[![CI](https://github.com/SolverForge/solverforge/actions/workflows/ci.yml/badge.svg)](https://github.com/SolverForge/solverforge/actions/workflows/ci.yml)
[![Release](https://github.com/SolverForge/solverforge/actions/workflows/release.yml/badge.svg)](https://github.com/SolverForge/solverforge/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/solverforge.svg)](https://crates.io/crates/solverforge)
[![Documentation](https://docs.rs/solverforge/badge.svg)](https://docs.rs/solverforge)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/solverforge.svg)](https://crates.io/crates/solverforge)

</div>

> **Used in Production**
>
> "Working like a charm, A+" — *Dr. Fawaz Halwani, Pathologist, The Ottawa Hospital*
>
> "High-level abstractions, zero-cost implementation. A masterclass in Rust architecture." — *Prof. Benjamin Abel, Computer Science, Côte d’Azur University, Nice*

A Rust planning engine for problems where you need to assign scarce resources
without breaking hard rules.

Use SolverForge when the answer depends on many coupled decisions: who works
which shift, which vehicle visits which stop, which task runs on which machine,
or which resource should be reserved for which job. You describe the facts, the
planning variables, and the constraints. SolverForge searches for a feasible,
high-scoring solution while preserving concrete Rust types through the runtime.

Most new projects should start with `solverforge-cli`. This repository contains
the runtime crates, examples, and API references that the generated application
uses after the scaffold exists.

## Get Started

```bash
cargo install solverforge-cli
solverforge new my-scheduler
cd my-scheduler
solverforge generate fact resource --field category:String --field load:i32
solverforge generate entity task --field label:String --field priority:i32
solverforge generate variable resource_idx --entity Task --kind scalar --range resources --allows-unassigned
solverforge server
```

Open http://localhost:7860 to see your solver in action.

That sequence creates a complete local SolverForge application, adds a resource
fact, adds a task planning entity, gives each task an optional scalar assignment,
and starts the generated web server.

The CLI is intentionally neutral. `solverforge new <name>` gives you the shell;
`solverforge generate ...` adds facts, entities, variables, constraints, and
demo data as your domain becomes clearer. Generated applications can mix scalar
planning variables with independent planning lists, and they use the same
`solverforge` facade crate documented here.

## Extend the Scaffold

- [Extend the domain](docs/extend-domain.md) when you need more entities,
  facts, variables, scoring, or mixed scalar/list modeling inside a generated
  app.
- [Extend the solver](docs/extend-solver.md) when you need custom phases,
  selectors, termination, or solver configuration beyond the scaffold defaults.

## Documentation Map

- New users should start with this README and the complete packages under
  [`examples/`](examples/).
- Generated app work starts in the standalone
  [`solverforge-cli`](https://github.com/solverforge/solverforge-cli)
  repository, then continues against the runtime crates in this workspace.
- `docs/extend-domain.md` and `docs/extend-solver.md` cover scaffold extension
  workflows.
- `crates/*/WIREFRAME.md` files are the canonical public API maps for each crate.
- `AGENTS.md` defines repository-level engineering and documentation expectations for coding agents.

## Zero-Erasure Architecture

SolverForge keeps its canonical native solver and scoring pipeline concrete,
with a small set of explicit erasure seams:

- **Concrete native dispatch** - Rust solver, selector, move, provider, and scoring generics resolve at compile time
- **Explicit erasure seams** - object-safe dispatch is confined to documented dynamic/host integration, descriptor access, real-time problem-change, and analysis/panic boundaries
- **No trait-object allocation in retained scoring state** - candidates stay cursor-owned until the selected move transfers by value, scoring pipelines stay monomorphized, and retained forager state is explicit
- **Deterministic neighborhood order** - canonical list, nearby-list, and sublist selector enumeration keeps seeded local search reproducible
- **Predictable native performance** - no GC pauses; move and selector carriers stay concrete, with only the documented scorer-agnostic and integration seams using dynamic dispatch

This enables aggressive compiler optimizations and cache-friendly data layouts.

Dynamic scalar/list access and metadata, configured candidate metrics, and
host-language compound providers are explicit integration boundaries. Native
Rust `ScalarCandidateProvider` and `RepairProvider` functions remain concrete
function pointers and carry typed `ScalarEdit`
values directly into the shared cursor normalizer.

The object-safe `EntityExtractor` boundary belongs to runtime-erased
`SolutionDescriptor` access; canonical generated runtime and scoring-stream
paths retain their concrete collection and selector types. Public concrete
adapters and selector APIs are `EntityCollectionExtractor`, `ValueSelector`,
and `MoveSelector`.

## Features

- **Score Types**: SoftScore, HardSoftScore, HardMediumSoftScore, BendableScore, HardSoftDecimalScore
- **ConstraintStream API**: Declarative constraints with fluent builders, model-owned collection sources, filtered join sources, single-source and cross-join projected scoring rows, symmetric and directed projected self-joins, direct cross-join grouping and grouped complements, projected grouped complements, existence checks, joins, grouping, `collect_vec`, `consecutive_runs`, `indexed_presence`, and balance/complemented streams
- **SERIO Engine**: Scoring Engine for Real-time Incremental Optimization
- **Solver Phases**:
  - Generic Construction Heuristics (`FirstFit`, `CheapestInsertion`) through one compiled runtime graph: scalar-only targets use its descriptor-placement schedule, mixed/list-bearing targets use its declaration-order global scan, and specialized list phases provide `ListRoundRobin`, `ListCheapestInsertion`, `ListRegretInsertion`, owner-aware `ListClarkeWright`, and `ListKOpt`
  - Grouped scalar construction for named `ScalarGroup` declarations, including
    candidate-backed compound edits and assignment-backed nullable scalar
    construction with required-slot handling, capacity repair, and heuristic
    ordering hooks
  - Local Search (`acceptor_forager` with Hill Climbing, Simulated Annealing, Tabu Search, Late Acceptance, Great Deluge, Step Counting Hill Climbing, and Diversified Late Acceptance; or `variable_neighborhood_descent` with ordered neighborhoods)
  - Exhaustive Search for exact small finite scalar spaces
  - Partitioned Search for explicit user-defined decomposition with optional Rayon parallelism
- **Move System**: Zero-erasure moves with cursor-scoped candidate ownership and by-value selected-winner materialization
  - Scalar: ChangeMove, SwapMove, PillarChangeMove, PillarSwapMove, RuinMove
  - List: ListChangeMove, ListSwapMove, ListPermuteMove, ListReverseMove, SublistChangeMove, SublistSwapMove, KOptMove, ListRuinMove
  - Scalar ruin-recreate, precedence repair, native and host-backed compound scalar moves, cartesian composition, and nearby selection for scalar and list neighborhoods
- **SolverManager API**: Retained job / snapshot / checkpoint lifecycle with exact pause/resume, lifecycle-complete events, snapshot retrieval, snapshot-bound analysis, and telemetry
- **Model Macros**: `planning_model!`, `#[solverforge_constraints]`, `#[planning_solution]`, `#[planning_entity]`, `#[problem_fact]`
- **Dynamic Bridge**: `solverforge-bridge` contracts for host-language integrations, with logical entity/fact/variable IDs, dynamic score support, descriptor-resolved scalar/list slots, lazy nearby-source adapters, and explicit dynamic-list access and metadata capabilities
- **Scalar Variables**: `#[planning_variable]` fields store candidate indexes
  as `Option<usize>`; keep external IDs on facts or entities.
- **List Variables**: `#[planning_list_variable]` fields store ordered element
  indexes in their owning entities, with list-aware construction and local
  search plus inverse, index, previous, and next shadow support.
- **CVRP List Profile**: `#[planning_list_variable(domain = "cvrp")]` wires
  stock CVRP distance meters plus split route/savings hooks. Route-local phases
  use strict capacity, time-window, and unreachable-leg feasibility;
  Clarke-Wright construction uses relaxed savings admissibility so assignment
  remains score-comparable.
- **Configuration**: TOML/YAML support with builder APIs, bounded candidate tracing, per-selector ordering and host metrics, weighted union scheduling, score tie-breaking, grouped scalar and conflict-repair selectors, and level-aware simulated annealing configuration
- **Console Output**: Colorful tracing-based progress display with solve telemetry

Grouped scoring weights receive both the group key and collected result:

```rust
ConstraintFactory::<Schedule, HardSoftScore>::new()
    .for_each(Schedule::shifts())
    .filter(|shift: &Shift| shift.nurse_idx.is_some())
    .group_by(
        |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
        consecutive_runs(|shift: &Shift| shift.day),
    )
    .penalize(|_nurse_idx: &usize, runs: &Runs| {
        let excess_days = runs
            .runs()
            .iter()
            .map(|run| run.point_count().saturating_sub(2) as i64)
            .sum();
        HardSoftScore::of_soft(excess_days)
    })
    .named("Long work streaks");
```

Collectors retain mapped values by ownership. The same collector protocol covers
unary rows and joined pairs: direct cross-join grouping passes `(&A, &B)` to the
collector mapper, while both direct cross-join grouped streams and projected
grouped streams can continue into `complement(...)` for supply/demand-style
scoring. `collect_vec` exposes a
`CollectedVec<T>` result view, so grouped payloads such as `String` labels do
not need `Copy`, `Clone`, or `PartialEq` just to be collected.

Constraint functions should be annotated with `#[solverforge_constraints]`.
The attribute keeps the same fluent Rust authoring style while giving
SolverForge a whole-function compiler boundary. When the same grouped stream
binding is reused by multiple terminal constraints, the compiler builds one
shared incremental node through the same fluent terminal path and keeps each
terminal constraint separately named:

```rust
#[solverforge_constraints]
fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    let g = ConstraintFactory::<Schedule, HardSoftScore>::new();
    let nurse_presence = g
        .for_each(Schedule::shifts())
        .filter(|shift: &Shift| shift.nurse_idx.is_some())
        .group_by(
            |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
            indexed_presence(|shift: &Shift| shift.day),
        );

    (
        nurse_presence
            .penalize(consecutive_work_bounds)
            .named("consecutiveWorkBounds"),
        nurse_presence
            .penalize(consecutive_off_bounds)
            .named("consecutiveOffBounds"),
        nurse_presence
            .penalize(working_weekends)
            .named("workingWeekends"),
    )
}
```

Repeated same-binding grouped terminals share incremental grouped work.
Terminal identity, ordering, metadata, and score explanation remain independent.

## Installation

If you are building directly against the runtime crates instead of starting from
`solverforge-cli`, add the facade crate to your `Cargo.toml`:

```toml
[dependencies]
solverforge = { version = "0.19.0", features = ["console"] }
```

### List planning variables

Use a list variable whenever planning decides assignment and order together.
The owner stores element indexes directly, so route and sequence topology has
one canonical representation rather than a scalar predecessor graph.

```rust
#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "visits")]
    pub visit_order: Vec<usize>,
}

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
```

List construction assigns unassigned elements to owners. List change, swap,
sublist, reversal, k-opt, ruin/recreate, and nearby selectors then operate on
the same representation and retain exact undo. Configure inverse, index,
previous, next, cascading, custom, or piggyback shadows only when constraints or
application logic need those derived views. Generated index maintenance uses an
`Option<usize>` field on each element and is enabled with
`#[shadow_variable_updates(list_owner = "routes", index_field = "index")]` on
the planning solution.

When `move_selector` is omitted from `acceptor_forager` local search, the
canonical runtime uses explicit streaming defaults instead of broad exhaustive
neighborhoods:

- scalar models default to nearby scalar change/swap selectors when hooks are
  declared, then plain `ChangeMoveSelector` plus `SwapMoveSelector` fallback
  selectors for every non-assignment-owned scalar slot
- assignment-backed scalar groups are edited through their owning grouped
  scalar selector; generic scalar selectors and conflict-repair defaults leave
  those group-owned slots on the grouped path
- list slots receive capability-matched families in a fixed order: precedence
  repair plus list permutation when the complete precedence move bundle exists;
  nearby change, plus nearby swap when set access exists, with a limit of 20
  when a cross-position metric exists; plain change, plus plain swap when set
  access exists, when that metric does not; sublist, reverse, and k-opt families
  only when their required operations are declared; and list ruin for every
  bound list slot. K-opt uses the nearby metric when available and the unbounded
  form otherwise
- mixed models declare list families first, then nearby scalar, grouped-scalar,
  compound conflict-repair, and ordinary scalar families. Default leaf order
  is seeded `Random`; a multi-family union uses `StratifiedRandom` (a single
  family uses `Sequential`)

List variables can declare fixed element ownership with
`#[planning_list_variable(element_collection = "...", element_owner_fn = "...")]`.
The hook signature is `fn(&Solution, usize) -> Option<usize>` and returns the
only owner entity index allowed for that list element, or `None` to leave the
element unrestricted. Out-of-range owner indexes are invalid rather than widened
to unrestricted. When the model is assembled through `planning_model!`, the hook
must be visible from the manifest root module. Construction, Clarke-Wright list
construction, ruin/recreate, and owner-changing list neighborhoods honor the
hook; intra-owner ordering moves remain available.

### Specialized list-construction source identity

`ListConstructionPhaseBuilder`, `ListCheapestInsertionPhase`,
`ListRegretInsertionPhase`, and `ListClarkeWrightPhase` require an explicit
`element_source_key` function. The function maps each declared element, each
currently assigned element, and each precedence-successor value to the same
unique, stable `usize` source identity. SolverForge freezes that binding before
construction, then uses source indexes for ordering, ownership, precedence,
candidate traces, and static/dynamic execution. It never recovers identity by
payload equality or hashing; duplicate, missing, or inconsistent keys are a
construction bind error. When multiple construction phases target the same
list slot, they share the frozen declaration binding but refresh current
assignments before each phase, so later phases cannot reinsert earlier work or
reread declaration callbacks.

Those omitted-config defaults run as one streaming acceptor/forager local
search phase after construction. Multi-family unions use stratified-random
selection and the stock forager applies finite accepted-count horizons where
applicable; `limited_neighborhood` remains the explicit cap for configured
exhaustive selectors. Variable Neighborhood Descent is never prepended
implicitly.

When full `phases` are omitted, construction runs model-aware defaults before
that local search: list variables use the matching specialized list
construction, assignment-backed scalar groups run named grouped
`CheapestInsertion` passes for required and optional slots, and remaining
non-assignment-owned scalar variables use descriptor-backed single-slot
construction.
Explicit scalar construction targets that name an assignment-owned variable
must use the owning `group_name`.

Variable Neighborhood Descent is configured as a local-search type:

```toml
[[phases]]
type = "local_search"
local_search_type = "variable_neighborhood_descent"

[[phases.neighborhoods]]
type = "change_move_selector"
```

Runtime telemetry now preserves exact counts and `Duration`s through the whole
pipeline. Retained status/events expose generated, evaluated, and accepted move
counts together with generation and evaluation durations; human-facing
`moves/s` remains a display-only derived value.

Neighborhood selector cleanup is benchmark-gated in the runtime crates. Shared
support code backs exact sizing and reusable bookkeeping, while the
move-enumeration hot loops for list and sublist neighborhoods stay explicit and
monomorphized.

### Feature Flags

| Feature | Description |
|---------|-------------|
| `console` | Colorful console output with progress tracking |
| `verbose-logging` | DEBUG-level phase progress updates (1/sec during long-running work) |
| `decimal` | Forwards the currently empty `solverforge-core/decimal` feature; fixed-scale `HardSoftDecimalScore` is always available |
| `serde` | Serialization support for score types |

## Quick Start

The README quickstart is the checked-in
[`minimal-shift-scheduling`](examples/minimal-shift-scheduling/) package. It is
small enough to read in one sitting, but it is still a real SolverForge model:
`make examples` builds it, and the command below compiles and runs it from the
workspace.

```bash
cargo run -p minimal-shift-scheduling
```

A successful run prints a feasible nurse assignment:

```text
score: 0hard/0soft
day 0 slot 0 -> Amina
day 0 slot 1 -> Bruno
day 1 slot 0 -> Bruno
day 1 slot 1 -> Chiara
day 2 slot 0 -> Chiara
day 2 slot 1 -> Amina
day 3 slot 0 -> Amina
day 3 slot 1 -> Bruno
day 4 slot 0 -> Bruno
day 4 slot 1 -> Chiara
day 5 slot 0 -> Chiara
day 5 slot 1 -> Amina
```

Read the quickstart files in this order:

- [`src/domain/mod.rs`](examples/minimal-shift-scheduling/src/domain/mod.rs)
  declares the model with `solverforge::planning_model!`.
- [`src/domain/nurse.rs`](examples/minimal-shift-scheduling/src/domain/nurse.rs)
  defines the problem facts.
- [`src/domain/shift.rs`](examples/minimal-shift-scheduling/src/domain/shift.rs)
  defines the planning entities and the `nurse_idx` planning variable.
- [`src/domain/schedule.rs`](examples/minimal-shift-scheduling/src/domain/schedule.rs)
  owns the solution, constraints, `solver.toml` link, and assignment-backed
  `ScalarGroup` hooks.
- [`solver.toml`](examples/minimal-shift-scheduling/solver.toml) configures
  required-shift assignment construction and grouped scalar local search.
- [`src/main.rs`](examples/minimal-shift-scheduling/src/main.rs) builds a small
  schedule, starts `SolverManager`, consumes solver events, and prints the
  completed assignment.

The important boundary is that SolverForge model metadata is owned by the model,
not by detached helper imports. The domain manifest lists normal Rust modules;
the `#[planning_solution]` macro generates collection source functions such as
`Schedule::shifts()`, and constraints use those functions through
`ConstraintFactory::for_each(...)`. It also generates a matching
`ScheduleConstraintStreams` trait for shorthand calls such as
`ConstraintFactory::new().shifts()` when the trait is imported.

For a new application, start with `solverforge-cli` so the file layout,
`solver.toml`, model manifest, constraints, and frontend handoff are generated
together. Use the runtime crates directly when you want to embed SolverForge in
an existing Rust service or build lower-level integrations.

## Console Output

With `features = ["console"]`, SolverForge displays colorful progress:

The solve-start line is shape-aware: list models show `elements`, and scalar
models show average `candidates`.

```
 ____        _                 _____
/ ___|  ___ | |_   _____ _ __ |  ___|__  _ __ __ _  ___
\___ \ / _ \| \ \ / / _ \ '__|| |_ / _ \| '__/ _` |/ _ \
 ___) | (_) | |\ V /  __/ |   |  _| (_) | | | (_| |  __/
|____/ \___/|_| \_/ \___|_|   |_|  \___/|_|  \__, |\___|
                                             |___/
                   v0.19.0 - Zero-Erasure Constraint Solver

  0.000s ▶ Solving │ 14 entities │ 5 candidates │ scale 9.799 x 10^0
  0.001s ▶ Construction Heuristic started
  0.002s ◀ Construction Heuristic ended │ 1ms │ 14 steps │ 14,000 moves/s │ 14 moves │ 14 generated │ 14 accepted moves │ 14 calcs │ 0hard/-50soft
  0.002s ▶ Local Search started │ 0hard/-50soft
  1.002s ⚡    12,456 steps │      445,000/s │ 12,456 moves │ 1,234 accepted │ 12,456 calcs │ 9.9% │ -2hard/8soft │ 12,456 generated
  2.003s ⚡    24,891 steps │      448,000/s │ 24,891 moves │ 2,734 accepted │ 24,891 calcs │ 11.0% │ 0hard/12soft │ 24,891 generated
 30.001s ◀ Local Search ended │ 30s │ 104,864 steps │ 456,000 moves/s │ 11.9% accepted │ 104,864 moves │ 104,864 generated │ 12,456 accepted moves │ 104,864 calcs │ gen 1s 240ms │ eval 28s 760ms │ 0hard/15soft
 30.001s ■ Solving complete │ 0hard/15soft │ FEASIBLE

╔══════════════════════════════════════════════════════════╗
║                 FEASIBLE SOLUTION FOUND                  ║
╠══════════════════════════════════════════════════════════╣
║  Final Score:                            0hard/15soft    ║
║  Moves Generated:                            104,864     ║
║  Steps:                                      104,864     ║
║  Generation Time:                              1.24s     ║
║  Evaluation Time:                             28.76s     ║
║  Moves/s:                                    456,000     ║
║  Moves Evaluated:                            104,864     ║
║  Moves Accepted:                              12,456     ║
║  Score Calcs:                                104,864     ║
║  Acceptance:                                  11.9%      ║
╚══════════════════════════════════════════════════════════╝
```

### Log Levels

| Level | Content | When |
|-------|---------|------|
| INFO | Lifecycle events (solve/phase start/end) | Default |
| DEBUG | Phase progress updates (1/sec with phase, speed, and score) | `verbose-logging` feature |
| TRACE | Individual move evaluations | `RUST_LOG=solverforge_solver=trace` |

## Architecture

![SERIO incremental scoring](assets/SERIO.jpg)

```text
solverforge
  facade + prelude + public re-exports
  |
  |-- solverforge-core      scores, domain traits, descriptors, logical IDs
  |-- solverforge-macros    planning_model! and attribute macros
  |-- solverforge-scoring   ConstraintStream API and SERIO scoring
  |-- solverforge-config    TOML/YAML configuration
  |-- solverforge-solver    phases, moves, selectors, runtime, SolverManager
  |-- solverforge-bridge    dynamic host-language binding contracts
  |-- solverforge-cvrp      CVRP profile helpers, traits, meters, and hook bundles
  `-- solverforge-console   optional tracing console feature

Dependency layers:
  solverforge          -> all listed SolverForge crates (console optional)
  solverforge-scoring -> solverforge-core
  solverforge-config  -> solverforge-core
  solverforge-solver  -> solverforge-core + solverforge-scoring + solverforge-config + tokio sync
  solverforge-bridge  -> solverforge-core + solverforge-scoring + solverforge-config + solverforge-solver
  solverforge-cvrp    -> solverforge-solver
  solverforge-macros  -> proc-macro dependencies only; generated code references the facade
  solverforge-console -> tracing + tracing-subscriber + owo-colors + num-format
```

## Crate Overview

| Crate | Purpose |
|-------|---------|
| `solverforge` | Main facade with prelude and re-exports |
| `solverforge-bridge` | Dynamic host-language binding contracts: logical IDs, dynamic score support, dynamic slots, and binding runner helper |
| `solverforge-core` | Core types: scores, domain traits, descriptors |
| `solverforge-solver` | Solver engine: phases, moves, termination, SolverManager, telemetry |
| `solverforge-scoring` | ConstraintStream API, SERIO incremental scoring |
| `solverforge-config` | Configuration via TOML and builder API |
| `solverforge-console` | Tracing-based console output with banner and progress display |
| `solverforge-macros` | Procedural macros for domain model |
| `solverforge-cvrp` | CVRP domain profile helpers: `VrpSolution`, `ProblemData`, distance meters, and split stock route/savings hook bundles |

## Score Types

```rust
use solverforge::prelude::*;

// Single-level score
let score = SoftScore::of(-5);

// Two-level score (hard + soft)
let score = HardSoftScore::of(-2, 100);
assert!(!score.is_feasible());  // Hard score < 0

// Three-level score
let score = HardMediumSoftScore::of(0, -50, 200);

// Decimal precision
let score = HardSoftDecimalScore::of_scaled(0, -12_345_000);

// N-level configurable
let score = BendableScore::<2, 2>::of([0, -1], [-50, -100]);
```

## Termination

Configure via `solver.toml`:

```toml
[termination]
seconds_spent_limit = 30
unimproved_seconds_spent_limit = 5
step_count_limit = 10000
```

Or programmatically:

```rust
let config = SolverConfig::load("solver.toml").unwrap_or_default();
```

For macro-generated retained solves, the solution module listed by
`planning_model!` can use `config = "..."` to decorate the loaded `solver.toml`
config instead of replacing it:

```rust
#[planning_solution(
    constraints = "define_constraints",
    config = "solver_config_for_solution"
)]
pub struct Schedule {
    // ...
}

fn solver_config_for_solution(solution: &Schedule, config: SolverConfig) -> SolverConfig {
    config.with_termination_seconds(solution.time_limit_secs)
}
```

Configured termination limits always remain binding, including during required
scalar and list construction. A configured solve publishes a best solution only
after every mandatory list element, required assignment row, and non-optional
scalar variable is assigned. If a limit is reached first, the solve ends as
`Failed`; an incomplete construction state is never emitted as a best or
completed solution.

## SolverManager API

The `SolverManager` owns the retained runtime lifecycle for each job. The public contract uses neutral `job`, `snapshot`, and `checkpoint` terminology throughout the API. `pause()` settles at a runtime-owned safe boundary and `resume()` continues from the exact in-process state rather than restarting from the best solution. Before mandatory construction completes, that state remains an internal resumable pause and is not exposed as a solution snapshot. Built-in search phases poll retained-runtime control during large neighborhood generation and evaluation, so config time limits, `pause()`, and `cancel()` unwind promptly without app-side watchdogs. Declare a `static` instance so it satisfies the `'static` lifetime requirement:

```rust
use solverforge::{SolverLifecycleState, SolverManager, SolverStatus, Solvable};

static MANAGER: SolverManager<MySchedule> = SolverManager::new();

let (job_id, mut events) = MANAGER.solve(problem).unwrap();

let status: SolverStatus<_> = MANAGER.get_status(job_id).unwrap();
assert_eq!(status.lifecycle_state, SolverLifecycleState::Solving);

MANAGER.pause(job_id).unwrap();
MANAGER.resume(job_id).unwrap();
MANAGER.cancel(job_id).unwrap();

let snapshot = MANAGER.get_snapshot(job_id, None).unwrap();
let analysis = MANAGER.analyze_snapshot(job_id, Some(snapshot.snapshot_revision)).unwrap();
MANAGER.delete(job_id).unwrap();
```

Lifecycle events carry `job_id`, monotonic `event_sequence`, `snapshot_revision`, telemetry, and authoritative lifecycle state. Progress metadata reflects the current runtime state, including `PauseRequested` while a pause is settling. After `pause()` is accepted, the stream delivers `PauseRequested` before any later worker-side event already published in `PauseRequested` state. Snapshot analysis is always bound to a retained `snapshot_revision`, whether the job is still solving, pause-requested, paused, or already terminal, and analysis availability must never be treated as proof that a job has completed. `delete` is reserved for cleanup of terminal jobs only: it removes the retained job from the public API immediately, and the underlying slot becomes reusable once the worker has fully exited.

## Score Analysis

Analyze solutions without solving:

```rust
use solverforge::analyze;

let analysis = analyze(&solution);

println!("Score: {}", analysis.score);
for constraint in &analysis.constraints {
    println!("  {}: {}", constraint.name, constraint.score);
}
```

## Examples

Root workspace examples live under [`examples/`](examples/) as complete solver
packages:

```bash
make examples
cargo run -p scalar-graph-coloring
cargo run -p minimal-shift-scheduling
cargo run -p list-tsp
cargo run -p mixed-job-shop
cargo run -p nqueens
```

`make examples` builds every checked-in example package. Run an individual
`cargo run -p ...` command when you want to inspect one model's behavior.

`minimal-shift-scheduling` is the compact public solver path: it uses
`planning_model!`, generated collection sources, `solver.toml`,
assignment-backed `ScalarGroup`, `consecutive_runs`, and `SolverManager`.

For project scaffolding and end-to-end application templates, use the standalone [`solverforge-cli`](https://github.com/solverforge/solverforge-cli) repository: `cargo install solverforge-cli`, then `solverforge new ...`.

## Performance

SolverForge leverages Rust's zero-cost abstractions:

- **Zero-Erasure Moves**: Values stay concrete and inline, with cursor-owned candidate storage and by-value selected-move transfer
- **Cursor Selectors**: GAT-based concrete cursors expose stable candidate IDs and borrowable moves without `Box<dyn Iterator>` in selector hot paths
- **Incremental Scoring**: SERIO propagates only changed constraints
- **No GC**: Predictable latency without garbage collection
- **Cache-friendly**: Contiguous memory layouts for hot paths
- **Concrete native execution**: Score directors, deciders, bounders, move carriers, and selector carriers remain monomorphized; documented scorer-agnostic `&dyn Director` callbacks are narrow intentional seams

Absolute throughput depends on the model, constraints, neighborhood, hardware,
and termination policy. Use emitted generated/evaluated throughput for one run
and the controlled paired benchmark gate below for regression decisions.

`make pre-release` is portable across supported development hosts. Release
qualification also requires the separate controlled-host
`make bench-zero-regression` gate on Linux with `taskset`, `/usr/bin/time`,
readable `/sys` CPU topology, and the selected `BENCH_CPU` available (CPU 10 by
default); `perf` counters are collected when permitted. The gate builds an
independently linked binary for each case, alternates paired trials, verifies
full-enumeration candidate count and order separately from first-fit work, and
rejects a positive median or paired 95% upper bound for wall time, allocations,
peak memory, or available hardware counters. A host that cannot run the
controlled gate has not completed release qualification.

## Status

**Current workspace version:** 0.19.0

The current checked-in workspace exposes:

- the `solverforge` facade crate for application code;
- the `solverforge-bridge` crate for dynamic host-language binding contracts;
- `solverforge-cli` as the recommended first project path;
- model-owned `planning_model!` domains with generated collection sources;
- scalar, list, grouped-scalar,
  assignment-backed scalar-group,
  conflict-repair, ruin-recreate, cartesian, and nearby neighborhoods;
- retained `SolverManager` jobs with progress events, exact pause/resume,
  snapshots, checkpoint status, telemetry, and snapshot-bound analysis;
- TOML/YAML solver configuration plus programmatic overrides;
- optional console rendering through the `console` feature; and
- five workspace examples validated by `make examples`.

Release history belongs in [CHANGELOG.md](CHANGELOG.md). Maintainer release
steps live in [RELEASE.md](RELEASE.md).

## Release Operations

The workspace release checklist, publish order, and crate stability matrix live
in [RELEASE.md](RELEASE.md). Version bumps are explicit release decisions, and
`CHANGELOG.md` remains managed by the `commit-and-tag-version` workflow rather
than hand-maintained in feature branches.

## Minimum Rust Version

Rust 1.95 or later.

## License

Apache License 2.0. See [LICENSE](LICENSE).

## Contributing

Contributions welcome. Please open an issue or pull request.
