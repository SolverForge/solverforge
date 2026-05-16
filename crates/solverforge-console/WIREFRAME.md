# solverforge-console WIREFRAME

Tracing-driven terminal output for SolverForge solve lifecycle and telemetry events.

**Location:** `crates/solverforge-console/`
**Workspace Release:** `0.14.1`

## Dependencies

- `tracing` (workspace) — structured event input
- `tracing-subscriber` (workspace) — console layer registration
- `owo-colors` (workspace) — ANSI color styling
- `num-format` — locale-aware count formatting

**Features:** `verbose-logging`

## File Map

```
src/
├── lib.rs          — Crate root; module declarations and public exports
├── banner.rs       — ASCII banner and `CARGO_PKG_VERSION` version line
├── format.rs       — Event formatting for solve, phase, progress, and trace-step events
├── format_tests.rs — Console formatter tests
├── init.rs         — `init()` tracing subscriber setup
├── layer.rs        — `SolverConsoleLayer` tracing layer
├── time.rs         — Solve-start time tracking and elapsed formatting support
└── visitor.rs      — `EventVisitor` for structured tracing fields
```

## Public Re-exports

```rust
pub use init::init;
pub use layer::SolverConsoleLayer;
```

## Public Types And Functions

### `init()`

Initializes console output once per process. It prints the SolverForge banner,
configures a default tracing filter, and installs `SolverConsoleLayer`.

With the `verbose-logging` feature, the default solver target is
`solverforge_solver=debug`; otherwise it is `solverforge_solver=info`.

### `SolverConsoleLayer`

Tracing subscriber layer that formats SolverForge events. It accepts events from
`solverforge_solver`, `solverforge_dynamic`, `solverforge_py`, and
`solverforge::` targets. Unknown event names produce no output.

## Event Surface

The formatter recognizes these `event` field values:

| Event | Rendered Output |
|-------|-----------------|
| `solve_start` | Solve banner line with entity count, list element or scalar candidate count, problem scale, optional constraint count, and optional time limit |
| `phase_start` | Phase start line, including score when the event carries `score` |
| `phase_end` | Phase end line with duration, steps, throughput, accepted/generated/evaluated counts, score calculations, generation/evaluation time, and score |
| `progress` | Periodic progress line with steps, speed, evaluated/accepted/generated move counts, score calculations, acceptance rate, current score, and best score when distinct |
| `step` | TRACE-only individual move evaluation line keyed by `move_index` |
| `solve_end` | Final solve line and summary box with score, generated/evaluated/accepted move counts, step count, score calculations, timing, throughput, and acceptance rate |

Startup scale labels are shape-aware: list solves render `elements`; scalar
solves render `candidates`.

## Architectural Notes

- **Formatting-only crate.** Solver behavior, telemetry collection, and lifecycle state live in `solverforge-solver`.
- **Version line uses crate metadata.** The banner reads `env!("CARGO_PKG_VERSION")`, so release bumps update console output through normal Cargo metadata.
- **Subscriber setup is idempotent.** `init()` uses `OnceLock` and can be called multiple times safely.
- **No solver dependency.** The crate consumes structured tracing fields and does not depend on solver internals.
