# Running solves

`SolverManager` runs solves as retained jobs. You submit a problem, get a job
id and an event stream back, and can pause, resume, change, inspect and cancel
the job while it runs. The API speaks in neutral `job`, `snapshot` and
`checkpoint` terms throughout.

## Jobs

Declare the manager as a `static` so it satisfies the `'static` lifetime
requirement:

```rust
use solverforge::{SolverLifecycleState, SolverManager, SolverStatus};

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

`solve()` returns the job id and an owned `SolverEventReceiver`. Read the job's
`SolverEvent`s with `recv().await` in async code, `blocking_recv()` on a plain
thread, or `try_recv()` without waiting.

### Pause, resume, cancel

- `pause()` settles at a safe boundary owned by the runtime. `resume()`
  continues from that exact in-process state; it does not restart from the
  best solution.
- Before mandatory construction completes, a paused state stays an internal
  resumable pause and is not exposed as a solution snapshot.
- Built-in phases poll for control during large neighborhood generation and
  evaluation, so config time limits, `pause()` and `cancel()` take effect
  promptly. Apps do not need watchdogs.
- `delete` is for cleanup of terminal jobs only. It removes the job from the
  public API immediately; the underlying slot becomes reusable once the worker
  has fully exited.

### Events

Lifecycle events carry `job_id`, a monotonic `event_sequence`,
`snapshot_revision`, telemetry and the authoritative lifecycle state. Progress
metadata reflects the current runtime state, including `PauseRequested` while a
pause is settling. Once `pause()` is accepted, the stream delivers
`PauseRequested` before any later worker-side event that was already published
in that state.

### Snapshots and analysis

Snapshot analysis is always bound to a retained `snapshot_revision`, whether the
job is solving, pause-requested, paused or terminal. Analysis being available
never proves a job has completed. Which constraint set is used, and how to turn
analysis off, is covered in
[Configuration](configuration.md#choosing-a-constraint-set).

## Real-time problem changes

A running or paused job accepts problem changes. The runtime applies queued
changes in submission order at its next safe point, starting from the best
solution. It then rescores from scratch and restarts its phases, so
construction assigns the new work before local search resumes:

```rust
use solverforge::ClosureProblemChange;

let id = MANAGER
    .add_problem_change(
        job_id,
        ClosureProblemChange::<MySchedule, _>::new("add shift", move |director| {
            director.working_solution_mut().shifts.push(new_shift);
        }),
    )
    .unwrap();
// Applied once telemetry.problem_changes_applied >= id.get(); the next
// BestSolution carries the changed problem and may score lower than before.
```

- Implement `ProblemChange<S>` for reusable changes.
- Changes are rejected once a job is cancelled, terminal, or past its final
  phase pass. Queued changes are discarded if cancellation or a configured
  limit stops the solve first.
- Run real-time jobs without a solver-level time limit and end them with
  `cancel()`, because configured limits stay binding during re-construction.

How custom phases cooperate with problem changes is in
[Extend the solver](extend-solver.md#real-time-problem-changes).

## Telemetry

Telemetry keeps exact counts and `Duration`s through the whole pipeline.
Retained status and events expose generated, evaluated and accepted move counts
together with generation and evaluation durations. The `moves/s` figure you see
in the console is derived for display only.

When a solve on the compiled runtime ends, `telemetry.resolved_phase_plan`
holds the policy it actually ran: each phase, the construction each default
stage chose and the `reason` for it, the move families with their limits, the
families left out and the capability they were missing, the family ordering
and weighting, and the acceptor, forager and effective
`forager_accepted_count_limit`. It is set whether or not `candidate_trace` is
enabled. Together with the per-phase `step_count`, `moves_evaluated`,
`moves_accepted` and elapsed time in `telemetry.phases`, it separates a
costly step selection from a lack of useful moves.

Generic and specialized construction phases publish the same lifecycle and
progress telemetry as local search. With verbose logging, the first phase work
is reported promptly and long phases continue at roughly one-second intervals.
Rust applications and host-language bindings consume the same events; nothing
synthesizes construction output on the side.

## Console output

With `features = ["console"]`, SolverForge prints progress as it solves. The
solve-start line depends on the model's shape: list models show `elements`,
scalar models show average `candidates`.

```
 ____        _                 _____
/ ___|  ___ | |_   _____ _ __ |  ___|__  _ __ __ _  ___
\___ \ / _ \| \ \ / / _ \ '__|| |_ / _ \| '__/ _` |/ _ \
 ___) | (_) | |\ V /  __/ |   |  _| (_) | | | (_| |  __/
|____/ \___/|_| \_/ \___|_|   |_|  \___/|_|  \__, |\___|
                                             |___/
                   v0.27.0 - Zero-Erasure Constraint Solver

  0.000s ▶ Solving │ 14 entities │ 5 candidates │ scale 9.799 x 10^0
  0.001s ▶ Construction Heuristic started
  0.001s ⚡ Construction Heuristic │          0 steps │       14,000/s │ 1 moves │ 0 accepted │ 1 calcs │ 0.0% │ -14hard/0soft │ 1 generated
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

### Log levels

| Level | Content | How to enable |
|-------|---------|---------------|
| INFO | Lifecycle events (solve and phase start/end) | Default |
| DEBUG | Construction and local-search progress: first work promptly, then about once a second with phase, speed and score | `verbose-logging` feature |
| TRACE | Individual move evaluations | `RUST_LOG=solverforge_solver=trace` |
