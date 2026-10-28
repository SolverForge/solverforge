/* Scoring cases: incremental score-director churn over a fixed model with one
uni, one join (`OperatorTerminal`), two compiler-style shared grouped
terminals (`ScorerChain`), and one balance constraint.

Unlike the selector cases (which run against an empty constraint set), these
measure incremental scoring through every constraint family that constraint
weights touch:
- `scoring`: a solution type without constraint weights on either side, so
  the candidate director compiles to the unweighted scoring calls;
- `scoring_weights_one`: the candidate's solution type carries weights set
  to multiplier 1 explicitly, so the director is chosen exactly as a solve
  chooses it (`requires_weighted_mode`): the unweighted director, running the
  same code as `scoring`; the baseline (no weight API) runs unweighted;
- `scoring_weights_mixed`: the candidate binds multipliers 0 and 3 on a
  weighted director and takes the weighted per-constraint calls; reported for
  information only, since its scores intentionally differ.

Weighted code compiles only under the harness-only `constraint_weights`
feature, which the gate enables for the candidate side alone, so baselines
that predate the weight API still build and the `scoring` case never links
it. The cross-tree order hash covers
only the move sequence; scores feed the
per-tree identity hash, so weighted variants stay comparable. Run with
`BENCH_CASES="scoring scoring_weights_one scoring_weights_mixed"`.
*/

use solverforge_core::score::HardSoftScore;
use solverforge_scoring::stream::collection_extract::{source, ChangeSource};
use solverforge_scoring::stream::collector::sum;
use solverforge_scoring::stream::joiner::equal_bi;
use solverforge_scoring::stream::ConstraintFactory;
use solverforge_scoring::ConstraintSet;

const SCORING_SHIFTS: usize = 512;
const SCORING_EMPLOYEES: usize = 24;
const SCORING_CHANGES: usize = 4_096;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ScoringShift {
    employee: Option<usize>,
    hours: i64,
}

#[derive(Clone, Debug)]
struct ScoringEmployee {
    id: usize,
    capacity: i64,
}

#[derive(Clone, Debug)]
struct ScoringPlan {
    shifts: Vec<ScoringShift>,
    employees: Vec<ScoringEmployee>,
    #[cfg(feature = "constraint_weights")]
    weights: Option<solverforge_core::ConstraintWeights>,
    score: Option<HardSoftScore>,
}

impl PlanningSolution for ScoringPlan {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    #[cfg(feature = "constraint_weights")]
    const HAS_CONSTRAINT_WEIGHTS: bool = true;

    #[cfg(feature = "constraint_weights")]
    fn constraint_weights(&self) -> Option<&solverforge_core::ConstraintWeights> {
        self.weights.as_ref()
    }
}

fn scoring_shifts(plan: &ScoringPlan) -> &[ScoringShift] {
    plan.shifts.as_slice()
}

fn scoring_employees(plan: &ScoringPlan) -> &[ScoringEmployee] {
    plan.employees.as_slice()
}

fn scoring_constraints() -> impl ConstraintSet<ScoringPlan, HardSoftScore> {
    let shifts = || {
        source(
            scoring_shifts as fn(&ScoringPlan) -> &[ScoringShift],
            ChangeSource::Descriptor(0),
        )
    };
    let factory = ConstraintFactory::<ScoringPlan, HardSoftScore>::new;
    (
        factory()
            .for_each(shifts())
            .filter(|shift: &ScoringShift| shift.employee.is_none())
            .penalize(HardSoftScore::of_hard(1))
            .named("Unassigned"),
        factory()
            .for_each(shifts())
            .join((
                source(
                    scoring_employees as fn(&ScoringPlan) -> &[ScoringEmployee],
                    ChangeSource::Static,
                ),
                equal_bi(
                    |shift: &ScoringShift| shift.employee,
                    |employee: &ScoringEmployee| Some(employee.id),
                ),
            ))
            .filter(|shift: &ScoringShift, employee: &ScoringEmployee| {
                shift.hours > employee.capacity
            })
            .penalize(|shift: &ScoringShift, employee: &ScoringEmployee| {
                HardSoftScore::of_soft(shift.hours - employee.capacity)
            })
            .named("Over capacity"),
        factory()
            .for_each(shifts())
            .group_by(
                |shift: &ScoringShift| shift.employee.unwrap_or(usize::MAX),
                sum(|shift: &ScoringShift| shift.hours),
            )
            .penalize(|_employee: &usize, hours: &i64| HardSoftScore::of_soft(hours * hours))
            .named("Load")
            .reward(|_employee: &usize, hours: &i64| HardSoftScore::of_soft(*hours))
            .named("Coverage"),
        factory()
            .for_each(shifts())
            .balance(|shift: &ScoringShift| shift.employee)
            .penalize(HardSoftScore::of_soft(100))
            .named("Balance"),
    )
}

