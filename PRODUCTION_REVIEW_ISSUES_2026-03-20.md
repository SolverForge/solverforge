# Suggested issues from the production review — 2026-03-20

This file turns the findings from `PRODUCTION_REVIEW_2026-03-20.md` into a concrete set of issues that can be opened and tracked.

---

## Issue 1 — Make `solverforge-cli` pass strict Clippy

- **Priority:** P0
- **Affected crates:** `solverforge-cli`
- **Why this matters:** the documented strict lint gate currently fails, which weakens confidence in release readiness and CI signal quality.
- **Problem statement:** `cargo clippy --workspace --all-targets -- -D warnings` fails because two test modules trigger `clippy::module_inception`.
- **Suggested work:**
  - Rename the nested CLI test modules or flatten them.
  - Run Clippy again and make the workspace lint-clean.
  - Add a CI/local validation step that keeps this from regressing.
- **Acceptance criteria:**
  - `cargo clippy --workspace --all-targets -- -D warnings` passes.
  - No `#[allow(clippy::module_inception)]` escape hatch is added unless documented and justified.

## Issue 2 — Gate or implement the `solverforge console` command

- **Priority:** P1
- **Affected crates:** `solverforge-cli`, possibly `solverforge-console`
- **Why this matters:** users see a public command that explicitly says it is not available yet.
- **Problem statement:** `solverforge console` is exposed in the CLI but currently only prints that the feature is not yet available.
- **Suggested work:**
  - Either implement a minimal supported interactive workflow, or remove/gate the command until it exists.
  - Align CLI help text and generated documentation with the actual support level.
- **Acceptance criteria:**
  - The command is either functional and supported, or no longer exposed as a normal user-facing command.
  - Help output does not advertise unfinished functionality as if it were complete.

## Issue 3 — Finish or hide the generic list-variable scaffold

- **Priority:** P1
- **Affected crates:** `solverforge-cli`
- **Why this matters:** `--list` is advertised to users but intentionally errors at runtime.
- **Problem statement:** the generic list-variable template is marked “coming soon” and is not actually available.
- **Suggested work:**
  - Implement the generic list scaffold, or
  - Hide/remove the flag until the template exists.
- **Acceptance criteria:**
  - `solverforge new --list` either succeeds with a supported template or is not presented as an available option.
  - CLI docs/examples match runtime behavior.

## Issue 4 — Make generated constraints compile without raw `todo!()` placeholders

- **Priority:** P1
- **Affected crates:** `solverforge-cli`
- **Why this matters:** generated code that panics or does not compile cleanly feels unfinished and slows first-time users.
- **Problem statement:** generated constraint skeletons include multiple `todo!()` placeholders in executable code paths.
- **Suggested work:**
  - Replace raw `todo!()` calls with safer compile-time placeholders, explicit `unimplemented!()` markers with better guidance, or commented examples that keep the generated crate buildable.
  - Improve post-generation instructions.
- **Acceptance criteria:**
  - Newly generated constraints are clearer and safer for end users.
  - The generated code path is intentionally designed, documented, and tested.

## Issue 5 — Replace scaffolded data-loader stubs with compile-safe templates

- **Priority:** P1
- **Affected crates:** `solverforge-cli`
- **Why this matters:** generated projects should be as close as possible to compile-and-run quality.
- **Problem statement:** domain generation rewrites `src/data/mod.rs` with a `todo!("Implement data loading")` stub.
- **Suggested work:**
  - Generate a compile-safe placeholder implementation.
  - Add clear comments for where users should insert domain-specific loading code.
  - Add scaffold smoke tests covering this path.
- **Acceptance criteria:**
  - Generated projects compile cleanly after domain generation, or failures are deliberate, obvious, and well-explained.
  - Scaffold tests cover the generated data module.

## Issue 6 — Add direct unit tests for `solverforge-cvrp`

