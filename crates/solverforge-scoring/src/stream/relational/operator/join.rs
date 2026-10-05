use super::super::{DenseRowStore, HandleMap, JoinedIdentity, RowHandle};
use super::{Operator, Pair};
use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use std::collections::HashMap;
use std::marker::PhantomData;

pub struct JoinNode<S, L, R, P: IndexedPlan> {
    left: L,
    right: R,
    plan: P,
    indexes: P::Indexes,
    rows: DenseRowStore<JoinedIdentity>,
    outputs: HashMap<JoinedIdentity, RowHandle>,
    left_outputs: HandleMap<Vec<RowHandle>>,
    right_outputs: HandleMap<Vec<RowHandle>>,
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
            outputs: HashMap::new(),
            left_outputs: HandleMap::new(),
            right_outputs: HandleMap::new(),
            marker: PhantomData,
        }
    }
    fn remove_output(&mut self, handle: RowHandle) {
        if let Some(identity) = self.rows.retract(handle) {
            self.outputs.remove(&identity);
            unlink(&mut self.left_outputs, identity.left(), handle);
            unlink(&mut self.right_outputs, identity.right(), handle);
        }
    }
}

impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static> JoinNode<S, L, R, P>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
{
    fn emit_output(
        rows: &mut DenseRowStore<JoinedIdentity>,
        outputs: &mut HashMap<JoinedIdentity, RowHandle>,
        left_outputs: &mut HandleMap<Vec<RowHandle>>,
        right_outputs: &mut HandleMap<Vec<RowHandle>>,
        left: RowHandle,
        right: RowHandle,
    ) -> Option<RowHandle> {
        let identity = JoinedIdentity::new(left, right, 0);
        if outputs.contains_key(&identity) {
            return None;
        }
        let handle = rows.insert(identity);
        outputs.insert(identity, handle);
        left_outputs.get_or_insert_with(left, Vec::new).push(handle);
        right_outputs
            .get_or_insert_with(right, Vec::new)
            .push(handle);
        Some(handle)
    }
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
    #[inline]
    fn visit_all<'a>(&'a self, solution: &'a S, visitor: &mut impl FnMut(Self::View<'a>)) {
        let mut indexes = self.plan.new_indexes();
        // Evaluation has no retractions or slot reuse: ordinals are sufficient.
        // Keep generational storage only for retained rows across notifications.
        let mut right_rows = Vec::new();
        self.right.visit_all(solution, &mut |right| {
            let handle = RowHandle::new(right_rows.len() as u32, 0);
            right_rows.push(right);
            self.plan
                .insert_right_transient(&mut indexes, handle, &right);
        });
        self.left.visit_all(solution, &mut |left| {
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
    }
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
        let identity = self.rows.get(handle)?;
        Some(Pair {
            left: self.left.resolve(solution, identity.left())?,
            right: self.right.resolve(solution, identity.right())?,
        })
    }
    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if let Some(identity) = self.rows.get(handle) {
            self.left.visit_provenance(identity.left(), visitor);
            self.right.visit_provenance(identity.right(), visitor);
        }
    }
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        let left = self.left.retract(solution, descriptor, index);
        let right = self.right.retract(solution, descriptor, index);
        let mut removed = Vec::new();
        for handle in left {
            self.plan.remove_left(&mut self.indexes, handle);
            for output in self.left_outputs.remove(handle).unwrap_or_default() {
                if self.rows.get(output).is_some() {
                    self.remove_output(output);
                    removed.push(output);
                }
            }
        }
        for handle in right {
            self.plan.remove_right(&mut self.indexes, handle);
            for output in self.right_outputs.remove(handle).unwrap_or_default() {
                if self.rows.get(output).is_some() {
                    self.remove_output(output);
                    removed.push(output);
                }
            }
        }
        removed
    }
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> Vec<RowHandle> {
        let left = self.left.insert(solution, descriptor, index);
        let right = self.right.insert(solution, descriptor, index);
        for &handle in &left {
            let row = self
                .left
                .resolve(solution, handle)
                .expect("inserted left row");
            self.plan.insert_left(&mut self.indexes, handle, &row);
        }
        for &handle in &right {
            let row = self
                .right
                .resolve(solution, handle)
                .expect("inserted right row");
            self.plan.insert_right(&mut self.indexes, handle, &row);
        }
        let mut inserted = Vec::new();
        for handle in left {
            self.probe_left(solution, handle, &mut |h| inserted.push(h));
        }
        for handle in right {
            self.probe_right(solution, handle, &mut |h| inserted.push(h));
        }
        inserted
    }
}

fn unlink(links: &mut HandleMap<Vec<RowHandle>>, input: RowHandle, output: RowHandle) {
    if let Some(bucket) = links.get_mut(input) {
        bucket.retain(|h| *h != output);
        if bucket.is_empty() {
            links.remove(input);
        }
    }
}
