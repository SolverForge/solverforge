use crate::stream::relational::count_state::KeyState;
use super::{Operator, PairDelta};
use super::group_view::GroupView;
use crate::stream::collector::Accumulator;
use crate::stream::collection_extract::FlattenExtract;
use crate::stream::relational::view_plan::ViewKey;
use crate::stream::relational::{FastMap, HandleMap, Leaf, RowHandle};
use std::hash::Hash;
use std::marker::PhantomData;

/* Membership key source for one right-side row.

Plain rows contribute their single equality key; a parent row contributes the
multiset of its children's keys, read straight from the solution. Either way
the contribution is pushed into the node's reused scratch buffer — right-side
children never become rows and never receive handles.
*/
pub trait RightKeySet<V>: Send + Sync {
    type Key;

    fn push_keys(&self, view: &V, out: &mut Vec<Self::Key>);
}

/* One row, one key. */
pub struct UnitKeys<VK>(VK);
impl<VK> UnitKeys<VK> {
    pub fn new(inner: VK) -> Self {
        Self(inner)
    }
}
impl<V, VK, K> RightKeySet<V> for UnitKeys<VK>
where
    VK: ViewKey<V, Key = K> + Send + Sync,
    K: Clone,
{
    type Key = K;

    fn push_keys(&self, view: &V, out: &mut Vec<K>) {
        out.push(self.0.key(view));
    }
}

/* One parent row, its children's keys. The children are read through the
solution-borrowed flatten extractor, so a changed parent costs one pass over
its own elements and nothing is materialized. */
pub struct ParentChildKeys<KB, F> {
    key_b: KB,
    flatten: F,
}
impl<KB, F> ParentChildKeys<KB, F> {
    pub fn new(key_b: KB, flatten: F) -> Self {
        Self { key_b, flatten }
    }
}
impl<P, B, KB, F, K> RightKeySet<Leaf<'_, P>> for ParentChildKeys<KB, F>
where
    KB: Fn(&B) -> K + Send + Sync,
    F: FlattenExtract<P, Item = B> + Send + Sync,
    K: Clone,
{
    type Key = K;

    fn push_keys(&self, view: &Leaf<'_, P>, out: &mut Vec<K>) {
        for child in self.flatten.extract(view.entity) {
            out.push((self.key_b)(child));
        }
    }
}

/* Pushes one key per grouped row: the group's aggregated result, read through
the accumulator so grouped-existence keys off the aggregate like any other view
key. */
pub struct GroupRightKeys<RK>(RK);

impl<RK> GroupRightKeys<RK> {
    pub fn new(inner: RK) -> Self {
        Self(inner)
    }
}

impl<'a, S: 'static, O: Operator<S>, K, A: Accumulator<V, R>, V, R, RK>
    RightKeySet<GroupView<'a, S, O, K, A, V, R>> for GroupRightKeys<RK>
