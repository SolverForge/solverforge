use super::{ComplementView, ExistenceNode, Operator, Pair, ProjectView, RowChanges};
use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use crate::stream::relational::{DenseRowStore, HandleMap, JoinedIdentity, RowHandle};
use std::collections::HashMap;

enum Output<T> {
    Real(JoinedIdentity),
    Default(RowHandle, T),
}
/// Fresh owned default values, with one concrete owner for both input evaluations.
#[doc(hidden)]
pub struct ComplementEvaluation<E, T> {
    relation: E,
    defaults: Vec<T>,
}
/// Target-domain left outer join with owned defaults. Inputs, including grouped
/// accumulators, have exactly one owner in the indexed membership relation.
pub struct ComplementNode<S, L, R, P: IndexedPlan, F, T> {
    relation: ExistenceNode<S, L, R, P>,
    default: F,
    rows: DenseRowStore<Output<T>>,
    real_outputs: HashMap<JoinedIdentity, RowHandle>,
    default_outputs: HandleMap<RowHandle>,
    left_outputs: HandleMap<Vec<RowHandle>>,
    right_outputs: HandleMap<Vec<RowHandle>>,
}
impl<S: 'static, L: Operator<S>, R: Operator<S>, P: IndexedPlan + 'static, F, T>
    ComplementNode<S, L, R, P, F, T>
