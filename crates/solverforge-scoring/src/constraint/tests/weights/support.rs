// Shared fixture and oracle for constraint weight multiplier tests.

pub(super) use crate::api::constraint_set::ConstraintSet;
pub(super) use crate::director::score_director::ScoreDirector;
pub(super) use crate::director::Director;
pub(super) use crate::stream::collection_extract::{source, ChangeSource};
pub(super) use crate::stream::ConstraintFactory;
use solverforge_core::domain::PlanningSolution;
pub(super) use solverforge_core::score::{HardSoftScore, Score};
pub(super) use solverforge_core::ConstraintWeights;

pub(super) const MULTIPLIERS: [u32; 3] = [0, 1, 3];

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Shift {
    pub(super) employee: Option<usize>,
    pub(super) hours: i64,
    pub(super) mentor: Option<usize>,
}

#[derive(Clone, Debug)]
pub(super) struct Employee {
    pub(super) id: usize,
    pub(super) capacity: i64,
}

#[derive(Clone, Debug)]
pub(super) struct Task {
    pub(super) duration: usize,
    pub(super) next: Option<usize>,
}

#[derive(Clone, Debug)]
pub(super) struct Route {
    pub(super) tasks: Vec<usize>,
}

#[derive(Clone, Debug)]
pub(super) struct Plan {
    pub(super) shifts: Vec<Shift>,
    pub(super) employees: Vec<Employee>,
    pub(super) tasks: Vec<Task>,
    pub(super) routes: Vec<Route>,
    pub(super) weights: ConstraintWeights,
    pub(super) score: Option<HardSoftScore>,
}

impl PlanningSolution for Plan {
    type Score = HardSoftScore;

    const HAS_CONSTRAINT_WEIGHTS: bool = true;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    fn constraint_weights(&self) -> Option<&ConstraintWeights> {
        Some(&self.weights)
    }
}

pub(super) fn shifts(plan: &Plan) -> &[Shift] {
    plan.shifts.as_slice()
}

pub(super) fn employees(plan: &Plan) -> &[Employee] {
    plan.employees.as_slice()
}

pub(super) const SHIFTS: ChangeSource = ChangeSource::Descriptor(0);
pub(super) const EMPLOYEES: ChangeSource = ChangeSource::Descriptor(1);
pub(super) const ROUTES: usize = 2;

pub(super) fn shift(employee: Option<usize>, hours: i64, mentor: Option<usize>) -> Shift {
    Shift {
        employee,
        hours,
        mentor,
    }
}

pub(super) fn plan(weights: ConstraintWeights) -> Plan {
    Plan {
        shifts: vec![
            shift(Some(0), 4, None),
            shift(Some(0), 5, Some(1)),
            shift(Some(1), 3, Some(0)),
            shift(None, 2, None),
            shift(Some(2), 6, Some(1)),
            shift(Some(1), 1, None),
        ],
        employees: vec![
            Employee { id: 0, capacity: 8 },
            Employee { id: 1, capacity: 4 },
            Employee { id: 2, capacity: 6 },
        ],
        tasks: vec![
            Task {
                duration: 2,
                next: Some(1),
            },
            Task {
                duration: 3,
                next: None,
            },
            Task {
                duration: 1,
                next: None,
            },
        ],
        routes: vec![Route { tasks: vec![0, 2] }, Route { tasks: vec![1] }],
        weights,
        score: None,
    }
}

pub(super) type Mutation = (usize, usize, fn(&mut Plan));

// A churn script that touches every descriptor, including undo moves.
pub(super) fn mutations() -> Vec<Mutation> {
    vec![
        (0, 1, |p| p.shifts[1].employee = Some(2)),
        (0, 3, |p| p.shifts[3].employee = Some(1)),
        (0, 0, |p| p.shifts[0].hours = 9),
        (1, 2, |p| p.employees[2].capacity = 1),
        (0, 4, |p| p.shifts[4].mentor = Some(2)),
        (0, 3, |p| p.shifts[3].employee = None),
        (ROUTES, 0, |p| p.routes[0].tasks = vec![2, 0]),
        (ROUTES, 1, |p| p.routes[1].tasks = vec![1, 2]),
        (ROUTES, 0, |p| p.routes[0].tasks = vec![0]),
        (0, 5, |p| p.shifts[5].employee = Some(0)),
        (1, 0, |p| p.employees[0].capacity = 30),
        (0, 1, |p| p.shifts[1].employee = Some(0)),
        (0, 4, |p| p.shifts[4].employee = Some(0)),
    ]
}

