# solverforge-cli WIREFRAME

Rails-inspired CLI that scaffolds, generates, and manages SolverForge constraint solver projects in Rust. Replaces the `solverforge` facade crate as the single point of entry for all SolverForge users. Binary name: `solverforge`.

**Location:** `crates/solverforge-cli/`

## Mission

Eliminate `solverforge-quickstarts` as user-facing infrastructure. The CLI becomes the only thing a new user needs:

```
cargo install solverforge-cli
solverforge new my-project --basic=employee-scheduling
cd my-project
solverforge server
```

No cloning repos. No copying quickstarts. No wiring boilerplate. One command, one project, one binary.

## Dependencies

- `clap` v4 (workspace) — CLI argument parsing (derive macros)
- `clap_complete` v4 (workspace) — Shell completion generation (bash, zsh, fish, etc.)
- `dialoguer` (workspace) — Interactive prompts (constraint wizard, destroy confirmation, cargo check prompt)
- `include_dir` (workspace) — Embed templates at compile time
- `owo-colors` (workspace) — Terminal color output (respects `NO_COLOR`)
- `strsim` (workspace) — String similarity for "did you mean?" suggestions
- `toml` (workspace) — TOML parsing and serialization for `config` command and `.solverforgerc`
- `tempfile` (dev-only) — Temp directories for integration tests

## File Map

```
src/
├── main.rs                          — CLI definition (clap derive) and command dispatch
├── error.rs                         — CliError enum, CliResult type alias, keyword validation
├── output.rs                        — Consistent output formatting, NO_COLOR, verbosity control
├── rc.rs                            — .solverforgerc loader: RcConfig, load_rc()
├── template.rs                      — {{key}} placeholder renderer; load_custom() for project overrides
└── commands/
    ├── new.rs                       — Project scaffolding (git init, .gitignore, README, file listing, cargo check prompt)
    ├── server.rs                    — Wraps `cargo run` with --port and --debug support
    ├── console.rs                   — REPL stub: prints guidance to use solver directly
    ├── destroy.rs                   — Resource removal + unwiring with confirmation prompts
    ├── info.rs                      — Project summary: entities, facts, constraints, score type
    ├── check.rs                     — Structural validation of project configuration
    ├── test.rs                      — Wraps `cargo test` with passthrough args
    ├── routes.rs                    — Parses src/api/ for axum .route() calls, prints METHOD/PATH/HANDLER table
    ├── config.rs                    — Show and set keys in solver.toml
    ├── generate_constraint/
    │   ├── run.rs                   — Entry point for constraint generation (--force, --pretend, verbose diff)
    │   ├── domain.rs                — Parses src/domain/*.rs for DomainModel
    │   ├── skeleton.rs              — Generates constraint function bodies by pattern
    │   ├── mod_rewriter.rs          — Updates constraints/mod.rs
    │   ├── wizard.rs                — Interactive pattern selection (dialoguer)
    │   └── utils.rs                 — Name validation & transforms
    ├── generate_domain/
    │   ├── run.rs                   — Entry points per resource type (--force, --pretend, verbose diff)
    │   ├── generators.rs            — Code generation for structs (with custom template lookup + inline test modules)
    │   ├── wiring.rs                — Auto-wires new types into solution struct
    │   └── utils.rs                 — snake_to_pascal, pluralize, validation
    └── generate_scaffold/
        └── run.rs                   — Compound scaffold: entity + constraint + twin entity in one command
```

## Integration Tests

```
tests/
└── scaffold_test.rs   — Scaffolding integration tests; cargo-check tests are #[ignore]
```

## Global Flags

| Flag | Short | Effect |
|------|-------|--------|
| `--quiet` | `-q` | Suppress all output except errors |
| `--verbose` | `-v` | Show extra diagnostic output |
| `--no-color` | — | Disable colored output (also respects `NO_COLOR` env var) |

## CLI Commands

### `solverforge new <PROJECT_NAME> [TEMPLATE]`

Scaffolds a complete SolverForge project from embedded templates.

| Flag | Template | Variable Style |
|------|----------|----------------|
| `--basic` | Generic skeleton | Standard (basic) variable |
| `--basic=employee-scheduling` | Pre-built employee shift scheduling | Standard (basic) variable |
| `--list` | Generic list-variable skeleton | List variable |
| `--list=vehicle-routing` | Pre-built CVRP domain | List variable |