where
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
    F: for<'a> Fn(&S, &L::View<'a>) -> T,
{
    pub fn new<C: CompileCondition<Plan = P>>(
        targets: L,
        results: R,
        condition: C,
        default: F,
    ) -> Self {
        Self {
            relation: ExistenceNode::new(targets, results, condition, false),
            default,
            rows: DenseRowStore::new(),
            real_outputs: HashMap::new(),
            default_outputs: HandleMap::new(),
            left_outputs: HandleMap::new(),
            right_outputs: HandleMap::new(),
        }
    }
    fn remove_output(&mut self, h: RowHandle) {
        if let Some(row) = self.rows.retract(h) {
            match row {
                Output::Real(id) => {
                    self.real_outputs.remove(&id);
                    unlink(&mut self.left_outputs, id.left(), h);
                    unlink(&mut self.right_outputs, id.right(), h);
                }
                Output::Default(input, _) => {
                    self.default_outputs.remove(input);
                    unlink(&mut self.left_outputs, input, h);
                }
            }
        }
    }
    fn emit_real(
        rows: &mut DenseRowStore<Output<T>>,
        outputs: &mut HashMap<JoinedIdentity, RowHandle>,
        left_outputs: &mut HandleMap<Vec<RowHandle>>,
        right_outputs: &mut HandleMap<Vec<RowHandle>>,
        left: RowHandle,
        right: RowHandle,
        inserted: &mut Vec<RowHandle>,
    ) {
        let id = JoinedIdentity::new(left, right, 0);
        if outputs.contains_key(&id) {
            return;
        }
        let h = rows.insert(Output::Real(id));
        outputs.insert(id, h);
        left_outputs.get_or_insert_with(left, Vec::new).push(h);
        right_outputs.get_or_insert_with(right, Vec::new).push(h);
        inserted.push(h);
    }
    fn emit_left(&mut self, left: RowHandle, inserted: &mut Vec<RowHandle>) {
        for &right in self.relation.right_matches(left) {
            Self::emit_real(
                &mut self.rows,
                &mut self.real_outputs,
                &mut self.left_outputs,
                &mut self.right_outputs,
                left,
                right,
                inserted,
            );
        }
    }
    fn emit_default(&mut self, solution: &S, input: RowHandle, inserted: &mut Vec<RowHandle>) {
        assert!(
            self.default_outputs.get(input).is_none(),
            "duplicate default output"
        );
        let row = self
            .relation
            .left()
            .resolve(solution, input)
            .expect("live complement target");
        let value = (self.default)(solution, &row);
        let h = self.rows.insert(Output::Default(input, value));
        self.default_outputs.insert(input, h);
        self.left_outputs
            .get_or_insert_with(input, Vec::new)
            .push(h);
        inserted.push(h);
    }
    fn update(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        retract: bool,
    ) -> RowChanges {
        let (membership, left, right) = self.relation.update(solution, descriptor, index, retract);
        let mut removed = Vec::new();
        for input in left.removed {
            for h in self.left_outputs.remove(input).unwrap_or_default() {
                if self.rows.get(h).is_some() {
                    self.remove_output(h);
                    removed.push(h);
                }
            }
        }
        for input in right.removed {
            for h in self.right_outputs.remove(input).unwrap_or_default() {
                if self.rows.get(h).is_some() {
                    self.remove_output(h);
                    removed.push(h);
                }
            }
        }
        for input in membership.removed {
            if let Some(h) = self.default_outputs.get(input).copied() {
                self.remove_output(h);
                removed.push(h);
            }
        }
        let mut inserted = Vec::new();
        for input in left.inserted {
            self.emit_left(input, &mut inserted);
        }
        for input in right.inserted {
            for &left in self.relation.left_matches(input) {
                Self::emit_real(
                    &mut self.rows,
                    &mut self.real_outputs,
                    &mut self.left_outputs,
                    &mut self.right_outputs,
                    left,
                    input,
                    &mut inserted,
                );
            }
        }
        for input in membership.inserted {
            self.emit_default(solution, input, &mut inserted);
        }
        RowChanges { removed, inserted }
    }
}
impl<
        S: 'static,
        L: Operator<S>,
        R: Operator<S>,
        P: IndexedPlan + 'static,
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
        let relation = self.relation.prepare_evaluation(solution);
        let mut defaults = Vec::new();
        self.relation
            .visit_evaluation(solution, &relation, &mut |row| {
                defaults.push((self.default)(solution, &row))
            });
        ComplementEvaluation { relation, defaults }
    }
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        let mut ordinal = 0;
        self.relation
            .visit_outer_evaluation(solution, &evaluation.relation, &mut |left, right| {
                if let Some(right) = right {
                    visitor(ComplementView::Real(Pair { left, right }));
                } else {
                    visitor(ComplementView::Default(ProjectView {
                        input: left,
                        value: &evaluation.defaults[ordinal],
                        emission: 0,
                    }));
                    ordinal += 1;
                }
            });
    }
    fn clear(&mut self) {
        self.relation.clear();
        self.rows.clear();
        self.real_outputs.clear();
        self.default_outputs.clear();
        self.left_outputs.clear();
        self.right_outputs.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.clear();
        self.relation.initialize(solution);
        for input in self.relation.left().handles() {
            self.emit_left(input, &mut Vec::new());
        }
        for input in self.relation.handles() {
            self.emit_default(solution, input, &mut Vec::new());
        }
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, h: RowHandle) -> Option<Self::View<'a>> {
        match self.rows.get(h)? {
            Output::Real(id) => Some(ComplementView::Real(Pair {
                left: self.relation.left().resolve(solution, id.left())?,
                right: self.relation.right().resolve(solution, id.right())?,
            })),
            Output::Default(input, value) => Some(ComplementView::Default(ProjectView {
                input: self.relation.left().resolve(solution, *input)?,
                value,
                emission: 0,
            })),
        }
    }
    fn visit_provenance(&self, h: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        match self.rows.get(h) {
            Some(Output::Real(id)) => {
                self.relation.left().visit_provenance(id.left(), visitor);
                self.relation.right().visit_provenance(id.right(), visitor);
            }
            Some(Output::Default(input, _)) => {
                self.relation.left().visit_provenance(*input, visitor)
            }
            None => {}
        }
    }
    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        self.update(solution, descriptor, index, true)
    }
    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        self.update(solution, descriptor, index, false)
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
