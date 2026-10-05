/* Binary equi-join operator over two retained sources.

Each join owns its independent typed equality key `K`: the left and right
key closures map their own entity into that one domain, and the hash
indexes on both sides stay private to this join. Heterogeneous successive
joins keep separate `K` types because each `EquiJoin` is its own type.

Retained outputs live in a `DenseRowStore` behind stable handles with
per-side reverse links, so retraction never scans. Root notifications
open a `DeltaBuffer` epoch: duplicate output paths coalesce and each
changed output publishes once. Provenance records both bindings with
their descriptors for explanation and invalidation. A `pair_of` map keeps
the semantic index pair per output identity so drained deltas translate
to operator-boundary `JoinDelta`s even after the output row is gone.
*/

use std::collections::HashMap;
use std::hash::Hash;

use super::super::collection_extract::{ChangeSource, CollectionExtract};
use super::{
    BindingId, DeltaBuffer, DenseRowStore, JoinedIdentity, OutputDelta, Participation, Provenance,
    RowHandle, Source,
};

/* Operator-boundary delta: which output pair changed, in which direction.

Indexes are semantic source slice indexes, not storage slots, so tests
and terminals read them without resolving handles.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeltaKind {
    Insert,
    Retract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct JoinDelta {
    pub(crate) kind: DeltaKind,
    pub(crate) left_idx: usize,
    pub(crate) right_idx: usize,
}

struct Output {
    left_idx: usize,
    right_idx: usize,
    identity: JoinedIdentity,
    provenance: Provenance,
}

/* Binary equi-join with its own key domain and retained output rows. */
pub struct EquiJoin<S, A, B, EA, EB, K, KA, KB, F> {
    left: Source<S, A, EA, K, KA>,
    right: Source<S, B, EB, K, KB>,
    left_descriptor: usize,
    right_descriptor: usize,
    filter: F,
    outputs: DenseRowStore<Output>,
    output_of: HashMap<(usize, usize), RowHandle>,
    pair_of: HashMap<JoinedIdentity, (usize, usize)>,
    to_outputs: [HashMap<usize, Vec<RowHandle>>; 2],
    deltas: DeltaBuffer,
    name: String,
}

