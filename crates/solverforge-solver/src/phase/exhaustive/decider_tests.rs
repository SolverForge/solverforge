use super::*;
use solverforge_core::domain::SolutionDescriptor;
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
