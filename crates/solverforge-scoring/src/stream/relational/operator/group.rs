use super::{group_view::Contributors, GroupView, Operator};
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::relational::{DenseRowStore, FastMap, HandleMap, RowHandle};
use std::hash::Hash;
use std::marker::PhantomData;

struct Group<K, A> {
    key: K,
    accumulator: A,
    inputs: Vec<RowHandle>,
    dirty: bool,
}
struct EvaluatedGroup<K, A> {
    key: K,
    accumulator: A,
    ordinals: Vec<usize>,
}
/// Concrete full-evaluation group owner; accumulators own all collected payloads.
#[doc(hidden)]
pub struct GroupEvaluation<E, K, A> {
    input: E,
    groups: Vec<EvaluatedGroup<K, A>>,
}
/// Group arbitrary input rows by a typed key, retaining exact collector tokens.
pub struct GroupNode<O, F, C, K, A: Accumulator<V, R>, V, R> {
    input: O,
    key: F,
    collector: C,
    groups: DenseRowStore<Group<K, A>>,
    by_key: FastMap<K, RowHandle>,
    tokens: HandleMap<(RowHandle, A::Retraction, usize)>,
    rows: DenseRowStore<RowHandle>,
    output_of: HandleMap<RowHandle>,
    changed: Vec<RowHandle>,
    /// Child notification scratch, reused across events.
    child_removed: Vec<RowHandle>,
    child_inserted: Vec<RowHandle>,
    marker: PhantomData<fn() -> (V, R)>,
}
impl<O, F, C, K, A: Accumulator<V, R>, V, R> GroupNode<O, F, C, K, A, V, R> {
    pub fn new(input: O, key: F, collector: C) -> Self {
        Self {
            input,
            key,
            collector,
            groups: DenseRowStore::new(),
            by_key: FastMap::default(),
            tokens: HandleMap::new(),
            rows: DenseRowStore::new(),
            output_of: HandleMap::new(),
            changed: Vec::new(),
            child_removed: Vec::new(),
            child_inserted: Vec::new(),
            marker: PhantomData,
        }
    }
    fn apply_changes<S: 'static>(
        &mut self,
        solution: &S,
        input_removed: &[RowHandle],
        input_inserted: &[RowHandle],
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) where
        O: Operator<S>,
        F: for<'a> Fn(&O::View<'a>) -> K,
        for<'a> C: Collector<O::View<'a>, Value = V, Result = R, Accumulator = A>,
        K: Eq + Hash + Clone,
    {
        let mut changed = std::mem::take(&mut self.changed);
        changed.clear();
        for h in input_removed.iter().copied() {
            if let Some((group, token, position)) = self.tokens.remove(h) {
                let g = self
                    .groups
                    .get_mut(group)
                    .expect("retained contributor group");
                g.accumulator.retract(token);
                debug_assert_eq!(g.inputs[position], h);
                g.inputs.swap_remove(position);
                if let Some(&moved) = g.inputs.get(position) {
                    self.tokens
                        .get_mut(moved)
                        .expect("moved contributor token")
                        .2 = position;
                }
                if !g.dirty {
                    g.dirty = true;
                    changed.push(group);
                }
            }
        }
        for h in input_inserted.iter().copied() {
            assert!(
                self.tokens.get(h).is_none(),
                "duplicate grouped contributor"
            );
            let row = self
                .input
                .resolve(solution, h)
                .expect("inserted group input");
            let key = (self.key)(&row);
            let group = if let Some(&group) = self.by_key.get(&key) {
                group
            } else {
                let group = self.groups.insert(Group {
                    key: key.clone(),
                    accumulator: self.collector.create_accumulator(),
                    inputs: Vec::new(),
                    dirty: false,
                });
                self.by_key.insert(key, group);
                group
            };
            let g = self.groups.get_mut(group).expect("live insertion group");
            let token = g.accumulator.accumulate(self.collector.extract(row));
            let position = g.inputs.len();
            g.inputs.push(h);
            self.tokens.insert(h, (group, token, position));
            if !g.dirty {
                g.dirty = true;
                changed.push(group);
            }
        }
        // Publish exactly one final replacement per group after all contributors update.
        for group in changed.iter().copied() {
            if let Some(output) = self.output_of.remove(group) {
                self.rows.retract(output);
                removed.push(output);
            }
            let g = self.groups.get_mut(group).expect("changed group");
            g.dirty = false;
            if g.inputs.is_empty() {
                self.by_key.remove(&g.key);
                self.groups.retract(group);
            } else {
                let output = self.rows.insert(group);
                self.output_of.insert(group, output);
                inserted.push(output);
            }
        }
        changed.clear();
        self.changed = changed;
    }
}
impl<S: 'static, O, F, C, K, A, V: 'static, R: 'static> Operator<S>
    for GroupNode<O, F, C, K, A, V, R>
