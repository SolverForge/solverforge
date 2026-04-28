# PRD: Generic Solver Lifecycle, Pause/Resume, and Snapshot Contract

Date: 2026-04-06

## Purpose

`solverforge-rs`, `solverforge-ui`, and `solverforge-cli` must converge on one
generic lifecycle contract for solving jobs.

This contract must be:

- domain-neutral
- runtime-owned
- exact about pause/resume semantics
- explicit about snapshot identity
- explicit about analysis availability
- explicit about versioning and release order

This is a full contract program, not an incremental patch.

## Problem

The current stack is internally inconsistent:

- the runtime only exposes coarse solve status
- the UI infers completion from transport behavior instead of authoritative
  lifecycle state
- the CLI scaffold still uses scheduling-specific terminology and route names
- retained stop, delete, analysis, and resume semantics are overloaded or
  ambiguous
- "resume" can currently degrade into "restart from last best solution" instead
  of exact continuation

That is not acceptable for a generic planning optimization platform.

SolverForge is not a scheduling-specific product. It is a generic solver and a
generic CLI tool that must support any planning optimization problem.

## Product Principles

### 1. Domain neutrality

No lifecycle, API, or shared UI contract may encode schedule-specific,
board-specific, route-specific, or other domain-specific assumptions.

Shared terminology must use neutral words:

- job
- snapshot
- checkpoint
- analysis
- pause
- resume
- cancel
- delete

Legacy `schedule` terminology is removed from shared UI contracts and generated
scaffold HTTP surfaces in this initiative.

### 2. Runtime truth

Lifecycle truth must come from the runtime contract, not from UI heuristics and
not from scaffold-local helper logic.

The UI may orchestrate a generic lifecycle against that truth, but it must not:

- infer completion from `GET` readability
- infer completion from analysis success
- reconstruct telemetry locally
- fake exact resume semantics that the runtime does not own

### 3. Exact resume means exact resume

If the user pauses and resumes a job, SolverForge must continue from the same
solver session, not from a fresh solve seeded with the last best solution.

Exact resume means preserving, at minimum:

- current working solution state
- best solution state
- current score and best score
- phase pipeline position
- selector / acceptor / forager internal state
- RNG state
- telemetry counters
- configured termination counters and budgets

This guarantee applies to an in-process retained job. Disk persistence and
cross-process checkpoint portability are out of scope for this PRD.

### 4. Analysis is always available

Score analysis must be available:

- while solving
- while pause is requested
- while paused
- after completion
- after configured termination
- after failure, if a retained analyzable snapshot exists

Analysis availability must never be treated as proof that a job is terminal.

## Definitions

### Job

A retained solver session with stable identity from creation until deletion.

### Snapshot

A renderable and analyzable solution state produced by the runtime.

Every renderable snapshot has a monotonic `snapshot_revision` within its job.

### Checkpoint

An internal retained runtime state sufficient for exact resume.

A checkpoint is stronger than a snapshot. A snapshot is renderable. A checkpoint
is resumable.

### Event sequence

Every streamed lifecycle event has a monotonic `event_sequence` within its job.

## Required Lifecycle Model

### Job states

The runtime contract must represent at least these generic states:

- `SOLVING`
- `PAUSE_REQUESTED`
- `PAUSED`
- `COMPLETED`
- `CANCELLED`
- `FAILED`

`NOT_SOLVING` alone is not expressive enough for the required contract.

### Terminal reason

Terminal jobs must expose an explicit terminal reason, separate from current
state.

Required reasons:

- `completed`
- `terminated_by_config`
- `cancelled`
- `failed`

Pause is not terminal and must not be represented as terminal completion.

### State transitions

Required transitions:

- create job -> `SOLVING`
- pause request on live job -> `PAUSE_REQUESTED`
- safe runtime quiescence -> `PAUSED`
- resume paused job -> `SOLVING`
- normal solve end -> `COMPLETED`
- config-driven end -> `COMPLETED` with terminal reason
  `terminated_by_config`
- explicit cancel -> `CANCELLED`
- unrecoverable runtime error -> `FAILED`

Invalid transitions must fail explicitly.

Examples:

- resume a non-paused job
- delete a live job
- pause an already terminal job

## Required Runtime Contract Changes (`solverforge-rs`)

