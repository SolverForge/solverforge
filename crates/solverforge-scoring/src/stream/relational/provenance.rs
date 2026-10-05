/* Authored binding provenance for joined rows.

Every participating binding is recorded, including a repeated binding of
one entity (self-joins, one descriptor in several bindings). Explanations
and tuple orientation use this; nothing is deduplicated. Physical
dependency traversal (deduplicated invalidation sets) arrives with the
root update router that consumes it.
*/

use super::identity::{BindingId, RowHandle};

/* One authored participation in a joined row.

`binding` is the authored position (0, 1, 2, ...), not a descriptor or
slot. `handle` names the input row; `descriptor`/`index` name the
physical entity for invalidation. A repeated entity yields one entry per
binding with the same physical pair.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Participation {
    pub binding: BindingId,
    pub handle: RowHandle,
    pub descriptor: usize,
    pub index: usize,
}

impl Participation {
    pub(crate) fn new(
        binding: BindingId,
        handle: RowHandle,
        descriptor: usize,
        index: usize,
    ) -> Participation {
        Participation {
            binding,
            handle,
            descriptor,
            index,
        }
    }
}

/* Authored provenance of one output row: every binding in order.

Construction pushes left-spine participations first, so iteration order
is authored tuple orientation. No deduplication, ever.
*/
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Provenance {
    participations: Vec<Participation>,
}

impl Provenance {
    pub(crate) fn new() -> Provenance {
        Provenance {
            participations: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, participation: Participation) {
        self.participations.push(participation);
    }

    pub(crate) fn bindings(&self) -> &[Participation] {
        &self.participations
    }
}
