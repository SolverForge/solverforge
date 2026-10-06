/* Stable row identities for relational operators.

A `RowHandle` is a generational index into a dense store: the slot may be
reused after retraction, but the generation bump makes stale handles
unresolvable. `BindingId` names authored source positions; `JoinedIdentity`
composes input handles plus an emission discriminator into output identity.
*/

/* Generational handle into a `DenseRowStore`.

Handles are `Copy` so operator reverse links can duplicate them without
touching payloads. Handles are store-relative, like `usize` into a
specific `Vec`: only the minting store resolves them, and a generation
bump retires them within that store.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RowHandle {
    slot: u32,
    generation: u32,
}

impl RowHandle {
    pub(super) fn new(slot: u32, generation: u32) -> RowHandle {
        RowHandle { slot, generation }
    }

    /* Slot index for dense engine-side bookkeeping arrays. */
    pub(crate) fn slot(self) -> u32 {
        self.slot
    }

    /* Generation check for slot-indexed records under slot reuse. */
    pub(crate) fn generation(self) -> u32 {
        self.generation
    }
}

/* Authored binding identity: which source binding a row belongs to.

Distinct from descriptor identity, source slice index, and storage slot.
A single descriptor occurring in several bindings yields several ids.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct BindingId(pub u32);

/* Composed identity of one joined output row.

Determined by the left/right input row handles plus an emission
discriminator for operators that emit several outputs per input pair
(projections, flattening). The discriminator keeps sibling emissions
distinct without fabricating source entity indexes.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct JoinedIdentity {
    left: RowHandle,
    right: RowHandle,
    emission: u32,
}

impl JoinedIdentity {
    pub(crate) fn new(left: RowHandle, right: RowHandle, emission: u32) -> JoinedIdentity {
        JoinedIdentity {
            left,
            right,
            emission,
        }
    }

    pub(crate) fn left(self) -> RowHandle {
        self.left
    }

    pub(crate) fn right(self) -> RowHandle {
        self.right
    }
}
