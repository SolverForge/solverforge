use super::Operator;
use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use crate::stream::relational::{FastSet, HandleMap, RowHandle};
use std::marker::PhantomData;

/// Semi/anti join over concrete row producers WITH a per-pair residual
/// predicate compiled into the plan. Matching pair rows are retained (the
/// residual plan's per-pair path); only left rows are published for scoring.
///
/// For a pure equality join (no residual predicate), use [`ExistenceNode`
/// (count-backed)] instead: it keeps no pair rows at all.
pub struct JoinerExistenceNode<S, L, R, P: IndexedPlan> {
    left: L,
    right: R,
    plan: P,
    indexes: P::Indexes,
    left_matches: HandleMap<Vec<RowHandle>>,
    right_matches: HandleMap<Vec<RowHandle>>,
    accepted: HandleMap<()>,
    /// Link buffers retired by an unlink, reused by the next link.
    ///
    /// A variable change retracts and re-inserts the changed row, so every
    /// maintained link is torn down and rebuilt on each event. Reusing the
    /// retired buffers keeps each row's link list in one allocation for the
    /// life of the search instead of one per event.
    retired: Vec<Vec<RowHandle>>,
    /// Per-event scratch, kept across events so no event allocates for it.
    changed: FastSet<RowHandle>,
    probed: Vec<RowHandle>,
    /// Child notification scratch (L/R, removed/inserted).
    child_l_removed: Vec<RowHandle>,
    child_l_inserted: Vec<RowHandle>,
    child_r_removed: Vec<RowHandle>,
    child_r_inserted: Vec<RowHandle>,
    exists: bool,
    marker: PhantomData<fn() -> S>,
}
impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static>
    JoinerExistenceNode<S, L, R, P>
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
            retired: Vec::new(),
            changed: FastSet::default(),
            probed: Vec::new(),
            child_l_removed: Vec::new(),
            child_l_inserted: Vec::new(),
            child_r_removed: Vec::new(),
            child_r_inserted: Vec::new(),
            exists,
            marker: PhantomData,
        }
    }
    /// One child notification into a reused scratch pair; `side` picks the
    /// left (0) or right (1) scratch buffers.
    fn child_call(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        retract: bool,
        side: usize,
    ) -> (Vec<RowHandle>, Vec<RowHandle>) {
        let (removed, inserted) = if side == 0 {
            (&mut self.child_l_removed, &mut self.child_l_inserted)
        } else {
            (&mut self.child_r_removed, &mut self.child_r_inserted)
        };
        removed.clear();
        inserted.clear();
        if side == 0 {
            if retract {
                self.left
                    .retract(solution, descriptor, index, removed, inserted);
            } else {
                self.left
                    .insert(solution, descriptor, index, removed, inserted);
            }
        } else if retract {
            self.right
                .retract(solution, descriptor, index, removed, inserted);
        } else {
            self.right
                .insert(solution, descriptor, index, removed, inserted);
        }
        let r = if side == 0 {
            std::mem::take(&mut self.child_l_removed)
        } else {
            std::mem::take(&mut self.child_r_removed)
        };
        let i = if side == 0 {
            std::mem::take(&mut self.child_l_inserted)
        } else {
            std::mem::take(&mut self.child_r_inserted)
        };
        (r, i)
    }
    fn link(&mut self, left: RowHandle, right: RowHandle) {
        if self
            .left_matches
            .get(left)
            .is_some_and(|matches| matches.contains(&right))
        {
            return;
        }
        self.left_matches
            .get_or_insert_with(left, || self.retired.pop().unwrap_or_default())
            .push(right);
        self.right_matches
            .get_or_insert_with(right, || self.retired.pop().unwrap_or_default())
            .push(left);
    }
    fn probe_left(&mut self, solution: &S, left: RowHandle) {
        let row = self
            .left
            .resolve(solution, left)
            .expect("live semi-join left row");
        let mut matches = std::mem::take(&mut self.probed);
        matches.clear();
        for &right in self.plan.right_candidates(&self.indexes, &row).iter() {
            let r = self
                .right
                .resolve(solution, right)
                .expect("live semi-join candidate");
            if self.plan.candidate_matches(&row, &r) {
                matches.push(right);
            }
        }
        for right in matches.drain(..) {
            self.link(left, right);
        }
        self.probed = matches;
    }
    fn probe_right(&mut self, solution: &S, right: RowHandle, changed: &mut FastSet<RowHandle>) {
        let row = self
            .right
            .resolve(solution, right)
            .expect("live semi-join right row");
        let mut matches = std::mem::take(&mut self.probed);
        matches.clear();
        for &left in self.plan.left_candidates(&self.indexes, &row).iter() {
            let l = self
                .left
                .resolve(solution, left)
                .expect("live semi-join candidate");
            if self.plan.candidate_matches(&l, &row) {
                matches.push(left);
            }
        }
        for left in matches.drain(..) {
            self.link(left, right);
            changed.insert(left);
        }
        self.probed = matches;
    }
    fn apply_changes(
        &mut self,
        solution: &S,
        left: (Vec<RowHandle>, Vec<RowHandle>),
        right: (Vec<RowHandle>, Vec<RowHandle>),
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        let mut changed = std::mem::take(&mut self.changed);
        changed.clear();
        for h in left.0.iter().copied() {
            self.plan.remove_left(&mut self.indexes, h);
            if let Some(mut matches) = self.left_matches.remove(h) {
                for r in matches.drain(..) {
                    if let Some(right) = self.right_matches.get_mut(r) {
                        right.retain(|l| *l != h);
                    }
                }
                self.retired.push(matches);
            }
            if self.accepted.remove(h).is_some() {
                removed.push(h);
            }
        }
        for h in right.0.iter().copied() {
            self.plan.remove_right(&mut self.indexes, h);
            let Some(mut matches) = self.right_matches.remove(h) else {
                continue;
            };
            for l in matches.drain(..) {
                if let Some(left) = self.left_matches.get_mut(l) {
                    left.retain(|r| *r != h);
                }
                changed.insert(l);
            }
            self.retired.push(matches);
        }
        for h in left.1.iter().copied() {
            let row = self
                .left
                .resolve(solution, h)
                .expect("inserted semi-join left row");
            self.plan.insert_left(&mut self.indexes, h, &row);
        }
        for h in right.1.iter().copied() {
            let row = self
                .right
                .resolve(solution, h)
                .expect("inserted semi-join right row");
            self.plan.insert_right(&mut self.indexes, h, &row);
        }
        for h in left.1.iter().copied() {
            self.probe_left(solution, h);
            changed.insert(h);
        }
        for h in right.1.iter().copied() {
            self.probe_right(solution, h, &mut changed);
        }
        for h in changed.drain() {
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
        self.changed = changed;
    }
}
impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static> Operator<S>
    for JoinerExistenceNode<S, L, R, P>
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
        self.retired.clear();
        self.changed.clear();
        self.probed.clear();
        self.child_l_removed.clear();
        self.child_l_inserted.clear();
        self.child_r_removed.clear();
        self.child_r_inserted.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.clear();
        self.left.initialize(solution);
        self.right.initialize(solution);
        let mut sink_removed = Vec::new();
        let mut sink_inserted = Vec::new();
        self.apply_changes(
            solution,
            (Vec::new(), self.left.handles()),
            (Vec::new(), self.right.handles()),
            &mut sink_removed,
            &mut sink_inserted,
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
        let left = self.child_call(solution, descriptor, index, true, 0);
        let right = self.child_call(solution, descriptor, index, true, 1);
        self.apply_changes(solution, left, right, removed, inserted);
    }
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
        let left = self.child_call(solution, descriptor, index, false, 0);
        let right = self.child_call(solution, descriptor, index, false, 1);
        self.apply_changes(solution, left, right, removed, inserted);
    }
}
