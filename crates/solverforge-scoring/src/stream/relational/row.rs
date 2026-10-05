/* Borrowed leaf rows for relational operators.

A leaf borrows one source entity with its semantic source slice index —
no entity cloning, no stored solution borrows between callbacks. Leaf
rows resolve against the current solution during traversal; derived
operators borrow their own retained outputs instead. Recursive
concatenation for chained joins arrives with the chaining operator.
*/

/* A borrowed view of one source entity.

The lifetime is the traversal borrow, not a stored solution reference.
`index` carries the semantic source slice index alongside the entity so
low-level filters keep exact index semantics without storage-ID leakage.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Leaf<'r, T> {
    pub entity: &'r T,
    pub index: usize,
}

impl<'r, T> Leaf<'r, T> {
    pub(crate) fn new(entity: &'r T, index: usize) -> Leaf<'r, T> {
        Leaf { entity, index }
    }
}
