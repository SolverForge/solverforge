# Constraints and scores

Constraints are written as fluent Rust streams over the model's collections.
SERIO, the incremental scoring engine, keeps each stream's state between moves
and propagates a move only through the constraints it changes. This guide
covers score types, the stream operators, joins, score analysis and runtime
weights. To write new
operators or a custom `IncrementalConstraint`, read
[Extending typed relational scoring](extend-scoring.md).

## Score types

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

A solution is feasible when no hard level is negative.

## What streams can do

Start from `ConstraintFactory::for_each(...)` on a generated collection source,
then filter, join, group and score. The stream API covers:

- model-owned collection sources and filtered join sources;
- keyed joins of any arity, including three-source keyed joins
  (`stream::cross::Tri`);
- single-source and cross-join projected scoring rows, and symmetric and
  directed projected self-joins;
- existence checks;
- grouping with collectors such as `collect_vec`, `consecutive_runs` and
  `indexed_presence`, including direct cross-join grouping;
- grouped complements and projected grouped complements for supply/demand
  scoring, plus balance streams.

## Grouping and collectors

A grouped scoring weight receives both the group key and the collected result:

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

How grouping behaves:

- Collectors own the values they retain. The same collector protocol covers
  single rows and joined tuples.
- `Tri`, `Quad` and `Penta` cross streams, and the same-source streams of those
  arities, expose `.group_by(key, collector)`. The key receives three, four or
  five entity arguments; the collector mapper receives the matching borrowed
  tuple.
- Grouped scoring weights receive `(&key, &result)`.
- Filters before the grouping stay binding in both full and incremental
  scoring.
- Direct cross-join grouped pairs and projected grouped streams can continue
  into `complement(...)`.
- `collect_vec` returns a `CollectedVec<T>` view, so payloads such as `String`
  labels do not need `Copy`, `Clone` or `PartialEq` just to be collected.

## Shared grouped terminals

Annotate constraint functions with `#[solverforge_constraints]`. You keep
writing the same fluent Rust, and SolverForge gets a whole-function compiler
boundary. When one grouped stream binding feeds several terminal constraints,
the compiler builds a single shared incremental node for it and keeps each
terminal separately named:

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

The three terminals share the grouped work. Their identity, ordering, metadata
and score explanation stay independent.

## Joins

Each `.join((target, condition))` carries its own key or predicate type. A later
join's left key receives the whole borrowed row, so it can read any earlier
binding or combine several. This constraint relates assignments to shifts on a
`u32` shift id, then to employees on a `String` employee code, two independent
key domains in one chain:

```rust
use solverforge::prelude::*;
use solverforge::stream::joiner::{equal_bi, equal_on};
use solverforge::stream::relational::{Leaf, operator::Pair};

fn night_shift_staffed() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(Schedule::assignments())
        // Relationship 1 (u32): assignment.shift_id == shift.id
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new().for_each(Schedule::shifts()),
            equal_bi(
                |a: &Assignment| a.shift_id,
                |s: &Shift| s.id,
            ),
        ))
        // Relationship 2 (String): the whole (assignment, shift) row
        // joined to an employee on the employee code.
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new().for_each(Schedule::employees()),
            equal_on(
                |row: &Pair<Leaf<'_, Assignment>, Leaf<'_, Shift>>| {
                    row.left.entity.employee_code.clone()
                },
                |e: &Leaf<'_, Employee>| e.entity.code.clone(),
            ),
        ))
        .filter(|_a: &Assignment, s: &Shift, _e: &Employee| s.night)
        .penalize(SoftScore::of(1))
        .named("night shift staffed")
}
```

### Conditions

Every relationship picks its own condition, and conditions compose:

| Condition | How candidates are found |
|-----------|--------------------------|
| Equality (`equal_bi`, `equal_on`) | Indexed lookup |
| Comparison (`less_than`, `greater_than`, and their `_or_equal` forms) | Own index |
| Interval `overlapping` | Own index |
| Arbitrary predicate | Scan of the retained opposite rows |

Combine conditions with `.and(...)`. `equal_bi(..).and(less_than(..))` indexes
the equality part and checks the comparison as an exact residual. Fluent
right-hand targets implement `CollectionExtract`, which includes filtered unary
streams. At the low-level operator layer either `JoinNode` input may be a
compatible derived producer.

### Filter before the first equality join

On the first unary keyed equality cross-join, filters on a unary source drop
rows before key extraction and pair creation. Filters written after the join
operate on joined rows. This pushdown applies only to that equality dispatch;
comparison, overlap, composed-condition, predicate and same-source self-join
dispatch do not get it. [Extend the domain](extend-domain.md#filter-nullable-scoring-sources-before-equality-joins)
shows the nullable-assignment case this matters for.

### Arity

There is no arity ceiling. `Uni`, `Bi`, `Tri`, `Quad` and `Penta` are ergonomic
adapters; `stream::cross::Penta::join` continues into the arity-free `Chain`,
whose `.join(...)` nests further typed joins as deep as needed. Filters on the
cross `Bi`, `Tri`, `Quad` or `Penta` stream apply once at the penta boundary
and carry into the `Chain`. The same-source `PentaConstraintStream` is
terminal.

## Score analysis

Analyze a solution without solving it:

```rust
use solverforge::analyze;

let analysis = analyze(&solution);

println!("Score: {}", analysis.score);
for constraint in &analysis.constraints {
    println!("  {}: {}", constraint.name, constraint.score);
}
```

Each `ConstraintAnalysis` reports the weighted `score` and the constraint's
`weight_multiplier`, so a disabled constraint (`0`) is distinguishable from a
satisfied one. For analysis of a running job, see
[Running solves](running-solves.md).

## Runtime constraint weights

Mark one `ConstraintWeights` field with `#[constraint_weights]` to scale named
constraints per solution without recompiling:

```rust
#[planning_solution(constraints = "define_constraints")]
pub struct Schedule {
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    #[constraint_weights]
    pub weights: ConstraintWeights,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
```

`validate_constraint_weights` runs the same checks a solve runs at start, so a
typo is caught before the job is submitted:

```rust
let mut weights = ConstraintWeights::new();
weights.set("Travel time", 10).set("Overtime", 0);
Schedule::validate_constraint_weights(&weights)?;
```

The rules:

- Multipliers are non-negative integers keyed by constraint name.
- An unlisted constraint keeps its coded weight (`1`), `0` disables a
  constraint, and `n` scales its contribution exactly `n` times, hard levels
  included.
- A solve whose weights name an unknown constraint, or exceed
  `MAX_CONSTRAINT_WEIGHT_MULTIPLIER`, fails at start through the normal
  `Failed` lifecycle event.
- Weights cost nothing until they change a score. When every multiplier is
  `1`, the solve runs on the unweighted score director, the same code path as
  a model without a weights field. Only multipliers other than `1` select the
  weighted director.

For fine-grained sliders, author base weights at the finest granularity and
ship baseline multipliers in the data, for example `{"Travel time": 10}` with a
slider over `0..=20`.

To change weights while a job runs, submit a problem change that replaces the
field. The solver rescores under the new multipliers and restarts its phase
sequence. A change that crosses between all-`1` and other multipliers moves the
job to the matching score director without losing its statistics, limits or
event stream. Weights installed this way are validated the same way, and an
invalid set fails the job:

```rust
MANAGER.add_problem_change(
    job_id,
    ClosureProblemChange::<Schedule, _>::new("reweight", move |director| {
        director.working_solution_mut().weights = weights.clone();
    }),
)?;
```