### 1. Runtime-managed exact pause/resume

The runtime must expose exact pause and resume semantics for retained jobs.

The manager layer must own:

- pause request
- pause settlement
- retained checkpoint ownership
- resume from retained checkpoint
- cancel
- delete eligibility

### 2. Rich lifecycle status

The public runtime status/event model must distinguish:

- actively solving
- pause requested but not yet settled
- paused and resumable
- completed
- cancelled
- failed

### 3. Snapshot identity

The runtime contract must expose:

- `job_id`
- `event_sequence`
- `snapshot_revision`

Rules:

- `event_sequence` increments on every emitted lifecycle event
- `snapshot_revision` increments only when a new renderable snapshot is created
- progress-only events may omit a snapshot body, but must still identify the
  latest known `snapshot_revision` when applicable

### 4. Pause-safe checkpoints

Pause must settle only at a runtime-safe boundary where the checkpoint is exact
and resumable.

The runtime may not report `PAUSED` until the checkpoint is ready.

### 5. Termination budget preservation

`solver.toml` termination settings must remain authoritative across pause and
resume.

Paused wall-clock time must not consume active solve budgets.

This applies to:

- `seconds_spent_limit`
- `minutes_spent_limit`
- `unimproved_seconds_spent_limit`
- `step_count_limit`
- `unimproved_step_count_limit`
- any additional runtime counters introduced later

Resume must continue from the preserved counters and budgets, not reset them.

### 6. Event model

The runtime event model must become lifecycle-complete.

Required event families:

- `progress`
- `best_solution`
- `pause_requested`
- `paused`
- `resumed`
- `completed`
- `cancelled`
- `failed`

Every event must carry enough metadata for consumers to render truthfully:

- `job_id`
- `event_sequence`
- current lifecycle state
- terminal reason when applicable
- telemetry
- current score / best score where applicable
- `snapshot_revision` when applicable

### 7. Exact-resume validation mode

The runtime must support deterministic validation of exact resume in
reproducible mode.

Acceptance must compare:

- uninterrupted run
- paused then resumed run at the same event boundary

From the pause boundary onward, the resumed run must match the uninterrupted run
for event order, snapshot revisions, scores, and final result.

## Required Shared UI Contract Changes (`solverforge-ui`)

### 1. Generic naming

The shared backend adapter and solver lifecycle API must stop using
schedule-specific method names.

Required neutral naming:

- create job
- get job
- get job status
- get snapshot
- analyze snapshot
- pause job
- resume job
- cancel job
- delete job
- stream job events

### 2. Shared lifecycle orchestration

`solverforge-ui` must own the generic orchestration for:

- pause request
- waiting for authoritative `PAUSED`
- resume
- completion handling
- cancellation handling
- analysis against specific snapshot revisions

Template applications must not reimplement stop/poll/sync logic.

### 3. Callback contract

The shared solver API must distinguish:

- progress metadata
- live best-solution snapshots
- paused checkpoint availability
- terminal completion
- failure

`onComplete` must not fire for a paused job.

If needed, introduce separate lifecycle callbacks rather than overloading
completion semantics.

### 4. Analysis consistency

The UI must request analysis against a specific `snapshot_revision` when it
needs the rendered snapshot and analysis to match exactly.

Successful analysis during solving must not collapse the job to an idle or
completed state.

### 5. Shared controls remain generic

Shared controls and status components may reflect state, but must remain dumb.
They must not encode scaffold-specific or domain-specific lifecycle rules.

## Required CLI Scaffold Changes (`solverforge-cli`)

### 1. Neutral scaffold API

The generated scaffold must expose a domain-neutral HTTP lifecycle.

Required resource shape:

- `POST /jobs`
- `GET /jobs/{id}`
- `GET /jobs/{id}/snapshot`
- `GET /jobs/{id}/analysis`
- `POST /jobs/{id}/pause`
- `POST /jobs/{id}/resume`
- `POST /jobs/{id}/cancel`
- `DELETE /jobs/{id}`
- `GET /jobs/{id}/events`

The current generated `schedules` surface is retired in this initiative.

### 2. Thin generated UI

The generated neutral shell must consume the shared `solverforge-ui` lifecycle
contract instead of reconstructing it locally.

The scaffold may decide presentation, but not lifecycle truth.

