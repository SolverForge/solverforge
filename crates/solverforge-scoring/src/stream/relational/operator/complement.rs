use super::{ComplementView, Operator, Pair, PairDelta, ProjectView};
use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use crate::stream::relational::hash::FastMap;
use crate::stream::relational::keyed_plan::{KeyOf, RightKeyOf};
use crate::stream::relational::{DenseRowStore, HandleMap, JoinedIdentity, RowHandle};
use std::marker::PhantomData;

mod outputs;

struct Output<T> {
    value: OutputValue<T>,
    left_position: usize,
    right_position: usize,
}

enum OutputValue<T> {
    Real(JoinedIdentity),
    Default(RowHandle, T),
}
/// Fresh owned default values, with one concrete owner for both input evaluations.
#[doc(hidden)]
pub struct ComplementEvaluation<E, T> {
    left: E,
    defaults: Vec<T>,
}
/// Target-domain left outer join with owned defaults. Inputs, including grouped
/// accumulators, have exactly one owner in the indexed membership relation.
///
/// The complement is a scored join: each matching (target, result) pair is a
/// published row and each unmatched target emits its default, so the node owns
/// the equality plan's indexes directly — the same retention shape as
/// `JoinNode`. Membership per se lives in the count-backed `ExistenceNode`;
/// this node never reuses that engine.
pub struct ComplementNode<
    S,
    L,
    R,
    P: IndexedPlan + KeyOf + RightKeyOf<Key = <P as KeyOf>::Key>,
    F,
    T,
