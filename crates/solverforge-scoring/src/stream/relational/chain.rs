/* Chained equi-join: a retained binary join's outputs joined to a new source.

The second relationship owns its own typed key `K2`, independent of the
first join's key type: heterogeneous successive keys stay in separate
domains because each operator is its own type. The left key closure
receives the whole left row (`&Concat<Leaf<A>, B>`), so it can inspect
any earlier binding or combine several — the capability the shared-key
chained API could never express.

The chained operator owns the first join by value (composition, not
borrowing), so retention lifetimes stay flat. Left-side notifications
delegate to the first join and cascade per retracted/inserted pair;
right-side notifications mirror the binary operator's targeted paths.
Output provenance extends the left pair's retained participations with
the new right binding, preserving authored order and descriptors.
*/

use std::collections::HashMap;
use std::hash::Hash;

use super::super::collection_extract::{ChangeSource, CollectionExtract};
use super::join::DeltaKind;
use super::{
    BindingId, Concat, DeltaBuffer, DenseRowStore, EquiJoin, JoinedIdentity, Leaf, OutputDelta,
    Participation, Provenance, RowHandle, Source,
};

/* Chained output delta with full triple indexes. */
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChainedDelta {
    pub(crate) kind: DeltaKind,
    pub(crate) a_idx: usize,
    pub(crate) b_idx: usize,
    pub(crate) c_idx: usize,
}

struct ChainedOutput {
    a_idx: usize,
    b_idx: usize,
    c_idx: usize,
    identity: JoinedIdentity,
    provenance: Provenance,
}

/* Second equi-join over a retained first join plus a new right source. */
pub struct ChainedJoin<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2> {
    first: EquiJoin<S, A, B, EA, EB, K1, KA, KB, F1>,
    right: Source<S, C, EC, K2, KC>,
    left_key: LK,
    filter: F2,
    left_a_descriptor: usize,
    left_b_descriptor: usize,
    right_descriptor: usize,
    outputs: DenseRowStore<ChainedOutput>,
    output_of: HashMap<(usize, usize, usize), RowHandle>,
    pair_of: HashMap<JoinedIdentity, (usize, usize, usize)>,
    to_outputs_left: HashMap<(usize, usize), Vec<RowHandle>>,
    to_outputs_right: HashMap<usize, Vec<RowHandle>>,
    deltas: DeltaBuffer,
    name: String,
}

impl<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2>
    ChainedJoin<S, A, B, C, EA, EB, EC, K1, KA, KB, K2, LK, KC, F1, F2>