impl<S, A, B, EA, EB, K, KA, KB, F> EquiJoin<S, A, B, EA, EB, K, KA, KB, F>
where
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    K: Eq + Hash + Clone,
    KA: Fn(&A) -> K,
    KB: Fn(&B) -> K,
    F: Fn(&S, &A, &B, usize, usize) -> bool,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        left_extractor: EA,
        right_extractor: EB,
        left_key: KA,
        right_key: KB,
        filter: F,
        left_descriptor: usize,
        right_descriptor: usize,
        name: &str,
    ) -> EquiJoin<S, A, B, EA, EB, K, KA, KB, F> {
        EquiJoin {
            left: Source::new(
                left_extractor,
                left_key,
                BindingId(0),
                ChangeSource::Descriptor(left_descriptor),
            ),
            right: Source::new(
                right_extractor,
                right_key,
                BindingId(1),
                ChangeSource::Descriptor(right_descriptor),
            ),
            left_descriptor,
            right_descriptor,
            filter,
            outputs: DenseRowStore::new(),
            output_of: HashMap::new(),
            pair_of: HashMap::new(),
            to_outputs: [HashMap::new(), HashMap::new()],
            deltas: DeltaBuffer::new(),
            name: name.to_string(),
        }
    }

    /* Builds all outputs from the current solution. */
    pub(crate) fn refresh(&mut self, solution: &S) {
        self.left.refresh(solution);
        self.right.refresh(solution);
        self.outputs.clear();
        self.output_of.clear();
        self.pair_of.clear();
        for buckets in self.to_outputs.iter_mut() {
            buckets.clear();
        }
        self.deltas.begin();
        let count = self.left_len(solution);
        for a_idx in 0..count {
            self.probe_left(solution, a_idx);
        }
        // Refresh builds silently: probed inserts stay unpublished.
        self.deltas.begin();
    }

    fn left_len(&self, solution: &S) -> usize {
        self.left.extractor_ref().extract(solution).len()
    }

    fn probe_left(&mut self, solution: &S, a_idx: usize) {
        let Some(a_handle) = self.left.handle_for(a_idx) else {
            return;
        };
        let Some(key) = self.left.key_at(solution, a_idx) else {
            return;
        };
        let partners: Vec<(usize, RowHandle)> = self
            .right
            .lookup(&key)
            .iter()
            .filter_map(|handle| self.right.index_of(*handle).map(|b_idx| (b_idx, *handle)))
            .collect();
        for (b_idx, b_handle) in partners {
            self.emit_if_match(solution, a_idx, a_handle, b_idx, b_handle);
        }
    }

    fn probe_right(&mut self, solution: &S, b_idx: usize) {
        let Some(b_handle) = self.right.handle_for(b_idx) else {
            return;
        };
        let Some(key) = self.right.key_at(solution, b_idx) else {
            return;
        };
        let partners: Vec<(usize, RowHandle)> = self
            .left
            .lookup(&key)
            .iter()
            .filter_map(|handle| self.left.index_of(*handle).map(|a_idx| (a_idx, *handle)))
            .collect();
        for (a_idx, a_handle) in partners {
            self.emit_if_match(solution, a_idx, a_handle, b_idx, b_handle);
        }
    }

    fn emit_if_match(
        &mut self,
        solution: &S,
        a_idx: usize,
        a_handle: RowHandle,
        b_idx: usize,
        b_handle: RowHandle,
    ) {
        if self.output_of.contains_key(&(a_idx, b_idx)) {
            return;
        }
        let entities_a = self.left.extractor_ref().extract(solution);
        let entities_b = self.right.extractor_ref().extract(solution);
        let (Some(a), Some(b)) = (entities_a.get(a_idx), entities_b.get(b_idx)) else {
            return;
        };
        if !(self.filter)(solution, a, b, a_idx, b_idx) {
            return;
        }
        let identity = JoinedIdentity::new(a_handle, b_handle, 0);
        let mut provenance = Provenance::new();
        provenance.push(Participation::new(
            self.left.binding(),
            a_handle,
            self.left_descriptor,
            a_idx,
        ));
        provenance.push(Participation::new(
            self.right.binding(),
            b_handle,
            self.right_descriptor,
            b_idx,
        ));
        let handle = self.outputs.insert(Output {
            left_idx: a_idx,
            right_idx: b_idx,
            identity,
            provenance,
        });
        self.output_of.insert((a_idx, b_idx), handle);
        self.pair_of.insert(identity, (a_idx, b_idx));
        self.to_outputs[0].entry(a_idx).or_default().push(handle);
        self.to_outputs[1].entry(b_idx).or_default().push(handle);
        self.deltas.push(OutputDelta::insert(identity));
    }

    /* Borrowed extractors for full evaluation without retained state. */
    pub(crate) fn left_extractor(&self) -> &EA {
        self.left.extractor_ref()
    }

    pub(crate) fn right_extractor(&self) -> &EB {
        self.right.extractor_ref()
    }

    /* Borrowed filter for full evaluation without retained state. */
    pub(crate) fn filter(&self) -> &F {
        &self.filter
    }

    /* Borrowed key closures for stateless key re-derivation. */
    pub(crate) fn left_key(&self) -> &KA {
        self.left.key_ref()
    }

    pub(crate) fn right_key(&self) -> &KB {
        self.right.key_ref()
    }

    /* Drops all retained outputs and deltas without touching sources. */
    pub(crate) fn clear_outputs(&mut self) {
        self.outputs.clear();
        self.output_of.clear();
        self.pair_of.clear();
        for buckets in self.to_outputs.iter_mut() {
            buckets.clear();
        }
        self.deltas.begin();
    }

    /* Stable handle of one live output pair, if still retained. */
    pub(crate) fn output_handle(&self, a_idx: usize, b_idx: usize) -> Option<RowHandle> {
        self.output_of.get(&(a_idx, b_idx)).copied()
    }

    /* Retained provenance for one output pair, if still live. */
    pub(crate) fn provenance_of(&self, a_idx: usize, b_idx: usize) -> Option<&Provenance> {
        let handle = self.output_of.get(&(a_idx, b_idx))?;
        self.outputs.get(*handle).map(|out| &out.provenance)
    }

    /* Current output pairs as semantic (left, right) indexes. */
    pub(crate) fn output_rows(&self) -> Vec<(usize, usize)> {
        let mut rows: Vec<(usize, usize)> = self
            .outputs
            .iter()
            .map(|(_, out)| (out.left_idx, out.right_idx))
            .collect();
        rows.sort();
        rows
    }

    /* Retracts one source entity's outputs, publishing each exactly once. */
    pub(crate) fn on_retract_source(&mut self, descriptor: usize, idx: usize) -> Vec<JoinDelta> {
        self.deltas.begin();
        let side = if descriptor == self.left_descriptor {
            if !self.left.assert_localizes(descriptor, &self.name) {
                return Vec::new();
            }
            0
        } else if descriptor == self.right_descriptor {
            if !self.right.assert_localizes(descriptor, &self.name) {
                return Vec::new();
            }
            1
        } else {
            return Vec::new();
        };
        let handles: Vec<RowHandle> = self.to_outputs[side].remove(&idx).unwrap_or_default();
        for handle in handles {
            if let Some(out) = self.outputs.retract(handle) {
                self.output_of.remove(&(out.left_idx, out.right_idx));
                self.unlink(out.left_idx, out.right_idx, handle);
                self.deltas.push(OutputDelta::retract(out.identity));
            }
        }
        self.forget_source_handle(side, idx);
        self.drain_published()
    }

    /* Inserts one source entity's new outputs, publishing each exactly once.

    Mints only the notified entity's handle: every other handle stays
    stable, so retained outputs keep valid identities. A changed key
    arrives as retract (old key, retained) + insert (new key, derived).
    */
    pub(crate) fn on_insert_source(
        &mut self,
        solution: &S,
        descriptor: usize,
        idx: usize,
    ) -> Vec<JoinDelta> {
        self.deltas.begin();
        if descriptor == self.left_descriptor {
            if !self.left.assert_localizes(descriptor, &self.name) {
                return Vec::new();
            }
            if self.left.insert_idx(solution, idx).is_some() {
                self.probe_left(solution, idx);
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

    fn unlink(&mut self, a_idx: usize, b_idx: usize, handle: RowHandle) {
        for (side, idx) in [(0, a_idx), (1, b_idx)] {
            if let Some(bucket) = self.to_outputs[side].get_mut(&idx) {
                if let Some(pos) = bucket.iter().position(|h| *h == handle) {
                    bucket.swap_remove(pos);
                }
                if bucket.is_empty() {
                    self.to_outputs[side].remove(&idx);
                }
            }
        }
    }

    fn forget_source_handle(&mut self, side: usize, idx: usize) {
        if side == 0 {
            self.left.forget(idx);
        } else {
            self.right.forget(idx);
        }
    }

    fn drain_published(&mut self) -> Vec<JoinDelta> {
        if self.deltas.is_empty() {
            return Vec::new();
        }
        let mut published = Vec::new();
        for delta in self.deltas.drain_ordered() {
            if let Some(pair) = self.pair_of.remove(&delta.row) {
                // Retracted outputs never publish twice: the pair entry is
                // gone, so a duplicate path finds nothing.
                published.push(JoinDelta {
                    kind: if delta.insert {
                        DeltaKind::Insert
                    } else {
                        DeltaKind::Retract
                    },
                    left_idx: pair.0,
                    right_idx: pair.1,
                });
            } else if delta.insert {
                // Fresh insert whose pair entry was consumed by an earlier
                // duplicate path in the same epoch: already published.
            }
            // A retract without a pair entry means the output was never
            // emitted (filtered out); nothing to publish.
        }
        published
    }
}
