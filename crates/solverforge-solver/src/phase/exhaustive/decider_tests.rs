use super::*;
use solverforge_core::domain::{EntityCollectionExtractor, EntityDescriptor, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct TestSolution {
    values: Vec<Option<i32>>,
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn set_row(s: &mut TestSolution, idx: usize, v: Option<i32>) {
    if let Some(slot) = s.values.get_mut(idx) {
        *slot = v;
    }
}

#[test]
fn test_simple_decider_creation() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![0, 1, 2, 3], set_row);

    let debug = format!("{:?}", decider);
    assert!(debug.contains("SimpleDecider"));
    assert!(debug.contains("value_count: 4"));
}

#[test]
fn simple_decider_replays_and_resets_assignment_nodes() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![10, 20, 30], set_row).with_variable_index(1);
    let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>());
    let mut director = ScoreDirector::simple(
        TestSolution {
            values: vec![Some(7), Some(8)],
            score: None,
        },
        descriptor,
        |solution, _| solution.values.len(),
    );
    let node = ExhaustiveSearchNode::child(0, 1, SoftScore::of(0), 0, 1, 1, 2);

    decider.reset_assignments(&mut director);
    assert_eq!(director.working_solution().values, vec![None, None]);

    decider.apply_assignment(&node, &mut director);
    assert_eq!(director.working_solution().values, vec![None, Some(30)]);
}

#[test]
fn simple_decider_keeps_pinned_input_values_through_reset_and_replay() {
    let decider: SimpleDecider<TestSolution, i32> =
        SimpleDecider::new(0, "row", vec![10, 20], set_row);
    let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(
            EntityDescriptor::new("Row", TypeId::of::<Option<i32>>(), "values")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Row",
                    "values",
                    |solution: &TestSolution| &solution.values,
                    |solution: &mut TestSolution| &mut solution.values,
                )))
                .with_pin_predicate(|entity| {
                    *entity.downcast_ref::<Option<i32>>().unwrap() == Some(7)
                }),
        );
    let mut director = ScoreDirector::simple(
        TestSolution {
            values: vec![Some(7), Some(8)],
            score: None,
        },
        descriptor,
        |solution, _| solution.values.len(),
    );

    decider.reset_assignments(&mut director);
    assert_eq!(director.working_solution().values, vec![Some(7), None]);
    let root = ExhaustiveSearchNode::root(SoftScore::of(0));
    let pinned = decider.expand(0, &root, &mut director);
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].depth(), 1);
    assert!(pinned[0].candidate_value_index().is_none());
    let free = decider.expand(1, &pinned[0], &mut director);
    assert_eq!(free.len(), 2);

    decider.reset_assignments(&mut director);
    decider.apply_assignment(&pinned[0], &mut director);
    decider.apply_assignment(&free[1], &mut director);
    assert_eq!(director.working_solution().values, vec![Some(7), Some(20)]);
}
