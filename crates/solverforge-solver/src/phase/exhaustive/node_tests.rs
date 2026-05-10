use super::*;
use solverforge_core::score::SoftScore;

#[derive(Clone, Debug)]
struct TestSolution {
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
        ExhaustiveSearchNode::child(0, 1, SoftScore::of(-1), 3, 4, 0, 2);

    assert_eq!(node.depth(), 1);
    assert_eq!(node.score(), &SoftScore::of(-1));
    assert_eq!(node.parent_index(), Some(0));
    assert_eq!(node.descriptor_index(), Some(3));
    assert_eq!(node.variable_index(), Some(4));
    assert_eq!(node.entity_index(), Some(0));
    assert_eq!(node.candidate_value_index(), Some(2));
}

#[test]
fn test_is_leaf() {
    let node: ExhaustiveSearchNode<TestSolution> =
        ExhaustiveSearchNode::child(0, 4, SoftScore::of(0), 0, 0, 3, 1);

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

#[test]
fn assignment_path_returns_root_to_leaf_assignments() {
    let root: ExhaustiveSearchNode<TestSolution> = ExhaustiveSearchNode::root(SoftScore::of(0));
    let first = ExhaustiveSearchNode::child(0, 1, SoftScore::of(-1), 0, 0, 0, 2);
    let second = ExhaustiveSearchNode::child(1, 2, SoftScore::of(0), 0, 0, 1, 3);
    let all_nodes = vec![root, first];

    let path = second.assignment_path(&all_nodes);

    assert_eq!(path.len(), 2);
    assert_eq!(path[0].entity_index(), Some(0));
    assert_eq!(path[0].candidate_value_index(), Some(2));
    assert_eq!(path[1].entity_index(), Some(1));
    assert_eq!(path[1].candidate_value_index(), Some(3));
}