#[cfg(feature = "constraint_weights")]
fn scoring_weights(entries: &[(&str, u32)]) -> Option<solverforge_core::ConstraintWeights> {
    let mut weights = solverforge_core::ConstraintWeights::new();
    for (name, multiplier) in entries {
        weights.set(*name, *multiplier);
    }
    Some(weights)
}

fn scoring_plan(_entries: Option<&[(&str, u32)]>) -> ScoringPlan {
    ScoringPlan {
        shifts: (0..SCORING_SHIFTS)
            .map(|index| ScoringShift {
                employee: (index % 7 != 0).then_some(index % SCORING_EMPLOYEES),
                hours: (index % 9) as i64 + 1,
            })
            .collect(),
        employees: (0..SCORING_EMPLOYEES)
            .map(|id| ScoringEmployee {
                id,
                capacity: (id % 5) as i64 + 3,
            })
            .collect(),
        #[cfg(feature = "constraint_weights")]
        weights: _entries.and_then(scoring_weights),
        score: None,
    }
}

/* Incremental churn over one director. A macro rather than a generic fn so
the harness names no weight-mode type and still builds against trees that
predate the weight API. */
macro_rules! churn_scoring {
    ($case:expr, $director:expr) => {{
        let mut director = $director;
        black_box(director.calculate_score());
        emit($case, SCORING_CHANGES, || {
            let mut count = 0;
            let mut order = 0xCBF2_9CE4_8422_2325;
            let mut scores = 0xCBF2_9CE4_8422_2325;
            for _ in 0..iterations() {
                for change in 0..SCORING_CHANGES {
                    let shift = change.wrapping_mul(193) % SCORING_SHIFTS;
                    director.before_variable_changed(0, shift);
                    let assigned = &mut director.working_solution_mut().shifts[shift].employee;
                    *assigned = match *assigned {
                        None => Some(change % SCORING_EMPLOYEES),
                        Some(employee) if employee + 1 == SCORING_EMPLOYEES => None,
                        Some(employee) => Some(employee + 1),
                    };
                    director.after_variable_changed(0, shift);
                    let score = director.get_score();
                    mix(&mut order, shift);
                    mix_u64(&mut scores, score.hard() as u64);
                    mix_u64(&mut scores, score.soft() as u64);
                    count += 1;
                }
            }
            (count, order, scores)
        });
    }};
}

fn run_scoring(case: &str, entries: Option<&[(&str, u32)]>) {
    let plan = scoring_plan(entries);
    // The weight mode is chosen per solve, exactly as the solver's entrypoint does.
    #[cfg(feature = "constraint_weights")]
    if solverforge_scoring::requires_weighted_mode(&plan) {
        churn_scoring!(
            case,
            ScoreDirector::new_weighted(plan, scoring_constraints())
        );
        return;
    }
    churn_scoring!(case, ScoreDirector::new(plan, scoring_constraints()));
}

fn scoring() {
    run_scoring("scoring", None);
}

fn scoring_weights_one() {
    run_scoring(
        "scoring_weights_one",
        Some(&[
            ("Unassigned", 1),
            ("Over capacity", 1),
            ("Load", 1),
            ("Coverage", 1),
            ("Balance", 1),
        ]),
    );
}

fn scoring_weights_mixed() {
    run_scoring(
        "scoring_weights_mixed",
        Some(&[
            ("Unassigned", 3),
            ("Over capacity", 0),
            ("Load", 3),
            ("Coverage", 0),
            ("Balance", 3),
        ]),
    );
}