> {
    targets: L,
    results: R,
    plan: P,
    indexes: P::Indexes,
    default: F,
    rows: DenseRowStore<Output<T>>,
    real_outputs: FastMap<JoinedIdentity, RowHandle>,
    default_outputs: HandleMap<RowHandle>,
    left_outputs: HandleMap<Vec<RowHandle>>,
    right_outputs: HandleMap<Vec<RowHandle>>,
    /// Child notification scratch, reused across events (targets/results).
    child_l_removed: Vec<RowHandle>,
    child_l_inserted: Vec<RowHandle>,
    child_r_removed: Vec<RowHandle>,
    child_r_inserted: Vec<RowHandle>,
    marker: PhantomData<fn() -> S>,
}
impl<S: 'static, L: Operator<S>, R: Operator<S>, P, F, T> ComplementNode<S, L, R, P, F, T>
where
    P: IndexedPlan + KeyOf<Key = <P as RightKeyOf>::Key> + RightKeyOf + 'static,
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
    F: for<'a> Fn(&S, &L::View<'a>) -> T,
{
    /// Creates the node: `targets` rows are matched against `results` rows by
    /// `condition`, and an unmatched target emits `default(solution, target)`.
    pub fn new<C: CompileCondition<Plan = P>>(
        targets: L,
        results: R,
        condition: C,
        default: F,
    ) -> Self {
        let plan = condition.compile();
        let indexes = plan.new_indexes();
        Self {
            targets,
            results,
            plan,
            indexes,
            default,
            rows: DenseRowStore::new(),
            real_outputs: FastMap::default(),
            default_outputs: HandleMap::new(),
            left_outputs: HandleMap::new(),
            right_outputs: HandleMap::new(),
            child_l_removed: Vec::new(),
            child_l_inserted: Vec::new(),
            child_r_removed: Vec::new(),
            child_r_inserted: Vec::new(),
            marker: PhantomData,
        }
    }
    fn update(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        retract: bool,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        let mut lm = std::mem::take(&mut self.child_l_removed);
        let mut li = std::mem::take(&mut self.child_l_inserted);
        lm.clear();
        li.clear();
        let mut rm = std::mem::take(&mut self.child_r_removed);
        let mut ri = std::mem::take(&mut self.child_r_inserted);
        rm.clear();
        ri.clear();
        if retract {
            self.targets
                .retract(solution, descriptor, index, &mut lm, &mut li);
            self.results
                .retract(solution, descriptor, index, &mut rm, &mut ri);
        } else {
            self.targets
                .insert(solution, descriptor, index, &mut lm, &mut li);
            self.results
                .insert(solution, descriptor, index, &mut rm, &mut ri);
        }
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
    fn apply_changes(
        &mut self,
        solution: &S,
        delta: PairDelta<'_>,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        let PairDelta {
            left_removed: left,
            left_inserted,
            right_removed: right,
            right_inserted,
        } = delta;
        for handle in left.iter().copied() {
            self.plan.remove_left(&mut self.indexes, handle);
            for h in self.left_outputs.remove(handle).unwrap_or_default() {
                if self.rows.get(h).is_some() {
                    self.remove_output(h);
                    removed.push(h);
                }
            }
        }
        // Keys of the result rows this event removes, read back from the
        // index before it drops them; their targets may regain a default
        // after the removals and re-probes below.
        let removed_keys: Vec<<P as KeyOf>::Key> = right
            .iter()
            .copied()
            .filter_map(|handle| self.plan.right_row_key(&self.indexes, handle).cloned())
            .collect();
        for handle in right.iter().copied() {
            self.plan.remove_right(&mut self.indexes, handle);
            for h in self.right_outputs.remove(handle).unwrap_or_default() {
                if self.rows.get(h).is_some() {
                    self.remove_output(h);
                    removed.push(h);
                }
            }
        }
        for handle in left_inserted.iter().copied() {
            if let Some(row) = self.targets.resolve(solution, handle) {
                self.plan.insert_left(&mut self.indexes, handle, &row);
            }
        }
        for handle in right_inserted.iter().copied() {
            if let Some(row) = self.results.resolve(solution, handle) {
                self.plan.insert_right(&mut self.indexes, handle, &row);
            }
        }
        for handle in left_inserted.iter().copied() {
            // New target: emit its matching real rows, or its default.
            let candidates = self.target_candidates(&handle, solution);
            let mut emitted = false;
            for right in candidates {
                if Self::emit_real(
                    &mut self.rows,
                    &mut self.real_outputs,
                    &mut self.left_outputs,
                    &mut self.right_outputs,
                    handle,
                    right,
                    inserted,
                )
                .is_some()
                {
                    emitted = true;
                }
            }
            if !emitted {
                self.emit_default(solution, handle, inserted);
            }
        }
        for handle in right_inserted.iter().copied() {
            // New result row: emit a real row for every matching target.
            let Some(row) = self.results.resolve(solution, handle) else {
                continue;
            };
            let matches: Vec<RowHandle> = self
                .plan
                .left_candidates(&self.indexes, &row)
                .iter()
                .copied()
                .filter(|&left| {
                    self.targets
                        .resolve(solution, left)
                        .is_some_and(|lrow| self.plan.candidate_matches(&lrow, &row))
                })
                .collect();
            for left in matches {
                // A target that only had a default loses it to the real row.
                if let Some(default_h) = self.default_outputs.get(left).copied() {
                    self.remove_output(default_h);
                    removed.push(default_h);
                }
                Self::emit_real(
                    &mut self.rows,
                    &mut self.real_outputs,
                    &mut self.left_outputs,
                    &mut self.right_outputs,
                    left,
                    handle,
                    inserted,
                );
            }
        }
        // A target that lost its last real row regains its default. The
        // candidates are re-derived from the live left index under the removed
        // result rows' keys, so every current target generation is seen.
        for key in removed_keys {
            let owned: Vec<RowHandle> = self
                .plan
                .left_candidates_by_key(&self.indexes, &key)
                .iter()
                .copied()
                .collect();
            for left in owned {
                let has_output = self
                    .left_outputs
                    .get(left)
                    .is_some_and(|outs| !outs.is_empty());
                if !has_output && self.targets.resolve(solution, left).is_some() {
                    self.emit_default(solution, left, inserted);
                }
            }
        }
    }
    fn target_candidates(&self, handle: &RowHandle, solution: &S) -> Vec<RowHandle> {
        let Some(row) = self.targets.resolve(solution, *handle) else {
            return Vec::new();
        };
        self.plan
            .right_candidates(&self.indexes, &row)
            .iter()
            .copied()
            .filter(|&right| {
                self.results
                    .resolve(solution, right)
                    .is_some_and(|rrow| self.plan.candidate_matches(&row, &rrow))
            })
            .collect()
    }
}
impl<
        S: 'static,
        L: Operator<S>,
        R: Operator<S>,
        P: IndexedPlan + KeyOf + RightKeyOf<Key = <P as KeyOf>::Key> + 'static,
        F: 'static,
        T: 'static,
    > Operator<S> for ComplementNode<S, L, R, P, F, T>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
    F: for<'a> Fn(&S, &L::View<'a>) -> T,
{
    type View<'a> = ComplementView<'a, L::View<'a>, R::View<'a>, T>;
    type Evaluation = ComplementEvaluation<(L::Evaluation, R::Evaluation), T>;
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        let targets = self.targets.prepare_evaluation(solution);
        let results = self.results.prepare_evaluation(solution);
        let mut defaults = Vec::new();
        self.targets
            .visit_evaluation(solution, &targets, &mut |row| {
                defaults.push((self.default)(solution, &row))
            });
        ComplementEvaluation {
            left: (targets, results),
            defaults,
        }
    }
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        let mut indexes = self.plan.new_indexes();
        let mut result_rows = Vec::new();
        self.results
            .visit_evaluation(solution, &evaluation.left.1, &mut |row| {
                let h = RowHandle::new(result_rows.len() as u32, 0);
                self.plan.insert_right_transient(&mut indexes, h, &row);
                result_rows.push(row);
            });
        let mut ordinal = 0;
        self.targets
            .visit_evaluation(solution, &evaluation.left.0, &mut |left| {
                let mut matched = false;
                for &h in self.plan.right_candidates(&indexes, &left).iter() {
                    let right = result_rows[h.slot() as usize];
                    if self.plan.candidate_matches(&left, &right) {
                        matched = true;
                        visitor(ComplementView::Real(Pair { left, right }));
                    }
                }
                if !matched {
                    visitor(ComplementView::Default(ProjectView {
                        input: left,
                        value: &evaluation.defaults[ordinal],
                        emission: 0,
                    }));
                }
                ordinal += 1;
            });
    }
    fn clear(&mut self) {
        self.targets.clear();
        self.results.clear();
        self.indexes = self.plan.new_indexes();
        self.rows.clear();
        self.real_outputs.clear();
        self.default_outputs.clear();
        self.left_outputs.clear();
        self.right_outputs.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.clear();
        self.targets.initialize(solution);
        self.results.initialize(solution);
        for handle in self.targets.handles() {
            if let Some(row) = self.targets.resolve(solution, handle) {
                self.plan.insert_left(&mut self.indexes, handle, &row);
            }
        }
        for handle in self.results.handles() {
            if let Some(row) = self.results.resolve(solution, handle) {
                self.plan.insert_right(&mut self.indexes, handle, &row);
            }
        }
        for handle in self.targets.handles() {
            let candidates = self.target_candidates(&handle, solution);
            let mut inserted = Vec::new();
            let mut emitted = false;
            for right in candidates {
                if Self::emit_real(
                    &mut self.rows,
                    &mut self.real_outputs,
                    &mut self.left_outputs,
                    &mut self.right_outputs,
                    handle,
                    right,
                    &mut inserted,
                )
                .is_some()
                {
                    emitted = true;
                }
            }
            if !emitted {
                self.emit_default(solution, handle, &mut inserted);
            }
        }
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, h: RowHandle) -> Option<Self::View<'a>> {
        match &self.rows.get(h)?.value {
            OutputValue::Real(id) => Some(ComplementView::Real(Pair {
                left: self.targets.resolve(solution, id.left())?,
                right: self.results.resolve(solution, id.right())?,
            })),
            OutputValue::Default(input, value) => Some(ComplementView::Default(ProjectView {
                input: self.targets.resolve(solution, *input)?,
                value,
                emission: 0,
            })),
        }
    }
    fn visit_provenance(&self, h: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        match self.rows.get(h).map(|row| &row.value) {
            Some(OutputValue::Real(id)) => {
                self.targets.visit_provenance(id.left(), visitor);
                self.results.visit_provenance(id.right(), visitor);
            }
            Some(OutputValue::Default(input, _)) => self.targets.visit_provenance(*input, visitor),
            None => {}
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
        self.update(solution, descriptor, index, true, removed, inserted)
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
        self.update(solution, descriptor, index, false, removed, inserted)
    }
}
