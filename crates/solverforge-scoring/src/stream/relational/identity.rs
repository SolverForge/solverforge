/* Stable row identities for relational operators.

A `RowHandle` is a generational index into a dense store: the slot may be
reused after retraction, but the generation bump makes stale handles
unresolvable. Composed joined/branch/binding identities arrive with the P2
operators that consume them.
*/

/* Generational handle into a `DenseRowStore`.

Handles are `Copy` so operator reverse links can duplicate them without
touching payloads. Handles are store-relative, like `usize` into a
specific `Vec`: only the minting store resolves them, and a generation
bump retires them within that store.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RowHandle {
    slot: u32,
    generation: u32,
}

impl RowHandle {
    pub(super) fn new(slot: u32, generation: u32) -> RowHandle {
        RowHandle { slot, generation }
    }

    pub(super) fn slot(self) -> u32 {
        self.slot
    }

    pub(super) fn generation(self) -> u32 {
        self.generation
    }
}