**Optional flags:**
- `--skip-git` — skip `git init` and initial commit
- `--skip-readme` — skip writing `README.md`

**Behavior:**
1. Validates project name (empty, invalid chars, Rust reserved keywords)
2. Copies embedded template tree, rendering `{{project_name}}`, `{{crate_name}}`, `{{solverforge_version}}`
3. Strips `.tmpl` extensions
4. Writes `.gitignore`; writes `README.md` unless `--skip-readme`
5. Prints file listing (Rails-style `create` labels)
6. Runs `git init` + initial commit unless `--skip-git`
7. Prints template-specific next-steps guidance
8. Prompts to run `cargo check` (skipped in `--quiet` mode)

### `solverforge generate <RESOURCE> [ARGS]`

Generates resources into the current project.

| Subcommand | Args | Effect |
|------------|------|--------|
| `constraint <name>` | `--unary/--pair/--join/--balance/--reward`, `--hard/--soft`, `--force`, `--pretend` | Generates constraint function, wires into constraints/mod.rs |
| `entity <name>` | `--planning-variable FIELD`, `--field NAME:TYPE` (repeatable), `--force`, `--pretend` | Creates `#[planning_entity]` struct with optional extra fields and inline test module, wires into solution |
| `fact <name>` | `--field NAME:TYPE` (repeatable), `--force`, `--pretend` | Creates `#[problem_fact]` struct with optional extra fields and inline test module, wires into solution |
| `solution <name>` | `--score SCORE_TYPE` | Creates `#[planning_solution]` struct |
| `variable <field>` | `--entity ENTITY` | Adds `#[planning_variable]` field to existing entity |
| `score <type>` | — | Changes score type in existing solution |
| `scaffold <name> [fields]` | `--entity`, `--constraint NAME`, `--pair`, `--force`, `--pretend` | Compound generator: entity + optional constraint + optional twin entity in one command |

**Common flags:**
- `--force` / `-f` — Overwrite if resource already exists
- `--pretend` — Preview changes without writing files (dry-run)

### `solverforge destroy <RESOURCE> <NAME>`

Removes a resource and unwires it from mod files and solution struct.

| Flag | Effect |
|------|--------|
| `--yes` / `-y` | Skip confirmation prompt |

| Subcommand | Effect |
|------------|--------|
| `solution` | Removes planning solution struct, unwires from mod.rs |
| `entity <name>` | Removes entity struct, unwires from solution + mod.rs |
| `fact <name>` | Removes fact struct, unwires from solution + mod.rs |
| `constraint <name>` | Removes constraint file, unwires from constraints/mod.rs |

**Confirmation:** All destroy operations prompt for confirmation by default. Use `--yes` to skip.

### `solverforge server`

Runs `cargo run` in the current project directory.

| Flag | Default | Effect |
|------|---------|--------|
| `--port` / `-p` | 7860 | Port to bind the server to (sets `PORT` env var) |
| `--debug` | false | Run in debug mode (faster compilation, slower runtime) |

### `solverforge info`

Displays project summary: solution type, score type, entities with planning variables, facts, constraints, and config file status.

### `solverforge check`

Validates project structure: domain model parsability, entity planning variables, constraint file consistency, solver.toml presence, domain/mod.rs integrity.

### `solverforge test [-- ARGS]`

Wraps `cargo test`. All arguments after `--` are passed directly to cargo test.

### `solverforge routes`

Parses `src/api/routes.rs`, `src/api/mod.rs`, or `src/api.rs` for axum `.route()` calls and prints a formatted METHOD / PATH / HANDLER table.

### `solverforge config <SUBCOMMAND>`

Manages `solver.toml` in the current project directory.

| Subcommand | Args | Effect |
|------------|------|--------|
| `show` | — | Print the full contents of `solver.toml` |
| `set <KEY> <VALUE>` | dotted key path, new value | Edit a key in `solver.toml` (integer, float, bool, or string auto-detected) |

### `solverforge console`

Stub command. Prints guidance explaining that the interactive console is not yet available and lists alternatives (`solverforge server`, `cargo run`, `solverforge test`).

### `solverforge completions <SHELL>`

