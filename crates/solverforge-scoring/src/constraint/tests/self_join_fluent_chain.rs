// Same-collection self-joins past two sources on the fluent surface (#158).
//
// `for_each(..).join(equal(k))` builds a `BiConstraintStream` whose key is an
// `EntityKeyAdapter`; each further `.join(equal(k))` must accept that key and
// continue to Tri, Quad and Penta. Every arity emits each set of distinct
// entities sharing a key once, so the oracle counts `C(n, r)` per key.

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal;
use crate::stream::ConstraintFactory;
use solverforge_core::score::SoftScore;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct Task {
    team: u32,
}

struct Plan {
    tasks: Vec<Task>,
}

fn tasks(plan: &Plan) -> &[Task] {
    plan.tasks.as_slice()
}

fn team(task: &Task) -> u32 {
    task.team
}

fn factory() -> ConstraintFactory<Plan, SoftScore> {
    ConstraintFactory::new()
}

fn tri() -> impl IncrementalConstraint<Plan, SoftScore> {
    factory()
        .for_each(source(
            tasks as fn(&Plan) -> &[Task],
            ChangeSource::Descriptor(0),
        ))
        .join(equal(team))
        .join(equal(team))
        .penalize(SoftScore::of(1))
        .named("three on a team")
}

fn quad() -> impl IncrementalConstraint<Plan, SoftScore> {
    factory()
        .for_each(source(
            tasks as fn(&Plan) -> &[Task],
            ChangeSource::Descriptor(0),
        ))
        .join(equal(team))
        .join(equal(team))
        .join(equal(team))
        .penalize(SoftScore::of(1))
        .named("four on a team")
}

fn penta() -> impl IncrementalConstraint<Plan, SoftScore> {
    factory()
        .for_each(source(
            tasks as fn(&Plan) -> &[Task],
            ChangeSource::Descriptor(0),
        ))
        .join(equal(team))
        .join(equal(team))
        .join(equal(team))
        .join(equal(team))
        .penalize(SoftScore::of(1))
        .named("five on a team")
}

fn choose(n: i64, r: i64) -> i64 {
    if n < r {
        return 0;
    }
    (0..r).fold(1, |acc, i| acc * (n - i) / (i + 1))
}

// Penalty for every set of `r` distinct tasks sharing a team.
fn oracle(plan: &Plan, r: i64) -> SoftScore {
    let mut teams = std::collections::BTreeMap::<u32, i64>::new();
    for task in &plan.tasks {
        *teams.entry(task.team).or_default() += 1;
    }
    SoftScore::of(-teams.values().map(|&n| choose(n, r)).sum::<i64>())
}

fn check<C: IncrementalConstraint<Plan, SoftScore>>(mut constraint: C, r: i64) {
    let mut plan = Plan {
        tasks: (0..9).map(|i| Task { team: i % 2 }).collect(),
    };
    assert_eq!(constraint.evaluate(&plan), oracle(&plan, r));
    assert_eq!(
        constraint.match_count(&plan) as i64,
        -oracle(&plan, r).score()
    );

    let mut score = constraint.initialize(&plan);
    assert_eq!(score, oracle(&plan, r));
    for step in 0..30usize {
        let entity = (step * 5) % plan.tasks.len();
        score = score + constraint.on_retract(&plan, entity, 0);
        plan.tasks[entity].team = ((step * 7) % 3) as u32;
        score = score + constraint.on_insert(&plan, entity, 0);
        assert_eq!(score, oracle(&plan, r), "arity {r}, step {step}");
        assert_eq!(constraint.evaluate(&plan), score, "arity {r}, step {step}");
    }
}

#[test]
fn fluent_tri_self_join_matches_the_combination_oracle() {
    check(tri(), 3);
}

#[test]
fn fluent_quad_self_join_matches_the_combination_oracle() {
    check(quad(), 4);
}

#[test]
fn fluent_penta_self_join_matches_the_combination_oracle() {
    check(penta(), 5);
}
