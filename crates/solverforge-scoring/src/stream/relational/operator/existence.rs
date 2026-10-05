use super::{Operator, RowChanges};
use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use crate::stream::relational::{HandleMap, RowHandle};
use std::collections::HashSet;
use std::marker::PhantomData;

/// Semi/anti join over concrete row producers. Matching links maintain counts;
/// only left rows are published, never scored Cartesian pair rows.
pub struct ExistenceNode<S, L, R, P: IndexedPlan> {
    left: L,
    right: R,
    plan: P,
    indexes: P::Indexes,
    left_matches: HandleMap<Vec<RowHandle>>,
    right_matches: HandleMap<Vec<RowHandle>>,
    accepted: HandleMap<()>,
    exists: bool,
    marker: PhantomData<fn() -> S>,
}
impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static> ExistenceNode<S, L, R, P>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
{
    pub fn new<C: CompileCondition<Plan = P>>(
        left: L,
        right: R,
        condition: C,
        exists: bool,
    ) -> Self {
        let plan = condition.compile();
        let indexes = plan.new_indexes();
        Self {
            left,
            right,
            plan,
            indexes,
            left_matches: HandleMap::new(),
            right_matches: HandleMap::new(),
            accepted: HandleMap::new(),
            exists,
            marker: PhantomData,
        }
    }
    pub(super) fn left(&self) -> &L {
        &self.left
    }
    pub(super) fn right(&self) -> &R {
        &self.right
    }
    pub(super) fn right_matches(&self, left: RowHandle) -> &[RowHandle] {
        self.left_matches.get(left).map_or(&[], Vec::as_slice)
    }
    pub(super) fn left_matches(&self, right: RowHandle) -> &[RowHandle] {
        self.right_matches.get(right).map_or(&[], Vec::as_slice)
    }
    pub(super) fn update(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        retract: bool,
    ) -> (RowChanges, RowChanges, RowChanges) {
        let left = if retract {
            self.left.retract(solution, descriptor, index)
        } else {
            self.left.insert(solution, descriptor, index)
        };
        let right = if retract {
            self.right.retract(solution, descriptor, index)
        } else {
            self.right.insert(solution, descriptor, index)
        };
        let membership = self.apply_changes(solution, &left, &right);
        (membership, left, right)
    }
    pub(super) fn visit_outer_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a (L::Evaluation, R::Evaluation),
        visitor: &mut impl FnMut(L::View<'a>, Option<R::View<'a>>),
    ) {
        let mut indexes = self.plan.new_indexes();
        let mut right_rows = Vec::new();
        self.right
            .visit_evaluation(solution, &evaluation.1, &mut |row| {
                let h = RowHandle::new(right_rows.len() as u32, 0);
                self.plan.insert_right_transient(&mut indexes, h, &row);
                right_rows.push(row);
            });
        self.left
            .visit_evaluation(solution, &evaluation.0, &mut |left| {
                let mut matched = false;
                for &h in self.plan.right_candidates(&indexes, &left).iter() {
                    let right = right_rows[h.slot() as usize];
                    if self.plan.candidate_matches(&left, &right) {
                        matched = true;
                        visitor(left, Some(right));
                    }
                }
                if !matched {
                    visitor(left, None);
                }
            });
    }
    fn link(&mut self, left: RowHandle, right: RowHandle) {
        let matches = self.left_matches.get_or_insert_with(left, Vec::new);
        if matches.contains(&right) {
            return;
        }
        matches.push(right);
        self.right_matches
            .get_or_insert_with(right, Vec::new)
            .push(left);
    }
    fn probe_left(&mut self, solution: &S, left: RowHandle) {
        let row = self
            .left
            .resolve(solution, left)
            .expect("live semi-join left row");
        let candidates = self.plan.right_candidates(&self.indexes, &row);
        let mut matches = Vec::new();
        for &right in candidates.iter() {
            let r = self
                .right
                .resolve(solution, right)
                .expect("live semi-join candidate");
            if self.plan.candidate_matches(&row, &r) {
                matches.push(right);
            }
        }
        for right in matches {
            self.link(left, right);
        }
    }
    fn probe_right(&mut self, solution: &S, right: RowHandle, changed: &mut HashSet<RowHandle>) {
        let row = self
            .right
            .resolve(solution, right)
            .expect("live semi-join right row");
        let candidates = self.plan.left_candidates(&self.indexes, &row);
        let mut matches = Vec::new();
        for &left in candidates.iter() {
            let l = self
                .left
                .resolve(solution, left)
                .expect("live semi-join candidate");
            if self.plan.candidate_matches(&l, &row) {
                matches.push(left);
            }
        }
        for left in matches {
            self.link(left, right);
            changed.insert(left);
        }
    }
    fn apply_changes(&mut self, solution: &S, left: &RowChanges, right: &RowChanges) -> RowChanges {
        let mut changed = HashSet::new();
        let mut removed = Vec::new();
        for h in left.removed.iter().copied() {
            self.plan.remove_left(&mut self.indexes, h);
            for r in self.left_matches.remove(h).unwrap_or_default() {
                if let Some(matches) = self.right_matches.get_mut(r) {
                    matches.retain(|l| *l != h);
                }
            }
            if self.accepted.remove(h).is_some() {
                removed.push(h);
            }
        }
        for h in right.removed.iter().copied() {
            self.plan.remove_right(&mut self.indexes, h);
            for l in self.right_matches.remove(h).unwrap_or_default() {
                if let Some(matches) = self.left_matches.get_mut(l) {
                    matches.retain(|r| *r != h);
                }
                changed.insert(l);
            }
        }
        for &h in &left.inserted {
            let row = self
                .left
                .resolve(solution, h)
                .expect("inserted semi-join left row");
            self.plan.insert_left(&mut self.indexes, h, &row);
        }
        for &h in &right.inserted {
            let row = self
                .right
                .resolve(solution, h)
                .expect("inserted semi-join right row");
            self.plan.insert_right(&mut self.indexes, h, &row);
        }
        for h in left.inserted.iter().copied() {
            self.probe_left(solution, h);
            changed.insert(h);
        }
        for h in right.inserted.iter().copied() {
            self.probe_right(solution, h, &mut changed);
        }
        let mut inserted = Vec::new();
        for h in changed {
            if self.left.resolve(solution, h).is_none() {
                continue;
            }
            let matches = self.left_matches.get(h).is_some_and(|m| !m.is_empty());
            let accepted = matches == self.exists;
            let was_accepted = self.accepted.get(h).is_some();
            if accepted && !was_accepted {
                self.accepted.insert(h, ());
                inserted.push(h);
            } else if !accepted && was_accepted {
                self.accepted.remove(h);
                removed.push(h);
            }
        }
        RowChanges { removed, inserted }
    }
}
impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static> Operator<S>
    for ExistenceNode<S, L, R, P>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
{
    type View<'a> = L::View<'a>;
    type Evaluation = (L::Evaluation, R::Evaluation);
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        (
            self.left.prepare_evaluation(solution),
            self.right.prepare_evaluation(solution),
        )
    }
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        let mut indexes = self.plan.new_indexes();
        let mut right_rows = Vec::new();
        self.right
            .visit_evaluation(solution, &evaluation.1, &mut |row| {
                let h = RowHandle::new(right_rows.len() as u32, 0);
                self.plan.insert_right_transient(&mut indexes, h, &row);
                right_rows.push(row);
            });
        self.left
            .visit_evaluation(solution, &evaluation.0, &mut |left| {
                let matched = self.plan.right_candidates(&indexes, &left).iter().any(|h| {
                    self.plan
                        .candidate_matches(&left, &right_rows[h.slot() as usize])
                });
                if matched == self.exists {
                    visitor(left);
                }
            });
    }
    fn clear(&mut self) {
        self.left.clear();
        self.right.clear();
        self.indexes = self.plan.new_indexes();
        self.left_matches.clear();
        self.right_matches.clear();
        self.accepted.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.clear();
        self.left.initialize(solution);
        self.right.initialize(solution);
        self.apply_changes(
            solution,
            &RowChanges {
                removed: Vec::new(),
                inserted: self.left.handles(),
            },
            &RowChanges {
                removed: Vec::new(),
                inserted: self.right.handles(),
            },
        );
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.left
            .handles()
            .into_iter()
            .filter(|h| self.accepted.get(*h).is_some())
            .collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, h: RowHandle) -> Option<Self::View<'a>> {
        self.accepted.get(h)?;
        self.left.resolve(solution, h)
    }
    fn visit_provenance(&self, h: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if self.accepted.get(h).is_some() {
            self.left.visit_provenance(h, visitor);
        }
    }
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        self.update(solution, descriptor, index, true).0
    }
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        self.update(solution, descriptor, index, false).0
    }
}