where
    RK: Fn(&GroupView<'a, S, O, K, A, V, R>) -> K + Send + Sync,
    K: Clone + 'static,
{
    type Key = K;

    fn push_keys(&self, view: &GroupView<'a, S, O, K, A, V, R>, out: &mut Vec<K>) {
        out.push((self.0)(view));
    }
}

/* Semi/anti join over concrete row producers, answered from per-key counts.

The node retains one occurrence count per right-side key, each right row's
stored key contribution, and one key bucket of left rows per key. A key whose
count crosses the alive/dead threshold publishes or retracts every left row
carrying it; nothing else changes. Left rows are re-keyed from the live
solution on insert, so a changed key moves the row between key domains without
pair bookkeeping. Only left rows are published, never scored pair rows.
*/
pub struct ExistenceNode<S, L, R, K, LK, RK> {
    left: L,
    right: R,
    left_key: LK,
    right_keys: RK,
    /// Per-key occurrence counts, left-row buckets and the current event's
    /// touched keys. The `usize`-keyed case stores dense arrays instead of
    /// hash tables, dispatched once at construction.
    keys: KeyState<K>,
    /// Per-event keys whose counts changed, in first-touch order.
    /// Emptied by the membership pass at the end of every event.
    touched: Vec<K>,
    /// Each right row's stored key contribution, for retraction without
    /// re-deriving keys from mutated values.
    right_stored: HandleMap<Vec<K>>,
    /// Each left row's current key.
    left_keys: HandleMap<K>,
    /// Whether each left row is published (its key alive matches `exists`),
    /// mirrored by membership in `accepted`.
    accepted: HandleMap<()>,
    /// Contribution buffers retired by a right retraction, reused by the next
    /// insert so each row keeps one buffer for the life of the search.
    retired: Vec<Vec<K>>,
    /// Per-event scratch for reading one row's key contribution.
    key_scratch: Vec<K>,
    /// Reused per-event notification buffers (left/right, removed/inserted)
    /// plus the published result buffers, so no event allocates.
    left_removed: Vec<RowHandle>,
    left_inserted: Vec<RowHandle>,
    right_removed: Vec<RowHandle>,
    right_inserted: Vec<RowHandle>,
    removed: Vec<RowHandle>,
    inserted: Vec<RowHandle>,
    /// Per-event scratch for membership-flip handles by polarity, so settled
    /// keys never allocate.
    flip_ins: Vec<RowHandle>,
    flip_rem: Vec<RowHandle>,
    exists: bool,
    marker: PhantomData<fn() -> S>,
}
impl<S: 'static, L: Operator<S>, R: Operator<S>, K, LK, RK> ExistenceNode<S, L, R, K, LK, RK>
where
    K: Eq + Hash + Clone + 'static,
    for<'a> LK: ViewKey<L::View<'a>, Key = K>,
    for<'a> RK: RightKeySet<R::View<'a>, Key = K>,
{
    pub fn new(left: L, right: R, left_key: LK, right_keys: RK, exists: bool) -> Self {
        Self {
            left,
            right,
            left_key,
            right_keys,
            keys: KeyState::new(),
            touched: Vec::new(),
            right_stored: HandleMap::new(),
            left_keys: HandleMap::new(),
            accepted: HandleMap::new(),
            retired: Vec::new(),
            key_scratch: Vec::new(),
            left_removed: Vec::new(),
            left_inserted: Vec::new(),
            right_removed: Vec::new(),
            right_inserted: Vec::new(),
            removed: Vec::new(),
            inserted: Vec::new(),
            flip_ins: Vec::new(),
            flip_rem: Vec::new(),
            exists,
            marker: PhantomData,
        }
    }
    fn notify(
        &mut self,
        solution: &S,
        descriptor: usize,
        index: usize,
        retract: bool,
        removed: &mut Vec<RowHandle>,
        inserted: &mut Vec<RowHandle>,
    ) {
        let mut left_removed = std::mem::take(&mut self.left_removed);
        let mut left_inserted = std::mem::take(&mut self.left_inserted);
        left_removed.clear();
        left_inserted.clear();
        let mut right_removed = std::mem::take(&mut self.right_removed);
        let mut right_inserted = std::mem::take(&mut self.right_inserted);
        right_removed.clear();
        right_inserted.clear();
        if retract {
            self.left.retract(solution, descriptor, index, &mut left_removed, &mut left_inserted);
            self.right.retract(solution, descriptor, index, &mut right_removed, &mut right_inserted);
        } else {
            self.left.insert(solution, descriptor, index, &mut left_removed, &mut left_inserted);
            self.right.insert(solution, descriptor, index, &mut right_removed, &mut right_inserted);
        }
        self.apply_changes(
            solution,
            PairDelta {
                left_removed: &left_removed,
                left_inserted: &left_inserted,
                right_removed: &right_removed,
                right_inserted: &right_inserted,
            },
            removed,
            inserted,
        );
        self.left_removed = left_removed;
        self.left_inserted = left_inserted;
        self.right_removed = right_removed;
        self.right_inserted = right_inserted;
    }
    /// Re-evaluate one touched key's left rows against the key's current
    /// membership. Idempotent per key, so an event may list a key once.
    /// Reads the bucket through a raw borrow and copies only the handles
    /// that flip, so an unchanged key's bucket is never copied.
    /// Applies one key's membership deltas to `self.accepted` and appends the
    /// flipped handles to the polarity scratch (`flip_ins` / `flip_rem`).
    fn settle_key(&mut self, key: &K) {
        let should = self.keys.alive(key) == self.exists;
        let bucket: &[RowHandle] = self.keys.left_bucket(key).unwrap_or(&[]);
        for &h in bucket.iter() {
            let settled = self.accepted.get(h).is_some() == should;
            if !settled {
                if should {
                    self.accepted.insert(h, ());
                    self.flip_ins.push(h);
                } else {
                    self.accepted.remove(h);
                    self.flip_rem.push(h);
                }
            }
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
        // Retract left rows first so the membership pass only touches live rows.
        for h in left_removed.iter().copied() {
            if let Some(key) = self.left_keys.remove(h) {
                self.keys.left_remove(&key, h);
            }
            if self.accepted.remove(h).is_some() {
                removed.push(h);
            }
        }
        self.keys.begin_event();
        for h in right_removed.iter().copied() {
            if let Some(mut keys) = self.right_stored.remove(h) {
                for k in keys.drain(..) {
                    self.keys.take(&k, 1);
                }
                keys.clear();
                self.retired.push(keys);
            }
        }
        for h in right_inserted.iter().copied() {
            let Some(row) = self.right.resolve(solution, h) else {
                continue;
            };
            let mut slot = self.retired.pop().unwrap_or_default();
            slot.clear();
            self.right_keys.push_keys(&row, &mut slot);
            for k in slot.iter() {
                self.keys.add(k, 1);
            }
            self.right_stored.insert(h, slot);
        }
        // One membership pass per touched key: counts are final here, so a
        // key removed and re-added inside one event settles on its final
        // state with no set bookkeeping.
        let mut touched = std::mem::take(&mut self.touched);
        touched.clear();
        self.keys.drain_touched(&mut touched);
        self.flip_ins.clear();
        self.flip_rem.clear();
        for k in touched.drain(..) {
            self.settle_key(&k);
        }
        self.touched = touched;
        removed.extend_from_slice(&self.flip_rem);
        inserted.extend_from_slice(&self.flip_ins);
        for h in left_inserted.iter().copied() {
            let Some(row) = self.left.resolve(solution, h) else {
                continue;
            };
            let key = self.left_key.key(&row);
            self.keys.left_insert(&key, h);
            self.left_keys.insert(h, key.clone());
            if self.keys.alive(&key) == self.exists {
                self.accepted.insert(h, ());
                inserted.push(h);
            }
        }
    }
}
impl<S: 'static, L: Operator<S>, R: Operator<S>, K, LK, RK> Operator<S>
    for ExistenceNode<S, L, R, K, LK, RK>
where
    K: Eq + Hash + Clone + 'static,
    LK: Send + Sync + 'static,
    RK: Send + Sync + 'static,
    for<'a> LK: ViewKey<L::View<'a>, Key = K>,
    for<'a> RK: RightKeySet<R::View<'a>, Key = K>,
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
        let mut counts: FastMap<K, usize> = FastMap::default();
        let mut scratch = Vec::new();
        let right_keys = &self.right_keys;
        self.right
            .visit_evaluation(solution, &evaluation.1, &mut |row| {
                scratch.clear();
                right_keys.push_keys(&row, &mut scratch);
                for k in scratch.iter() {
                    *counts.entry(k.clone()).or_insert(0) += 1;
                }
            });
        let left_key = &self.left_key;
        self.left
            .visit_evaluation(solution, &evaluation.0, &mut |left| {
                if counts.contains_key(&left_key.key(&left)) == self.exists {
                    visitor(left);
                }
            });
    }
    fn clear(&mut self) {
        self.left.clear();
        self.right.clear();
        self.keys.clear();
        self.touched.clear();
        self.right_stored.clear();
        self.left_keys.clear();
        self.accepted.clear();
        self.retired.clear();
        self.key_scratch.clear();
        self.flip_ins.clear();
        self.flip_rem.clear();
        self.left_removed.clear();
        self.left_inserted.clear();
        self.right_removed.clear();
        self.right_inserted.clear();
        self.removed.clear();
        self.inserted.clear();
    }
    fn initialize(&mut self, solution: &S) {
        self.clear();
        self.left.initialize(solution);
        self.right.initialize(solution);
        let left_inserted = self.left.handles();
        let right_inserted = self.right.handles();
        let mut sink_removed = Vec::new();
        let mut sink_inserted = Vec::new();
        self.apply_changes(
            solution,
            PairDelta {
                left_removed: &[],
                left_inserted: &left_inserted,
                right_removed: &[],
                right_inserted: &right_inserted,
            },
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
        self.notify(solution, descriptor, index, true, removed, inserted);
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
        self.notify(solution, descriptor, index, false, removed, inserted);
    }
}
