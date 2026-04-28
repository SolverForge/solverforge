use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, MoveCandidateRef, MoveCursor};

#[derive(Debug)]
pub struct LimitedMoveCursor<C> {
    inner: C,
    limit: usize,
    yielded: usize,
}

impl<C> LimitedMoveCursor<C> {
    pub(crate) fn new(inner: C, limit: usize) -> Self {
        Self {
            inner,
            limit,
            yielded: 0,
        }
    }
}

impl<S, M, C> MoveCursor<S, M> for LimitedMoveCursor<C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.yielded >= self.limit {
            return None;
        }
        let id = self.inner.next_candidate()?;
        self.yielded += 1;
        Some(id)
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.inner.take_candidate(id)
    }

    fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.inner.selector_index(id)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

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

    #[derive(Debug)]
    struct TestMove;

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

        fn tabu_signature<D: Director<TestSolution>>(
            &self,
            _score_director: &D,
        ) -> MoveTabuSignature {
            panic!("limited cursor test does not evaluate tabu signatures")
        }
    }

    struct CountingCursor {
        generated: Arc<AtomicUsize>,
        store: CandidateStore<TestSolution, TestMove>,
        total: usize,
    }

    impl MoveCursor<TestSolution, TestMove> for CountingCursor {
        fn next_candidate(&mut self) -> Option<CandidateId> {
            if self.generated.load(Ordering::Relaxed) >= self.total {
                return None;
            }
            self.generated.fetch_add(1, Ordering::Relaxed);
            Some(self.store.push(TestMove))
        }

        fn candidate(
            &self,
            id: CandidateId,
        ) -> Option<MoveCandidateRef<'_, TestSolution, TestMove>> {
            self.store.candidate(id)
        }

        fn take_candidate(&mut self, id: CandidateId) -> TestMove {
            self.store.take_candidate(id)
        }

        fn selector_index(&self, _id: CandidateId) -> Option<usize> {
            Some(7)
        }
    }

    impl Debug for CountingCursor {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("CountingCursor")
                .field("generated", &self.generated.load(Ordering::Relaxed))
                .field("total", &self.total)
                .finish()
        }
    }

    #[test]
    fn limited_cursor_stops_inner_generation_at_limit() {
        let generated = Arc::new(AtomicUsize::new(0));
        let inner = CountingCursor {
            generated: generated.clone(),
            store: CandidateStore::new(),
            total: 100,
        };
        let mut cursor = LimitedMoveCursor::new(inner, 3);

        for _ in 0..3 {
            let id = cursor.next_candidate().expect("candidate should exist");
            assert!(cursor.candidate(id).is_some());
            assert_eq!(cursor.selector_index(id), Some(7));
        }

        assert!(cursor.next_candidate().is_none());
        assert_eq!(generated.load(Ordering::Relaxed), 3);
    }
}