where
    O: Operator<S>,
    F: for<'a> Fn(&O::View<'a>) -> K + 'static,
    for<'a> C: Collector<O::View<'a>, Value = V, Result = R, Accumulator = A> + 'static,
    K: Eq + Hash + Clone + 'static,
    A: Accumulator<V, R> + 'static,
{
    type View<'a> = GroupView<'a, S, O, K, A, V, R>;
    type Evaluation = GroupEvaluation<O::Evaluation, K, A>;
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        let input = self.input.prepare_evaluation(solution);
        let mut groups: Vec<EvaluatedGroup<K, A>> = Vec::new();
        let mut by_key = FastMap::default();
        let mut ordinal = 0;
        self.input.visit_evaluation(solution, &input, &mut |row| {
            let key = (self.key)(&row);
            let index = match by_key.get(&key) {
                Some(&index) => index,
                None => {
                    let index = groups.len();
                    groups.push(EvaluatedGroup {
                        key,
                        accumulator: self.collector.create_accumulator(),
                        ordinals: Vec::new(),
                    });
                    by_key.insert(groups[index].key.clone(), index);
                    index
                }
            };
            groups[index]
                .accumulator
                .accumulate(self.collector.extract(row));
            groups[index].ordinals.push(ordinal);
            ordinal += 1;
        });
        GroupEvaluation { input, groups }
    }
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        for group in &evaluation.groups {
            visitor(GroupView {
                key: &group.key,
                accumulator: &group.accumulator,
                input: &self.input,
                solution,
                contributors: Contributors::Evaluation(&evaluation.input, &group.ordinals),
                marker: PhantomData,
            });
        }
    }
    fn clear(&mut self) {
        self.input.clear();
        self.groups.clear();
        self.by_key.clear();
        self.tokens.clear();
        self.rows.clear();
        self.output_of.clear();
        self.changed.clear();
        self.child_removed.clear();
        self.child_inserted.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.clear();
        self.input.initialize(solution);
        let inserted = self.input.handles();
        let mut sink_removed = Vec::new();
        let mut sink_inserted = Vec::new();
        self.apply_changes(
            solution,
            &[],
            &inserted,
            &mut sink_removed,
            &mut sink_inserted,
        );
    }
    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }
    fn resolve<'a>(&'a self, solution: &'a S, h: RowHandle) -> Option<Self::View<'a>> {
        let group = self.groups.get(*self.rows.get(h)?)?;
        Some(GroupView {
            key: &group.key,
            accumulator: &group.accumulator,
            input: &self.input,
            solution,
            contributors: Contributors::Retained(&group.inputs),
            marker: PhantomData,
        })
    }
    fn visit_provenance(&self, h: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if let Some(group) = self.rows.get(h).and_then(|g| self.groups.get(*g)) {
            for &input in &group.inputs {
                self.input.visit_provenance(input, visitor);
            }
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
        let mut in_removed = std::mem::take(&mut self.child_removed);
        let mut in_inserted = std::mem::take(&mut self.child_inserted);
        in_removed.clear();
        in_inserted.clear();
        self.input.retract(
            solution,
            descriptor,
            index,
            &mut in_removed,
            &mut in_inserted,
        );
        self.apply_changes(solution, &in_removed, &in_inserted, removed, inserted);
        self.child_removed = in_removed;
        self.child_inserted = in_inserted;
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
        let mut in_removed = std::mem::take(&mut self.child_removed);
        let mut in_inserted = std::mem::take(&mut self.child_inserted);
        in_removed.clear();
        in_inserted.clear();
        self.input.insert(
            solution,
            descriptor,
            index,
            &mut in_removed,
            &mut in_inserted,
        );
        self.apply_changes(solution, &in_removed, &in_inserted, removed, inserted);
        self.child_removed = in_removed;
        self.child_inserted = in_inserted;
    }
}
