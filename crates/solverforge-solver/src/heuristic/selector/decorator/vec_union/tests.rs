use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use super::*;
use crate::heuristic::r#move::{Move, MoveTabuSignature};
use crate::heuristic::selector::move_selector::CandidateStore;

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

#[derive(Debug, PartialEq, Eq)]
struct TestMove(i32);

impl Move<TestSolution> for TestMove {
    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, _score_director: &mut D) {}

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "test"
    }

    fn tabu_signature<D: Director<TestSolution>>(&self, _score_director: &D) -> MoveTabuSignature {
        panic!("vec union tests do not evaluate tabu signatures")
    }
}

struct TestCursor {
    store: CandidateStore<TestSolution, TestMove>,
    next_index: usize,
    count: usize,
}

impl TestCursor {
    fn new(values: impl IntoIterator<Item = i32>) -> Self {
        let mut store = CandidateStore::new();
        let mut count = 0;
        for value in values {
            store.push(TestMove(value));
            count += 1;
        }
        Self {
            store,
            next_index: 0,
            count,
        }
    }
}

impl MoveCursor<TestSolution, TestMove> for TestCursor {
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.next_index >= self.count {
            return None;
        }
        let id = CandidateId::new(self.next_index);
        self.next_index += 1;
        Some(id)
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, TestSolution, TestMove>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> TestMove {
        self.store.take_candidate(id)
    }
}

fn drain_values(mut cursor: VecUnionMoveCursor<TestSolution, TestMove, TestCursor>) -> Vec<i32> {
    let mut values = Vec::new();
    while let Some(id) = cursor.next_candidate() {
        values.push(cursor.take_candidate(id).0);
    }
    values
}

#[test]
fn sequential_drains_each_child_before_next_child() {
    let cursor = VecUnionMoveCursor::new(
        vec![TestCursor::new([1, 2, 3]), TestCursor::new([10, 11])],
        UnionSelectionOrder::Sequential,
    );

    assert_eq!(drain_values(cursor), vec![1, 2, 3, 10, 11]);
}

#[test]
fn round_robin_interleaves_uneven_child_lengths_and_skips_empty_children() {
    let cursor = VecUnionMoveCursor::new(
        vec![
            TestCursor::new([1, 2, 3]),
            TestCursor::new([]),
            TestCursor::new([10]),
            TestCursor::new([20, 21]),
        ],
        UnionSelectionOrder::RoundRobin,
    );

    assert_eq!(drain_values(cursor), vec![1, 10, 20, 2, 21, 3]);
}

#[test]
fn round_robin_candidates_remain_borrowable_and_takeable_after_interleaving() {
    let mut cursor = VecUnionMoveCursor::new(
        vec![TestCursor::new([1, 2]), TestCursor::new([10, 11])],
        UnionSelectionOrder::RoundRobin,
    );

    let first = cursor.next_candidate().unwrap();
    let second = cursor.next_candidate().unwrap();
    let third = cursor.next_candidate().unwrap();

    assert_eq!(cursor.selector_index(first), Some(0));
    assert_eq!(cursor.selector_index(second), Some(1));
    assert_eq!(cursor.selector_index(third), Some(0));
    assert!(cursor.candidate(second).is_some());
    assert_eq!(cursor.take_candidate(second), TestMove(10));
    assert_eq!(cursor.take_candidate(first), TestMove(1));
    assert_eq!(cursor.take_candidate(third), TestMove(2));
}