### 3. Neutral generated docs

Generated README and app docs must describe:

- CLI version separately from runtime/UI target versions
- generic job/pause/resume semantics
- `solver.toml` as search-strategy configuration

No generated docs may imply scheduling-specific semantics as the shared default.

## API Contract Requirements

### Job summary

`GET /jobs/{id}` must return at least:

- `id`
- current lifecycle state
- terminal reason when present
- `checkpoint_available`
- latest `snapshot_revision`
- current score
- best score
- runtime telemetry summary

### Snapshot payload

`GET /jobs/{id}/snapshot` must return:

- job metadata
- `snapshot_revision`
- lifecycle state
- renderable solution payload

If no `snapshot_revision` is requested, it returns the latest available
renderable snapshot and echoes that revision.

### Analysis payload

`GET /jobs/{id}/analysis` must:

- accept an optional `snapshot_revision`
- analyze that exact revision when provided
- echo the analyzed `snapshot_revision`

If no revision is provided, it may analyze the latest available snapshot, but it
must still report which revision it analyzed.

### Delete behavior

`DELETE /jobs/{id}` is destructive cleanup only.

It must not mean pause, stop, or cancel.

Deleting a live or pause-requested job must fail explicitly.

## Non-goals

This PRD does not require:

- disk-backed checkpoints
- cross-process checkpoint restore
- distributed solver migration
- domain-specific starter templates
- alternate `schedule` / `stop` command names

## Versioning and Release Governance

### Planned semver classes

This PRD is expected to require coordinated version changes across all three
repos.

Planned version lines:

- `solverforge-rs`: next breaking `0.8.x` line
- `solverforge-ui`: next breaking `0.5.x` line
- `solverforge-cli`: next `1.1.x` line to target the new released contracts

Exact version bumps are not automatic and are not authorized by this PRD alone.

### Governance rules

- changelogs are not to be authored manually in implementation PRs for this
  initiative
- changelog generation remains under `commit-and-tag-version`
- no version bump may be performed by the agent without explicit user
  confirmation at the time the repo is ready
- release execution remains user-managed

### Release order

Release order must follow dependency direction:

1. `solverforge-rs`
2. `solverforge-ui`
3. `solverforge-cli`

Downstream repos must not pin unpublished assumptions from upstream repos.

## Implementation Policy

All implementation for this initiative must happen from clean dedicated
worktrees and repo-specific PR branches.

Dirty existing working directories are not valid implementation bases.

The required execution order is:

1. merge this PRD
2. implement runtime contract in `solverforge-rs`
3. implement shared UI contract in `solverforge-ui`
4. adapt the neutral scaffold in `solverforge-cli`
5. request explicit user confirmation before each repo version bump
6. hand off release execution to the user

## Acceptance Criteria

### Runtime acceptance (`solverforge-rs`)

- exact pause/resume is real, not restart-from-best
- pause settles only when checkpoint is exact and resumable
- reproducible-mode pause/resume matches uninterrupted execution after the pause
  boundary
- lifecycle state distinguishes solving, pause requested, paused, completed,
  cancelled, and failed
- terminal reason is explicit
- `snapshot_revision` and `event_sequence` are monotonic and test-covered
- analysis can target exact snapshot revisions
- `solver.toml` termination budgets are preserved across pause/resume

### Shared UI acceptance (`solverforge-ui`)

- no shared API uses schedule-specific terminology
- pause does not trigger completion callbacks
- completion only follows authoritative terminal state
- analysis during solving does not collapse lifecycle state
- shared solver orchestration is generic and template-agnostic
- snapshot/analysis matching uses `snapshot_revision`

### CLI acceptance (`solverforge-cli`)

- the generated neutral scaffold exposes the new neutral `/jobs` lifecycle
- the generated app boots and solves through the released runtime/UI contracts
- the generated UI relies on shared lifecycle behavior instead of local helper
  orchestration
- runtime and browser tests cover create, pause, resume, cancel, delete, and
  analysis flows
- generated docs distinguish CLI version from runtime/UI targets

### Product acceptance

- no layer in the stack treats analysis availability as proof of completion
- no layer in the stack overloads delete as pause or stop
- no layer in the stack uses schedule-specific lifecycle terminology as the
  shared default
- exact resume is a real runtime capability and not a UI illusion
