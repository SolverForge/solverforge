/* Self-join uniqueness operator.

Same-source n-ary streams enumerate unique ordered combinations within one
collection: every binding shares a key, and each row is one increasing source-
index tuple of a fixed arity. This node owns that policy concretely so the
self-join terminals score through the generic `OperatorTerminal`.

The view is `[Leaf<'a, A>; N]` in canonical index order. Retention stores
input handle tuples; identities resolve on demand, and provenance falls out of
each member's own provenance.
*/

use super::{Operator, RowChanges};
use crate::stream::key_extract::KeyExtract;
use crate::stream::relational::{DenseRowStore, HandleMap, Leaf, RowHandle};
use std::collections::HashMap;
use std::marker::PhantomData;

/* Enumerates every increasing `N`-combination of `items` (already index-sorted). */
fn all_combinations<T: Copy, const N: usize>(items: &[T], mut emit: impl FnMut([T; N])) {
    let mut buf: [Option<T>; N] = [None; N];
    fn rec<T: Copy, const N: usize>(
        items: &[T],
        start: usize,
        depth: usize,
        buf: &mut [Option<T>; N],
        emit: &mut impl FnMut([T; N]),
    ) {
        if depth == N {
            emit(std::array::from_fn(|i| buf[i].expect("filled")));
            return;
        }
        for i in start..items.len() {
            buf[depth] = Some(items[i]);
            rec(items, i + 1, depth + 1, buf, emit);
        }
    }
    rec(items, 0, 0, &mut buf, &mut emit);
}

/* Enumerates every increasing `N`-combination of `items` that contains
`items[target]`, keeping canonical index order. `items` is index-sorted. */
fn combinations_with<T: Copy, const N: usize>(
    items: &[T],
    target: usize,
    mut emit: impl FnMut([T; N]),
) {
    let others: Vec<usize> = (0..items.len()).filter(|p| *p != target).collect();
    let mut picks: [usize; N] = [0; N];
    picks[N - 1] = target;
    fn rec<T: Copy, const N: usize>(
        items: &[T],
        others: &[usize],
        start: usize,
        depth: usize,
        picks: &mut [usize; N],
        emit: &mut impl FnMut([T; N]),
    ) {
        if depth == N - 1 {
            let mut ordered = *picks;
            ordered.sort_unstable();
            emit(std::array::from_fn(|i| items[ordered[i]]));
            return;
        }
        for i in start..others.len() {
            picks[depth] = others[i];
            rec(items, others, i + 1, depth + 1, picks, emit);
        }
    }
    rec(items, &others, 0, 0, &mut picks, &mut emit);
}

pub struct SelfJoinNode<S, A, O, K, KE, const N: usize> {
    input: O,
    key: KE,
    /* Key bucket: source index plus handle, kept sorted by index. */
    by_key: HashMap<K, Vec<(usize, RowHandle)>>,
    handle_index: HashMap<RowHandle, usize>,
    rows: DenseRowStore<[RowHandle; N]>,
    combos: HashMap<[RowHandle; N], RowHandle>,
    of_input: HandleMap<Vec<[RowHandle; N]>>,
    marker: PhantomData<fn() -> (S, A)>,
}

impl<S, A, O, K, KE, const N: usize> SelfJoinNode<S, A, O, K, KE, N> {
    pub fn new(input: O, key: KE) -> Self {
        Self {
            input,
            key,
            by_key: HashMap::new(),
            handle_index: HashMap::new(),
            rows: DenseRowStore::new(),
            combos: HashMap::new(),
            of_input: HandleMap::new(),
            marker: PhantomData,
        }
    }
}