- **Priority:** P1
- **Affected crates:** `solverforge-cvrp`
- **Why this matters:** the crate exposes public helpers used in routing scenarios but currently appears untested directly.
- **Problem statement:** `solverforge-cvrp` has no visible direct test module/files despite containing nontrivial routing/time-feasibility logic.
- **Suggested work:**
  - Add unit tests for distance helpers, route assignment helpers, and time-window feasibility.
  - Add edge-case tests for empty fleets, invalid route positions, and time-window boundaries.
- **Acceptance criteria:**
  - `solverforge-cvrp` has direct tests in the crate.
  - Core helper behavior is covered by deterministic unit tests.

## Issue 7 — Reduce unsafe surface and duplicate wrappers in `solverforge-cvrp`

- **Priority:** P1
- **Affected crates:** `solverforge-cvrp`, `solverforge`
- **Why this matters:** raw-pointer-based helpers and redundant wrappers raise maintenance and safety risk in a public helper API.
- **Problem statement:** the crate relies on `*const ProblemData` and repeated `unsafe` dereferences, while also exposing duplicate thin wrappers such as `assign_route` and `set_route`.
- **Suggested work:**
  - Consolidate duplicate route helpers.
  - Document safety invariants much more rigorously.
  - Investigate safer ownership/reference patterns where possible.
  - Re-evaluate what should be publicly re-exported from the façade crate.
- **Acceptance criteria:**
  - Duplicate wrapper surface is reduced.
  - Safety invariants are explicit and tested.
  - The public API is easier to reason about.

## Issue 8 — Add direct tests for `solverforge-console`

- **Priority:** P2
- **Affected crates:** `solverforge-console`
- **Why this matters:** global tracing/subscriber setup is subtle and easy to regress even in a small crate.
- **Problem statement:** `solverforge-console` appears to have no direct tests.
- **Suggested work:**
  - Add smoke tests for idempotent initialization.
  - Add tests for filter defaults and formatting behavior where practical.
- **Acceptance criteria:**
  - The crate has at least a minimal direct test suite.
  - Repeated initialization behavior is explicitly covered.

## Issue 9 — Add compile-fail / golden tests for `solverforge-macros`

- **Priority:** P2
- **Affected crates:** `solverforge-macros`, possibly `solverforge`
- **Why this matters:** proc macros benefit from direct expansion/regression tests rather than relying only on downstream behavior.
- **Problem statement:** macro validation appears to be mostly indirect.
- **Suggested work:**
  - Add compile-pass and compile-fail tests for key macro attributes.
  - Add snapshot/golden tests for expected expansion behavior where practical.
- **Acceptance criteria:**
  - Macro-specific regression coverage exists.
  - Common authoring mistakes produce stable, intentional diagnostics.

## Issue 10 — Publish a crate stability matrix and release checklist

- **Priority:** P2
- **Affected crates:** workspace-wide
- **Why this matters:** users need to know which crates/features are mature, experimental, or template-oriented.
- **Problem statement:** readiness varies significantly across crates, but that variance is not captured in a standard release/stability document.
- **Suggested work:**
  - Add a workspace-level matrix for stable/experimental/template-only surfaces.
  - Define a release checklist that includes `fmt`, `clippy`, `test`, and scaffold smoke validation.
- **Acceptance criteria:**
  - Stability/readiness expectations are documented.
  - Release validation is repeatable and explicit.

## Issue 11 — Add scaffold smoke tests for generated projects and workflows

- **Priority:** P2
- **Affected crates:** `solverforge-cli`
- **Why this matters:** the CLI is user-facing and should be validated end-to-end, not only by string/template unit tests.
- **Problem statement:** current validation catches many pieces, but there is still a gap between generated output and polished compile-ready projects.
- **Suggested work:**
  - Add smoke tests for `solverforge new`, entity/fact/constraint generation, and follow-on `cargo check` in representative cases.
  - Cover both basic and list-oriented project flows as they mature.
- **Acceptance criteria:**
  - Representative generated projects are exercised in automated tests.
  - Regressions in scaffolding behavior are caught before release.
