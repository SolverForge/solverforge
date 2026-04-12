# Ruby Modeling Integration Plan (Zero-Erasure, No Hot-Path Validation)

This document defines the implementation plan for enabling full SolverForge problem modeling from Ruby while preserving SolverForge's performance architecture.

## Goals

1. Preserve monomorphization and zero-erasure behavior in all solver hot paths.
2. Allow Ruby users to model the same classes of problems that Rust users can model.
3. Avoid all per-move, per-score-calculation, and per-constraint runtime validation.
4. Support real-time web streaming (SSE/WebSocket) and telemetry (including moves/steps per second).

## Non-Negotiable Performance Invariants

- No Ruby callbacks from the solving hot path.
- No runtime schema/type validation in move evaluation or scoring loops.
- No interpreted constraint execution in the hot path.
- No dynamic type-erasure introduced beyond existing intentional boundaries in the solver.

Validation must occur only at cold boundaries:

- build time (DSL -> generated Rust -> compile)
- model load time
- problem ingestion time (once per submission)

After ingestion, execution is fully native over typed Rust structures.

## Architecture Overview

The integration has three layers:

1. `solverforge-ruby` gem (authoring + orchestration)
2. code generation bridge (Ruby DSL -> Rust crate)
3. native runtime (`cdylib`) loaded by Ruby

### Layer 1: Ruby Authoring DSL

Ruby users define:

- planning entities and facts
- planning solution fields
- constraints
- solver configuration and termination settings

The DSL compiles into a canonical model IR (internal gem representation), then emits Rust source code.

### Layer 2: Rust Codegen Crate

The generated crate uses SolverForge's public API directly:

- `#[planning_solution]`, `#[planning_entity]`, `#[problem_fact]`
- `ConstraintFactory` and stream joins/filters/penalties
- `SolverManager` lifecycle APIs

The crate compiles into a shared library and is cached by:

- model hash
- SolverForge version
- target triple
- selected feature flags

### Layer 3: Native Runtime Boundary

Ruby interacts with native code through coarse-grained calls only, for example:

- `build_model`
- `solve`
- `pause` / `resume` / `cancel`
- `get_snapshot`
- `analyze_snapshot`
- `next_event` / event stream iterator

There is no per-move crossing of the Ruby/native boundary.

## Feature Parity Strategy (Ruby ~= Rust modeling power)

To match Rust modeling power without runtime penalties:

- Ruby DSL is declarative and compiles to Rust expressions.
- Advanced constraints are represented as codegen templates, not interpreted lambdas.
- Escape hatch: user-defined native extension snippets are generated into Rust modules when DSL abstraction is insufficient.

This preserves performance and avoids introducing compatibility shims in hot code.

## Validation Policy

### Allowed validation points

1. DSL compile/emit phase:
   - structure and naming checks
   - unsupported construct detection
2. Rust compile phase:
   - type correctness and trait bounds
3. Job submission ingest phase:
   - payload shape and ID consistency

### Forbidden validation points

- move selector iteration
- score calculation loops
- incremental constraint propagation
- accepted move application

## Eventing, Snapshots, and Streaming

The runtime exposes lifecycle events and snapshot revisions suitable for web UIs.

### Snapshot model

- Events include snapshot revision metadata for best/progress states.
- UI fetches immutable snapshots by revision.
- Analysis calls are always revision-bound.

### SSE/WebSocket model

- Server relays native events to SSE/WebSocket clients.
- Event payload includes lifecycle state, telemetry counters, snapshot revision, and terminal reason.

### Moves/steps per second

- Compute from telemetry deltas (`step_count` over wall-clock interval).
- Provide both instantaneous and rolling rates in server/UI layers.

## Implementation Phases

## Phase 0 (Foundation)

- Create `solverforge-ruby` repository skeleton.
- Set up native extension build pipeline (`cdylib`) and cache directory strategy.
- Define canonical IR schema used internally by the gem.

Deliverables:

- build command (`solverforge-ruby build`)
- deterministic model hash
- integration smoke test that compiles and loads a trivial model

## Phase 1 (Core Modeling Surface)

- Ruby DSL for entities/facts/solution and basic constraint streams.
- Codegen for uni/bi streams, join/filter, hard/soft penalties.
- Single-job solve with event polling and final solution retrieval.

Deliverables:

- parity with quick-start class of examples
- snapshot retrieval and basic telemetry

## Phase 2 (Lifecycle + Streaming)

- pause/resume/cancel integration
- revision-bound snapshot analysis API
- SSE and WebSocket relay adapters

Deliverables:

- dashboard-ready event stream
- robust reconnect behavior with last-seen revision

## Phase 3 (Advanced Parity)

- list-variable modeling and advanced move-family configuration
- nearby selectors and richer phase composition
- native escape-hatch modules for edge constraints

Deliverables:

- Ruby can model advanced list-heavy planning scenarios without falling back to interpreted scoring

## Observability and Performance Gates

The integration cannot ship without meeting all gates:

1. Zero hot-path validations verified by profiling/tracing.
2. No Ruby callback frames inside move scoring/evaluation traces.
3. Throughput baseline within agreed budget versus equivalent Rust-native model.
4. Event streaming overhead bounded and configurable.

## Risk Register

1. DSL expressiveness drift from Rust surface
   - Mitigation: codegen from canonical templates aligned to wireframed public APIs.
2. Build latency from on-demand codegen/compile
   - Mitigation: aggressive cache reuse and prebuild hooks.
3. Runtime feature skew across targets
   - Mitigation: target matrix CI with fixture models and golden snapshots.

## Acceptance Criteria

- Ruby model definitions compile to typed Rust artifacts.
- Solver runs with no Ruby calls in hot loops.
- Snapshot lifecycle works end-to-end.
- SSE/WebSocket events are consumable by web clients.
- Reported moves/steps-per-second derives from native telemetry counters.
