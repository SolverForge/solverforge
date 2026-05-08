use crate::builder::context::CoverageGroupBinding;
use crate::phase::construction::coverage::{
    capacity_conflict_moves, required_coverage_moves, CoverageMoveOptions,
};

const DEFAULT_MAX_MOVES_PER_STEP: usize = 256;

pub struct CoverageRepairSelector<S> {
    group: CoverageGroupBinding<S>,
    value_candidate_limit: Option<usize>,
    max_moves_per_step: Option<usize>,
    require_hard_improvement: bool,
}

impl<S> CoverageRepairSelector<S> {
    pub fn new(
        group: CoverageGroupBinding<S>,
        value_candidate_limit: Option<usize>,
        max_moves_per_step: Option<usize>,
        require_hard_improvement: bool,
    ) -> Self {
        Self {
            group,
            value_candidate_limit,
            max_moves_per_step,
            require_hard_improvement,
        }
    }

    fn max_moves_per_step(&self) -> usize {
        self.max_moves_per_step
            .or(self.group.limits.max_moves_per_step)
            .unwrap_or(DEFAULT_MAX_MOVES_PER_STEP)
    }
}

impl<S> std::fmt::Debug for CoverageRepairSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoverageRepairSelector")
            .field("group_name", &self.group.group_name)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .field("max_moves_per_step", &self.max_moves_per_step())
            .field("require_hard_improvement", &self.require_hard_improvement)
            .finish()
    }
}

pub struct CoverageRepairCursor<S>
where
    S: PlanningSolution + 'static,
{
    store: CandidateStore<S, ScalarMoveUnion<S, usize>>,
    next_index: usize,
}

impl<S> CoverageRepairCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn new(store: CandidateStore<S, ScalarMoveUnion<S, usize>>) -> Self {
        Self {
            store,
            next_index: 0,
        }
    }
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for CoverageRepairCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.next_index >= self.store.len() {
            return None;
        }
        let id = CandidateId::new(self.next_index);
        self.next_index += 1;
        Some(id)
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ScalarMoveUnion<S, usize>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ScalarMoveUnion<S, usize> {
        self.store.take_candidate(id)
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for CoverageRepairSelector<S>
where
    S: PlanningSolution + 'static,
{
    type Cursor<'a>
        = CoverageRepairCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let max_moves_per_step = self.max_moves_per_step();
        let options = CoverageMoveOptions::for_repair(
            &self.group,
            self.value_candidate_limit,
            max_moves_per_step,
        );
        let mut store = CandidateStore::with_capacity(max_moves_per_step);
        for mov in required_coverage_moves(&self.group, solution, options)
            .into_iter()
            .chain(capacity_conflict_moves(&self.group, solution, options))
        {
            if store.len() >= max_moves_per_step {
                break;
            }
            let mov = mov.with_require_hard_improvement(self.require_hard_improvement);
            if mov.is_doable(score_director) {
                store.push(ScalarMoveUnion::CompoundScalar(mov));
            }
        }
        CoverageRepairCursor::new(store)
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, _score_director: &D) -> usize {
        self.max_moves_per_step()
    }
}