where
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    EC: CollectionExtract<S, Item = C>,
    K1: Eq + Hash + Clone,
    KA: Fn(&A) -> K1,
    KB: Fn(&B) -> K1,
    F1: Fn(&S, &A, &B, usize, usize) -> bool,
    K2: Eq + Hash + Clone,
    LK: for<'r> Fn(&Concat<Leaf<'r, A>, B>) -> K2,
    KC: Fn(&C) -> K2,
    F2: Fn(&S, &A, &B, &C, usize, usize, usize) -> bool,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        first: EquiJoin<S, A, B, EA, EB, K1, KA, KB, F1>,
        right_extractor: EC,
        left_key: LK,
        right_key: KC,
        filter: F2,
        left_a_descriptor: usize,
        left_b_descriptor: usize,
        right_descriptor: usize,
        name: &str,
    ) -> Self {
        ChainedJoin {
            first,
            right: Source::new(
                right_extractor,
                right_key,
                BindingId(2),
                ChangeSource::Descriptor(right_descriptor),
            ),
            left_key,
            filter,
            left_a_descriptor,
            left_b_descriptor,
            right_descriptor,
            outputs: DenseRowStore::new(),
            output_of: HashMap::new(),
            pair_of: HashMap::new(),
            to_outputs_left: HashMap::new(),
            to_outputs_right: HashMap::new(),
            deltas: DeltaBuffer::new(),
            name: name.to_string(),
        }
    }

    /* Builds all chained outputs from the current solution. */
    pub(crate) fn refresh(&mut self, solution: &S) {
        self.first.refresh(solution);
        self.right.refresh(solution);
        self.outputs.clear();
        self.output_of.clear();
        self.pair_of.clear();
        self.to_outputs_left.clear();
        self.to_outputs_right.clear();
        self.deltas.begin();
        let pairs = self.first.output_rows();
        for (a_idx, b_idx) in pairs {
            self.probe_left(solution, a_idx, b_idx);
        }
        self.deltas.begin();
    }

    /* Borrowed accessors for terminals: extractors, filters, descriptors. */
    pub(crate) fn first(&self) -> &EquiJoin<S, A, B, EA, EB, K1, KA, KB, F1> {
        &self.first
    }

    pub(crate) fn right_extractor(&self) -> &EC {
        self.right.extractor_ref()
    }

    pub(crate) fn right_key(&self) -> &KC {
        self.right.key_ref()
    }

    pub(crate) fn filter(&self) -> &F2 {
        &self.filter
    }

    pub(crate) fn left_key(&self) -> &LK {
        &self.left_key
    }

    /* Current output triples as semantic indexes. */
    pub(crate) fn output_rows(&self) -> Vec<(usize, usize, usize)> {
        let mut rows: Vec<(usize, usize, usize)> = self
            .outputs
            .iter()
            .map(|(_, out)| (out.a_idx, out.b_idx, out.c_idx))
            .collect();
        rows.sort();
        rows
    }

    /* Retained provenance for one output triple, if still live. */
    pub(crate) fn provenance_of(
        &self,
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
    ) -> Option<&Provenance> {
        let handle = self.output_of.get(&(a_idx, b_idx, c_idx))?;
        self.outputs.get(*handle).map(|out| &out.provenance)
    }

    /* Drops all retained chained outputs and deltas. */
    pub(crate) fn clear_outputs(&mut self) {
        self.outputs.clear();
        self.output_of.clear();
        self.pair_of.clear();
        self.to_outputs_left.clear();
        self.to_outputs_right.clear();
        self.deltas.begin();
    }

    fn probe_left(&mut self, solution: &S, a_idx: usize, b_idx: usize) {
        let Some(left_handle) = self.first.output_handle(a_idx, b_idx) else {
            return;
        };
        let Some(key) = self.left_key_at(solution, a_idx, b_idx) else {
            return;
        };
        let partners: Vec<(usize, RowHandle)> = self
            .right
            .lookup(&key)
            .iter()
            .filter_map(|handle| self.right.index_of(*handle).map(|c_idx| (c_idx, *handle)))
            .collect();
        for (c_idx, c_handle) in partners {
            self.emit_if_match(solution, a_idx, b_idx, left_handle, c_idx, c_handle);
        }
    }

    fn probe_right(&mut self, solution: &S, c_idx: usize) {
        let Some(c_handle) = self.right.handle_for(c_idx) else {
            return;
        };
        let Some(key) = self.right.key_at(solution, c_idx) else {
            return;
        };
        // Scan live left pairs for key equality: the left side has no key
        // index of its own (its keys derive per-row on demand through the
        // row-aware closure).
        let pairs = self.first.output_rows();
        for (a_idx, b_idx) in pairs {
            let Some(left_handle) = self.first.output_handle(a_idx, b_idx) else {
                continue;
            };
            let Some(left_key) = self.left_key_at(solution, a_idx, b_idx) else {
                continue;
            };
            if left_key == key {
                self.emit_if_match(solution, a_idx, b_idx, left_handle, c_idx, c_handle);
            }
        }
    }

    /* Derives the left key for one live left pair through the row-aware closure. */
    fn left_key_at(&self, solution: &S, a_idx: usize, b_idx: usize) -> Option<K2> {
        let entities_a = self.first.left_extractor().extract(solution);
        let entities_b = self.first.right_extractor().extract(solution);
        let (Some(a), Some(b)) = (entities_a.get(a_idx), entities_b.get(b_idx)) else {
            return None;
        };
        let row = Concat::new(Leaf::new(a, a_idx), b, b_idx);
        Some((self.left_key)(&row))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_if_match(
        &mut self,
        solution: &S,
        a_idx: usize,
        b_idx: usize,
        left_handle: RowHandle,
        c_idx: usize,
        c_handle: RowHandle,
    ) {
        if self.output_of.contains_key(&(a_idx, b_idx, c_idx)) {
            return;
        }
        let entities_a = self.first.left_extractor().extract(solution);
        let entities_b = self.first.right_extractor().extract(solution);
        let entities_c = self.right.extractor_ref().extract(solution);
        let (Some(a), Some(b), Some(c)) = (
            entities_a.get(a_idx),
            entities_b.get(b_idx),
            entities_c.get(c_idx),
        ) else {
            return;
        };
        if !(self.filter)(solution, a, b, c, a_idx, b_idx, c_idx) {
            return;
        }
        let identity = JoinedIdentity::new(left_handle, c_handle, 0);
        // Provenance extends the left pair's retained participations with
        // the new right binding, preserving authored order.
        let mut provenance = match self.first.provenance_of(a_idx, b_idx) {
            Some(parent) => parent.clone(),
            None => return,
        };
        provenance.push(Participation::new(
            self.right.binding(),
            c_handle,
            self.right_descriptor,
            c_idx,
        ));
        let handle = self.outputs.insert(ChainedOutput {
            a_idx,
            b_idx,
            c_idx,
            identity,
            provenance,
        });
        self.output_of.insert((a_idx, b_idx, c_idx), handle);
        self.pair_of.insert(identity, (a_idx, b_idx, c_idx));
        self.to_outputs_left
            .entry((a_idx, b_idx))
            .or_default()
            .push(handle);
        self.to_outputs_right.entry(c_idx).or_default().push(handle);
        self.deltas.push(OutputDelta::insert(identity));
    }

    /* Retracts one source entity's chained outputs, publishing each once.

    Left-side notifications (either first-join descriptor) delegate to the
    first join and cascade per retracted pair; right-side notifications use
    the targeted path. Unknown descriptors are exact no-ops.
    */
    pub(crate) fn on_retract(&mut self, descriptor: usize, idx: usize) -> Vec<ChainedDelta> {
        self.deltas.begin();
        if descriptor == self.left_a_descriptor || descriptor == self.left_b_descriptor {
            let left_deltas = self.first.on_retract_source(descriptor, idx);
            for delta in left_deltas {
                self.retract_left_pair(delta.left_idx, delta.right_idx);
            }
        } else if descriptor == self.right_descriptor {
            if !self.right.assert_localizes(descriptor, &self.name) {
                return Vec::new();
            }
            let handles: Vec<RowHandle> = self.to_outputs_right.remove(&idx).unwrap_or_default();
            for handle in handles {
                self.retract_output(handle);
            }
            self.right.forget(idx);
        } else {
            return Vec::new();
        }
        self.drain_published()
    }

    /* Inserts one source entity's new chained outputs, publishing each once. */
    pub(crate) fn on_insert(
        &mut self,
        solution: &S,
        descriptor: usize,
        idx: usize,
    ) -> Vec<ChainedDelta> {
        self.deltas.begin();
        if descriptor == self.left_a_descriptor || descriptor == self.left_b_descriptor {
            for delta in self.first.on_insert_source(solution, descriptor, idx) {
                if delta.kind == DeltaKind::Insert {
                    self.probe_left(solution, delta.left_idx, delta.right_idx);
                }
            }
        } else if descriptor == self.right_descriptor {
            if !self.right.assert_localizes(descriptor, &self.name) {
                return Vec::new();
            }
            if self.right.insert_idx(solution, idx).is_some() {
                self.probe_right(solution, idx);
            }
        } else {
            return Vec::new();
        }
        self.drain_published()
    }

    fn retract_left_pair(&mut self, a_idx: usize, b_idx: usize) {
        let handles: Vec<RowHandle> = self
            .to_outputs_left
            .remove(&(a_idx, b_idx))
            .unwrap_or_default();
        for handle in handles {
            self.retract_output(handle);
        }
    }

    fn retract_output(&mut self, handle: RowHandle) {
        if let Some(out) = self.outputs.retract(handle) {
            self.output_of.remove(&(out.a_idx, out.b_idx, out.c_idx));
            self.unlink(out.a_idx, out.b_idx, out.c_idx, handle);
            self.deltas.push(OutputDelta::retract(out.identity));
        }
    }

    fn unlink(&mut self, a_idx: usize, b_idx: usize, c_idx: usize, handle: RowHandle) {
        if let Some(bucket) = self.to_outputs_left.get_mut(&(a_idx, b_idx)) {
            if let Some(pos) = bucket.iter().position(|h| *h == handle) {
                bucket.swap_remove(pos);
            }
            if bucket.is_empty() {
                self.to_outputs_left.remove(&(a_idx, b_idx));
            }
        }
        if let Some(bucket) = self.to_outputs_right.get_mut(&c_idx) {
            if let Some(pos) = bucket.iter().position(|h| *h == handle) {
                bucket.swap_remove(pos);
            }
            if bucket.is_empty() {
                self.to_outputs_right.remove(&c_idx);
            }
        }
    }

    fn drain_published(&mut self) -> Vec<ChainedDelta> {
        if self.deltas.is_empty() {
            return Vec::new();
        }
        let mut published = Vec::new();
        for delta in self.deltas.drain_ordered() {
            if let Some(triple) = self.pair_of.remove(&delta.row) {
                published.push(ChainedDelta {
                    kind: if delta.insert {
                        DeltaKind::Insert
                    } else {
                        DeltaKind::Retract
                    },
                    a_idx: triple.0,
                    b_idx: triple.1,
                    c_idx: triple.2,
                });
            }
        }
        published
    }
}
