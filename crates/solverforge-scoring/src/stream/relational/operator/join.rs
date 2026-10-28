use super::super::{DenseRowStore, HandleMap, JoinedIdentity, RowHandle};
use super::{links::remove_link, Operator, Pair, PairDelta};
use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use crate::stream::relational::hash::FastMap;
use std::marker::PhantomData;

struct JoinedRow {
    identity: JoinedIdentity,
    left_position: usize,
    right_position: usize,
}

pub struct JoinNode<S, L, R, P: IndexedPlan> {
    left: L,
    right: R,
    plan: P,
    indexes: P::Indexes,
    rows: DenseRowStore<JoinedRow>,
    outputs: FastMap<JoinedIdentity, RowHandle>,
    left_outputs: HandleMap<Vec<RowHandle>>,
    right_outputs: HandleMap<Vec<RowHandle>>,
    /// Child notification scratch, reused across events (L/R, removed/inserted).
    child_l_removed: Vec<RowHandle>,
    child_l_inserted: Vec<RowHandle>,
    child_r_removed: Vec<RowHandle>,
    child_r_inserted: Vec<RowHandle>,
    marker: PhantomData<fn() -> S>,
}

impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static> JoinNode<S, L, R, P>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
{
    pub fn new<C: CompileCondition<Plan = P>>(left: L, right: R, condition: C) -> Self {
        let plan = condition.compile();
        let indexes = plan.new_indexes();
        Self {
            left,
            right,
            plan,
            indexes,
            rows: DenseRowStore::new(),
            outputs: FastMap::default(),
            left_outputs: HandleMap::new(),
            right_outputs: HandleMap::new(),
            child_l_removed: Vec::new(),
            child_l_inserted: Vec::new(),
            child_r_removed: Vec::new(),
            child_r_inserted: Vec::new(),
            marker: PhantomData,
        }
    }
    fn apply_changes(
        &mut self,
        solution: &S,
        delta: PairDelta<'_>,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        let PairDelta {
            left_removed,
            left_inserted,
            right_removed,
            right_inserted,
        } = delta;
        for handle in left_removed.iter().copied() {
            self.plan.remove_left(&mut self.indexes, handle);
            for output in self.left_outputs.remove(handle).unwrap_or_default() {
                if self.rows.get(output).is_some() {
                    self.remove_output(output);
                    removed.push(output);
                }
            }
        }
        for handle in right_removed.iter().copied() {
            self.plan.remove_right(&mut self.indexes, handle);
            for output in self.right_outputs.remove(handle).unwrap_or_default() {
                if self.rows.get(output).is_some() {
                    self.remove_output(output);
                    removed.push(output);
                }
            }
        }
        for handle in left_inserted.iter().copied() {
            let row = self
                .left
                .resolve(solution, handle)
                .expect("inserted left row");
            self.plan.insert_left(&mut self.indexes, handle, &row);
        }
        for handle in right_inserted.iter().copied() {
            let row = self
                .right
                .resolve(solution, handle)
                .expect("inserted right row");
            self.plan.insert_right(&mut self.indexes, handle, &row);
        }
        for handle in left_inserted.iter().copied() {
            self.probe_left(solution, handle, &mut |h| inserted.push(h));
        }
        for handle in right_inserted.iter().copied() {
            self.probe_right(solution, handle, &mut |h| inserted.push(h));
        }
    }
    fn remove_output(&mut self, handle: RowHandle) {
        if let Some(row) = self.rows.retract(handle) {
            self.outputs.remove(&row.identity);
            if let Some(moved) = remove_link(
                &mut self.left_outputs,
                row.identity.left(),
                handle,
                row.left_position,
            ) {
                self.rows
                    .get_mut(moved)
                    .expect("moved left output")
                    .left_position = row.left_position;
            }
            if let Some(moved) = remove_link(
                &mut self.right_outputs,
                row.identity.right(),
                handle,
                row.right_position,
            ) {
                self.rows
                    .get_mut(moved)
                    .expect("moved right output")
                    .right_position = row.right_position;
            }
        }
    }
}

impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static> JoinNode<S, L, R, P>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
{
    fn emit_output(
        rows: &mut DenseRowStore<JoinedRow>,
        outputs: &mut FastMap<JoinedIdentity, RowHandle>,
        left_outputs: &mut HandleMap<Vec<RowHandle>>,
        right_outputs: &mut HandleMap<Vec<RowHandle>>,
        left: RowHandle,
        right: RowHandle,
    ) -> Option<RowHandle> {
        let identity = JoinedIdentity::new(left, right, 0);
        if outputs.contains_key(&identity) {
            return None;
        }
        let left_bucket = left_outputs.get_or_insert_with(left, Vec::new);
        let right_bucket = right_outputs.get_or_insert_with(right, Vec::new);
        let handle = rows.insert(JoinedRow {
            identity,
            left_position: left_bucket.len(),
            right_position: right_bucket.len(),
        });
        outputs.insert(identity, handle);
        left_bucket.push(handle);
        right_bucket.push(handle);
        Some(handle)
    }
    #[inline(always)]
    fn probe_left(&mut self, solution: &S, left: RowHandle, deltas: &mut impl FnMut(RowHandle)) {
        let candidates = {
            let Some(row) = self.left.resolve(solution, left) else {
                return;
            };
            self.plan.right_candidates(&self.indexes, &row)
        };
        for &right in candidates.iter() {
            let lv = self.left.resolve(solution, left).expect("live probe row");
            let rv = self
                .right
                .resolve(solution, right)
                .expect("live candidate row");
            if !self.plan.candidate_matches(&lv, &rv) {
                continue;
            }
            if let Some(h) = Self::emit_output(
                &mut self.rows,
                &mut self.outputs,
                &mut self.left_outputs,
                &mut self.right_outputs,
                left,
                right,
            ) {
                deltas(h);
            }
        }
    }
    #[inline(always)]
    fn probe_right(&mut self, solution: &S, right: RowHandle, deltas: &mut impl FnMut(RowHandle)) {
        let candidates = {
            let Some(row) = self.right.resolve(solution, right) else {
                return;
            };
            self.plan.left_candidates(&self.indexes, &row)
        };
        for &left in candidates.iter() {
            let lv = self
                .left
                .resolve(solution, left)
                .expect("live candidate row");
            let rv = self.right.resolve(solution, right).expect("live probe row");
            if !self.plan.candidate_matches(&lv, &rv) {
                continue;
            }
            if let Some(h) = Self::emit_output(
                &mut self.rows,
                &mut self.outputs,
                &mut self.left_outputs,
                &mut self.right_outputs,
                left,
                right,
            ) {
                deltas(h);
            }
        }
    }
}

impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static> Operator<S>
    for JoinNode<S, L, R, P>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
{
    type View<'a> = Pair<L::View<'a>, R::View<'a>>;
    type Evaluation = (L::Evaluation, R::Evaluation);
    #[inline]
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        (
            self.left.prepare_evaluation(solution),
            self.right.prepare_evaluation(solution),
        )
    }
    #[inline]
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        let mut indexes = self.plan.new_indexes();
        // Evaluation has no retractions or slot reuse: ordinals are sufficient.
        // Keep generational storage only for retained rows across notifications.
        let mut right_rows = Vec::new();
        self.right
            .visit_evaluation(solution, &evaluation.1, &mut |right| {
                let handle = RowHandle::new(right_rows.len() as u32, 0);
                right_rows.push(right);
                self.plan
                    .insert_right_transient(&mut indexes, handle, &right);
            });
        self.left
            .visit_evaluation(solution, &evaluation.0, &mut |left| {
                for &handle in self.plan.right_candidates(&indexes, &left).iter() {
                    let right = right_rows[handle.slot() as usize];
                    if self.plan.candidate_matches(&left, &right) {
                        visitor(Pair { left, right });
                    }
                }
            });
    }
    fn clear(&mut self) {
        self.left.clear();
        self.right.clear();
        self.indexes = self.plan.new_indexes();
        self.rows.clear();
        self.outputs.clear();
        self.left_outputs.clear();
        self.right_outputs.clear();
        self.child_l_removed.clear();
        self.child_l_inserted.clear();
        self.child_r_removed.clear();
        self.child_r_inserted.clear();
    }
    #[inline(always)]
    fn initialize(&mut self, solution: &S) {
        self.left.initialize(solution);
        self.right.initialize(solution);
        self.indexes = self.plan.new_indexes();
        self.rows.clear();
        self.outputs.clear();
        self.left_outputs.clear();
        self.right_outputs.clear();
        for handle in self.left.handles() {
            let row = self.left.resolve(solution, handle).expect("live left row");
            self.plan.insert_left(&mut self.indexes, handle, &row);
        }
        for handle in self.right.handles() {
            let row = self
                .right
                .resolve(solution, handle)
                .expect("live right row");
            self.plan.insert_right(&mut self.indexes, handle, &row);
        }
        for handle in self.left.handles() {
            self.probe_left(solution, handle, &mut |_| {});
        }
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        let identity = self.rows.get(handle)?.identity;
        Some(Pair {
            left: self.left.resolve(solution, identity.left())?,
            right: self.right.resolve(solution, identity.right())?,
        })
    }
    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if let Some(row) = self.rows.get(handle) {
            let identity = row.identity;
            self.left.visit_provenance(identity.left(), visitor);
            self.right.visit_provenance(identity.right(), visitor);
        }
    }
    #[inline(always)]
    fn retract(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        inserted.clear();
        let mut lm = std::mem::take(&mut self.child_l_removed);
        let mut li = std::mem::take(&mut self.child_l_inserted);
        lm.clear();
        li.clear();
        self.left
            .retract(solution, descriptor, index, &mut lm, &mut li);
        let mut rm = std::mem::take(&mut self.child_r_removed);
        let mut ri = std::mem::take(&mut self.child_r_inserted);
        rm.clear();
        ri.clear();
        self.right
            .retract(solution, descriptor, index, &mut rm, &mut ri);
        self.apply_changes(
            solution,
            PairDelta {
                left_removed: &lm,
                left_inserted: &li,
                right_removed: &rm,
                right_inserted: &ri,
            },
            removed,
            inserted,
        );
        self.child_l_removed = lm;
        self.child_l_inserted = li;
        self.child_r_removed = rm;
        self.child_r_inserted = ri;
    }
    #[inline(always)]
    fn insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        removed.clear();
        inserted.clear();
        let mut lm = std::mem::take(&mut self.child_l_removed);
        let mut li = std::mem::take(&mut self.child_l_inserted);
        lm.clear();
        li.clear();
        self.left
            .insert(solution, descriptor, index, &mut lm, &mut li);
        let mut rm = std::mem::take(&mut self.child_r_removed);
        let mut ri = std::mem::take(&mut self.child_r_inserted);
        rm.clear();
        ri.clear();
        self.right
            .insert(solution, descriptor, index, &mut rm, &mut ri);
        self.apply_changes(
            solution,
            PairDelta {
                left_removed: &lm,
                left_inserted: &li,
                right_removed: &rm,
                right_inserted: &ri,
            },
            removed,
            inserted,
        );
        self.child_l_removed = lm;
        self.child_l_inserted = li;
        self.child_r_removed = rm;
        self.child_r_inserted = ri;
    }
}