impl<S, A, O, K, KE, const N: usize> Operator<S> for SelfJoinNode<S, A, O, K, KE, N>
where
    S: Send + Sync + 'static,
    A: 'static,
    O: Operator<S>,
    for<'a> O: Operator<S, View<'a> = Leaf<'a, A>>,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    KE: KeyExtract<S, A, K> + 'static,
{
    type View<'a> = [Leaf<'a, A>; N];
    type Evaluation = O::Evaluation;

    #[inline]
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        self.input.prepare_evaluation(solution)
    }

    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        // Stream the input's own evaluation; bucket by key, then enumerate.
        let mut buckets: HashMap<K, Vec<Leaf<'a, A>>> = HashMap::new();
        self.input
            .visit_evaluation(solution, evaluation, &mut |leaf: Leaf<'a, A>| {
                let key = self.key.extract(solution, leaf.entity, leaf.index);
                buckets.entry(key).or_default().push(leaf);
            });
        for bucket in buckets.values_mut() {
            bucket.sort_unstable_by_key(|leaf| leaf.index);
            let mut found: Vec<[Leaf<'a, A>; N]> = Vec::new();
            all_combinations(bucket, |tuple| found.push(tuple));
            for tuple in found {
                visitor(tuple);
            }
        }
    }

    fn clear(&mut self) {
        self.input.clear();
        self.by_key.clear();
        self.handle_index.clear();
        self.rows.clear();
        self.combos.clear();
        self.of_input.clear();
    }

    fn initialize(&mut self, solution: &S) {
        self.clear();
        self.input.initialize(solution);
        for handle in self.input.handles() {
            self.insert_row(solution, handle);
        }
    }

    fn handles(&self) -> Vec<RowHandle> {
        self.rows.iter().map(|(h, _)| h).collect()
    }

    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        let tuple = *self.rows.get(handle)?;
        self.view(solution, &tuple)
    }

    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        let Some(tuple) = self.rows.get(handle) else {
            return;
        };
        for input in tuple {
            self.input.visit_provenance(*input, visitor);
        }
    }

    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        let changes = self.input.retract(solution, descriptor, index);
        let mut removed = Vec::new();
        for handle in changes.removed {
            for tuple in self.of_input.remove(handle).unwrap_or_default() {
                if let Some(output) = self.combos.remove(&tuple) {
                    self.rows.retract(output);
                    for member in tuple {
                        if member != handle {
                            if let Some(list) = self.of_input.get_mut(member) {
                                list.retain(|t| *t != tuple);
                            }
                        }
                    }
                    removed.push(output);
                }
            }
            if let Some(index) = self.handle_index.remove(&handle) {
                if let Some(key) = self
                    .by_key
                    .keys()
                    .find(|k| {
                        self.by_key
                            .get(*k)
                            .is_some_and(|b| b.iter().any(|(i, _)| *i == index))
                    })
                    .cloned()
                {
                    if let Some(bucket) = self.by_key.get_mut(&key) {
                        bucket.retain(|(i, _)| *i != index);
                        if bucket.is_empty() {
                            self.by_key.remove(&key);
                        }
                    }
                }
            }
        }
        RowChanges {
            removed,
            inserted: Vec::new(),
        }
    }

    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        let changes = self.input.insert(solution, descriptor, index);
        let mut inserted = Vec::new();
        for handle in changes.inserted {
            inserted.extend(self.insert_row(solution, handle));
        }
        RowChanges {
            removed: Vec::new(),
            inserted,
        }
    }
}

impl<S, A, O, K, KE, const N: usize> SelfJoinNode<S, A, O, K, KE, N>
where
    S: Send + Sync + 'static,
    A: 'static,
    O: Operator<S>,
    for<'a> O: Operator<S, View<'a> = Leaf<'a, A>>,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    KE: KeyExtract<S, A, K> + 'static,
{
    fn view<'a>(&'a self, solution: &'a S, tuple: &[RowHandle; N]) -> Option<[Leaf<'a, A>; N]> {
        let mut out = Vec::with_capacity(N);
        for handle in tuple {
            out.push(self.input.resolve(solution, *handle)?);
        }
        out.try_into().ok()
    }

    fn insert_row(&mut self, solution: &S, handle: RowHandle) -> Vec<RowHandle> {
        let Some(leaf) = self.input.resolve(solution, handle) else {
            return Vec::new();
        };
        let index = leaf.index;
        let key = self.key.extract(solution, leaf.entity, index);
        self.handle_index.insert(handle, index);
        let bucket = self.by_key.entry(key).or_default();
        let at = bucket.partition_point(|(i, _)| *i < index);
        bucket.insert(at, (index, handle));
        let handles: Vec<RowHandle> = bucket.iter().map(|(_, h)| *h).collect();
        let target = at;

        let mut found: Vec<[RowHandle; N]> = Vec::new();
        combinations_with(&handles, target, |tuple| found.push(tuple));

        let mut inserted = Vec::new();
        for tuple in found {
            if self.combos.contains_key(&tuple) {
                continue;
            }
            let output = self.rows.insert(tuple);
            self.combos.insert(tuple, output);
            for member in tuple {
                self.of_input
                    .get_or_insert_with(member, Vec::new)
                    .push(tuple);
            }
            inserted.push(output);
        }
        inserted
    }
}