fn weights_of(entries: &[(&str, u32)]) -> ConstraintWeights {
    let mut weights = ConstraintWeights::new();
    for (name, multiplier) in entries {
        weights.set(*name, *multiplier);
    }
    weights
}

fn multiplier(entries: &[(&str, u32)], name: &str) -> u32 {
    entries
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map_or(1, |(_, multiplier)| *multiplier)
}

fn scaled(score: HardSoftScore, multiplier: u32) -> HardSoftScore {
    score.scale(i64::from(multiplier))
}

/* Drives an unweighted and a weighted director through the same churn and
checks, at every step, that the weighted director's incremental score, full
recalculation, per-constraint results, and per-match explanations all equal
the unweighted values scaled by each constraint's multiplier.
*/
pub(super) fn assert_weighted<C>(build: impl Fn() -> C, entries: &[(&str, u32)])
where
    C: ConstraintSet<Plan, HardSoftScore>,
{
    let mut base = ScoreDirector::new(plan(ConstraintWeights::new()), build());
    let mut weighted = ScoreDirector::new(plan(weights_of(entries)), build());
    base.calculate_score();
    weighted.calculate_score();
    let mut scored = assert_step(&base, &weighted, entries, "initialize");

    for (step, (descriptor, entity, mutate)) in mutations().into_iter().enumerate() {
        base.do_change(descriptor, entity, mutate);
        weighted.do_change(descriptor, entity, mutate);
        scored |= assert_step(&base, &weighted, entries, &format!("step {step}"));
    }
    assert!(scored, "fixture never produced a non-zero baseline score");
}

fn assert_step<C>(
    base: &ScoreDirector<Plan, C>,
    weighted: &ScoreDirector<Plan, C>,
    entries: &[(&str, u32)],
    context: &str,
) -> bool
where
    C: ConstraintSet<Plan, HardSoftScore>,
{
    let base_each = base.constraint_results();
    let weighted_each = weighted.constraint_results();
    assert_eq!(base_each.len(), weighted_each.len(), "{context}");

    let mut expected = HardSoftScore::ZERO;
    for (b, w) in base_each.iter().zip(&weighted_each) {
        let m = multiplier(entries, b.name);
        assert_eq!(b.name, w.name, "{context}");
        assert_eq!(b.weight_multiplier, 1, "{context}: {}", b.name);
        assert_eq!(w.weight_multiplier, m, "{context}: {}", w.name);
        assert_eq!(w.score, scaled(b.score, m), "{context}: {}", w.name);
        assert_eq!(w.match_count, b.match_count, "{context}: {}", w.name);
        assert_eq!(w.is_hard, b.is_hard, "{context}: {}", w.name);
        expected = expected + scaled(b.score, m);
    }

    assert_eq!(Some(base.get_score()), base.fresh_score());
    assert_eq!(weighted.get_score(), expected, "{context}: incremental");
    assert_eq!(
        weighted.fresh_score(),
        Some(expected),
        "{context}: full recalculation"
    );

    let base_detail = base.constraint_analyses();
    let weighted_detail = weighted.constraint_analyses();
    for (b, w) in base_detail.iter().zip(&weighted_detail) {
        let m = multiplier(entries, b.name());
        assert_eq!(w.weight_multiplier, m, "{context}: {}", w.name());
        assert_eq!(w.score, scaled(b.score, m), "{context}: {}", w.name());
        let mut base_matches: Vec<_> = b.matches.iter().map(|x| scaled(x.score, m)).collect();
        let mut weighted_matches: Vec<_> = w.matches.iter().map(|x| x.score).collect();
        base_matches.sort();
        weighted_matches.sort();
        assert_eq!(weighted_matches, base_matches, "{context}: {}", w.name());
        if !w.matches.is_empty() {
            let total = weighted_matches
                .iter()
                .fold(HardSoftScore::ZERO, |sum, score| sum + *score);
            assert_eq!(total, w.score, "{context}: {} match total", w.name());
        }
    }
    base_each
        .iter()
        .all(|result| result.score != HardSoftScore::ZERO)
}

// Runs `assert_weighted` for one named constraint at every test multiplier.
pub(super) fn assert_family<C>(name: &str, build: impl Fn() -> C)
where
    C: ConstraintSet<Plan, HardSoftScore>,
{
    for multiplier in MULTIPLIERS {
        assert_weighted(&build, &[(name, multiplier)]);
    }
}