Generates shell completion scripts. Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

## Error Handling

### CliError Enum

Structured error type (`src/error.rs`) with variants:

| Variant | Context |
|---------|---------|
| `DirectoryExists` | Project name already taken |
| `InvalidProjectName` | Bad chars, empty, Rust keyword |
| `ReservedKeyword` | Rust reserved word as project name |
| `NotInProject` | Missing `src/domain/` or `src/constraints/` with remediation hint |
| `ResourceExists` | Entity/fact/constraint already exists (use `--force`) |
| `ResourceNotFound` | Destroy target not found |
| `InvalidName` | snake_case validation failure |
| `InvalidScoreType` | Unknown score type with known list |
| `IoError` | File I/O with context string |
| `SubprocessFailed` | Cargo subprocess exit failure |
| `General` | Catch-all with optional hint |

### NO_COLOR Support

Respects `NO_COLOR` env var (https://no-color.org/) and `--no-color` flag. All output formatting goes through `output.rs` which checks the global flag before applying colors.

### Output Consistency

All output uses Rails-style verb labels via `output.rs`:

| Label | Usage |
|-------|-------|
| `create` | New file written |
| `update` | Existing file modified |
| `remove` | File deleted |
| `invoke` | External tool called (e.g., `git init`) |
| `skip` | Operation skipped (e.g., user declined destroy) |
| `start` | Server or subprocess starting |
| `check` | Validation step |

### Subcommand Inference

`infer_subcommands = true` — `solverforge gen` resolves to `solverforge generate`, `solverforge ser` to `solverforge server`, etc.

## Template System

Templates live in `templates/` and are embedded at compile time via `include_dir!`.

```
templates/
├── basic/
│   ├── generic/                — Standard variable scaffold
│   │   ├── src/domain/         — plan, resource, task stubs
│   │   ├── src/constraints/    — all_assigned constraint
│   │   ├── src/solver/         — SolverManager-based service
│   │   ├── src/api/            — routes, DTOs
│   │   ├── src/data/           — demo plan
│   │   └── solver.toml         — termination config (30s)
│   └── employee-scheduling/    — Rich template: domain, constraints, API, UI, data
│       ├── src/domain/         — employee, shift, schedule (annotated)
│       ├── src/constraints/    — 7 constraint patterns (balance, overlap, skill, etc.)
│       ├── src/solver/         — SolverManager-based service
│       ├── src/api/            — routes, DTOs
│       ├── src/data/           — CSV generator/parser
│       ├── static/             — Web UI (timeline renderer, AJAX client)
│       └── solver.toml         — termination config (30s)
└── list/
    └── vehicle-routing/        — List variable template: domain, solver, API, UI
        ├── src/domain/         — vehicle (list variable), problem (cvrp), plan (VrpSolution)
        ├── src/constraints/    — capacity (hard) + totalDistance (soft)
        ├── src/solver/         — SolverManager-based service
        ├── src/api/            — routes, DTOs
        ├── src/data/           — demo instance
        └── solver.toml         — CW construction + late-acceptance (60s)
```

Scaffolded projects additionally receive:
- `.gitignore` — excludes `target/`, `*.rs.bk`, `Cargo.lock`
- `README.md` — project overview, quick start, development commands, structure table

**Renderer** (`template.rs`): Recursive `{{key}}` substitution. Strips `.tmpl` extensions during copy.

## RC Config (`.solverforgerc`)

`src/rc.rs` — `load_rc() -> CliResult<RcConfig>`

Loaded at startup before CLI arg parsing. Project root `.solverforgerc` takes precedence over `~/.solverforgerc`. CLI flags always override rc file values.

| Key | Type | Effect |
|-----|------|--------|
| `default_template` | string | Default template (e.g. `"basic/employee-scheduling"`) |
| `port` | integer (1–65535) | Default server port |
| `no_color` | bool | Disable colored output |
| `quiet` | bool | Suppress all output except errors |

Invalid TOML or missing file silently returns defaults.

## Custom Generator Templates (`.solverforge/templates/`)

`template::load_custom(name, vars)` — checks `.solverforge/templates/<name>.rs.tmpl` in the project root before falling back to built-in generation.

Template variable syntax: `{{NAME}}`, `{{SNAKE_NAME}}`, `{{FIELDS}}`.

| Template file | Overrides |
|---------------|-----------|
| `entity.rs.tmpl` | `generate entity` code body |
| `fact.rs.tmpl` | `generate fact` code body |
| `solution.rs.tmpl` | `generate solution` code body |

## Domain Parsing

`generate_constraint/domain.rs` parses Rust source files line-by-line for annotations:

| Annotation | Extracted Info |
|------------|----------------|
| `#[planning_solution]` | Solution struct name, score type |
| `#[planning_entity]` | Entity struct name |
| `#[planning_entity_collection]` | Collection field linking entity to solution |
| `#[problem_fact]` | Fact struct name |
| `#[problem_fact_collection]` | Collection field linking fact to solution |
| `#[planning_variable]` | Variable field name and type |

Falls back to `mod_rewriter::extract_types()` (regex-based) if annotation parsing fails.

## Wiring Chain

When generating entities/facts (`generate_domain/wiring.rs`):

1. Write code file → `src/domain/<name>.rs`
2. Update `src/domain/mod.rs` → `mod <name>;` + `pub use <name>::<PascalName>;`
3. Wire into solution struct:
   - Import statement
   - Field with `#[planning_entity_collection]` or `#[problem_fact_collection]`
   - Constructor parameter
   - `Self { <name>: <param>, ... }` initializer entry

## Mod Rewriting

`generate_constraint/mod_rewriter.rs` handles two mod.rs shapes:

| Shape | Structure | When Used |
|-------|-----------|-----------|
| Assemble | Nested mods, constraints as submodules | Templates with organized constraint dirs |
| Flat | All constraints declared at top level | Simpler projects |

Operations: insert constraint declaration, extend constraint tuple.

## Naming Conventions

| Context | Convention | Example |
|---------|-----------|---------|
| User input | snake_case | `shift`, `no_overlap` |
| Struct names | PascalCase via `snake_to_pascal()` | `Shift`, `NoOverlap` |
| Collection fields | Pluralized snake_case via `pluralize()` | `shifts`, `employees` |
| Filenames | snake_case | `shift.rs`, `no_overlap.rs` |
| Crate names | Validated via `to_crate_name()` | `my-project` → `my_project` |

## Constraint Patterns

Five skeleton patterns generated by `generate_constraint/skeleton.rs`:

| Pattern | Stream Shape | Use Case |
|---------|-------------|----------|
| Unary | `for_each` → `filter` → `penalize` | Single-entity property check |
| Pair | `for_each_unique_pair` → `filter` → `penalize` | Conflict between two entities |
| Join | `for_each` → `join` → `filter` → `penalize` | Entity-fact relationship |
| Balance | `for_each` → `group_by` → `penalize` | Even distribution across entities |
| Reward | `for_each` → `filter` → `reward` | Positive scoring for matches |

Hardness: `--hard` or `--soft` flag controls score impact.

## Architectural Notes

- **Pure CLI crate.** No runtime dependency on solverforge-rs. Templates are embedded at compile time. The CLI generates Rust code that depends on solverforge-rs, but the CLI binary itself does not link against the solver.
- **String-based code generation.** Templates and generators use string concatenation, not AST manipulation. This keeps the CLI lightweight and the generated code readable.
- **Line-by-line parsing.** Domain model extraction is regex/line-based, not a full Rust parser. Sufficient for the annotation-driven patterns SolverForge uses.
- **Structured error handling.** All commands return `CliResult<()>`. `CliError` enum carries context (which file, which resource) and remediation hints. Main prints errors via `output::print_error()` and exits with code 1.
- **NO_COLOR compliance.** Respects `NO_COLOR` env var and `--no-color` flag per https://no-color.org/.
- **No `solverforge` crate dependency.** The CLI does not depend on the solverforge facade crate. It generates projects that depend on it. This is intentional — the CLI is a development tool, not a runtime library.

## Cross-Crate Dependencies

```
solverforge-cli (this crate)
  ├── generates projects that depend on:
  │   ├── solverforge (facade only — no sub-crate imports)
  │   ├── axum, serde, tokio, uuid, parking_lot (all templates)
  │   └── chrono (employee-scheduling only)
  └── has no path/workspace dependency on solverforge-rs crates
```
