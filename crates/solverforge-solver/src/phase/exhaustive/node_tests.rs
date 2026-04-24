use super::*;
use crate::heuristic::r#move::ChangeMove;
use solverforge_core::score::SoftScore;

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

fn get_value(s: &TestSolution, idx: usize, _variable_index: usize) -> Option<i32> {
    s.values.get(idx).copied().flatten()
}

fn set_value(s: &mut TestSolution, idx: usize, _variable_index: usize, v: Option<i32>) {
    if let Some(val) = s.values.get_mut(idx) {
        *val = v;
    }
}

#[test]
fn test_root_node() {
    let node: ExhaustiveSearchNode<TestSolution> = ExhaustiveSearchNode::root(SoftScore::of(0));

    assert_eq!(node.depth(), 0);
    assert_eq!(node.score(), &SoftScore::of(0));
    assert!(node.parent_index().is_none());
    assert!(node.entity_index().is_none());
    assert!(!node.is_expanded());
}

#[test]
fn test_child_node() {
    let node: ExhaustiveSearchNode<TestSolution> =
        ExhaustiveSearchNode::child(0, 1, SoftScore::of(-1), 0, 2);

    assert_eq!(node.depth(), 1);
    assert_eq!(node.score(), &SoftScore::of(-1));
    assert_eq!(node.parent_index(), Some(0));
    assert_eq!(node.entity_index(), Some(0));
    assert_eq!(node.value_index(), Some(2));
}

#[test]
fn test_is_leaf() {
    let node: ExhaustiveSearchNode<TestSolution> =
        ExhaustiveSearchNode::child(0, 4, SoftScore::of(0), 3, 1);

    assert!(node.is_leaf(4));
    assert!(!node.is_leaf(5));
}

#[test]
fn test_optimistic_bound_pruning() {
    let mut node: ExhaustiveSearchNode<TestSolution> =
        ExhaustiveSearchNode::root(SoftScore::of(-5));

    assert!(!node.can_prune(&SoftScore::of(0)));

    node.set_optimistic_bound(SoftScore::of(-2));
    assert!(node.can_prune(&SoftScore::of(0)));

    node.set_optimistic_bound(SoftScore::of(5));
    assert!(!node.can_prune(&SoftScore::of(0)));
}

type TestMove = ChangeMove<TestSolution, i32>;

#[test]
fn test_move_sequence() {
    let mut seq: MoveSequence<TestSolution, TestMove> = MoveSequence::new();

    assert!(seq.is_empty());
    assert_eq!(seq.len(), 0);

    seq.push(ChangeMove::new(
        0,
        Some(42),
        get_value,
        set_value,
        0,
        "test",
        0,
    ));
    assert_eq!(seq.len(), 1);

    let m = seq.pop();
    assert!(m.is_some());
    assert!(seq.is_empty());
}
