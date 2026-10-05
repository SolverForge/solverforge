/* Change transactions: one root update, exactly-once output delivery.

One `on_retract` or `on_insert` notification is one root update with an
internal epoch. The update routes to every matching leaf binding —
including repeated bindings of one descriptor and both branches of a
self-join — while duplicate output paths coalesce so each changed output
publishes once. A single notification never applies a self-join match
twice and never exposes a transient partially updated group as final.

Retraction uses retained old keys, terminal scores, projection
identities, and collector tokens. After mutation, insertion derives new
keys from new values. Signed deltas accumulate as the director does
(`retract_delta + insert_delta`); the prose formula elsewhere is stale.
*/

use std::collections::HashSet;

use super::JoinedIdentity;

/* One published output change: which joined row, in which direction.

`insert == true` publishes a new output row; `false` retracts a retained
one. Consumers apply retractions before insertions for the same epoch so
a changed value never double-counts.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct OutputDelta {
    pub row: JoinedIdentity,
    pub insert: bool,
}

impl OutputDelta {
    pub(crate) fn insert(row: JoinedIdentity) -> OutputDelta {
        OutputDelta { row, insert: true }
    }

    pub(crate) fn retract(row: JoinedIdentity) -> OutputDelta {
        OutputDelta { row, insert: false }
    }
}

/* Epoch-scoped coalescing buffer for output deltas.

`begin` opens a new epoch; `push` records a delta unless its row already
published in this epoch; `drain_ordered` yields retractions before
insertions with a deterministic row order. Payloads are never copied —
only identities travel here; scores/tokens stay in retained state.
*/
#[derive(Clone, Debug, Default)]
pub(crate) struct DeltaBuffer {
    seen: HashSet<JoinedIdentity>,
    pending: Vec<OutputDelta>,
}

impl DeltaBuffer {
    pub(crate) fn new() -> DeltaBuffer {
        DeltaBuffer {
            seen: HashSet::new(),
            pending: Vec::new(),
        }
    }

    pub(crate) fn begin(&mut self) {
        self.seen.clear();
        self.pending.clear();
    }

    // Returns true when the delta was newly recorded.
    pub(crate) fn push(&mut self, delta: OutputDelta) -> bool {
        if !self.seen.insert(delta.row) {
            return false;
        }
        self.pending.push(delta);
        true
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub(crate) fn drain_ordered(&mut self) -> Vec<OutputDelta> {
        let mut deltas = std::mem::take(&mut self.pending);
        self.seen.clear();
        // Retractions first, then a deterministic row order within each
        // direction. JoinedIdentity is not Ord (handles are opaque), so
        // sort by the stable (left, right, emission) triple explicitly.
        deltas.sort_by(|a, b| {
            a.insert
                .cmp(&b.insert)
                .then(a.row.left().slot().cmp(&b.row.left().slot()))
                .then(a.row.left().generation().cmp(&b.row.left().generation()))
                .then(a.row.right().slot().cmp(&b.row.right().slot()))
                .then(a.row.right().generation().cmp(&b.row.right().generation()))
                .then(a.row.emission().cmp(&b.row.emission()))
        });
        deltas
    }
}
