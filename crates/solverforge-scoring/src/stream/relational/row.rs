/* Borrowed row shapes for relational operators.

A row is either a leaf borrowing one entity or a concatenation of a prior
row with a new entity reference. Concatenation is structural — no arity
ceiling, no erased values — so a later join's key closure can inspect any
earlier binding. Rows borrow; they never own entities and never hold
solution borrows between callbacks.
*/

/* A borrowed view of one source entity.

The lifetime is the traversal borrow, not a stored solution reference.
`index` carries the semantic source slice index alongside the entity so
low-level filters keep exact index semantics without storage-ID leakage.
*/
#[derive(Debug, PartialEq, Eq)]
pub struct Leaf<'r, T> {
    pub entity: &'r T,
    pub index: usize,
}

impl<'r, T> Leaf<'r, T> {
    pub fn new(entity: &'r T, index: usize) -> Leaf<'r, T> {
        Leaf { entity, index }
    }
}

impl<T> Copy for Leaf<'_, T> {}
impl<T> Clone for Leaf<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

/* Recursive concatenation of a prior row with a new entity reference.

The chained join's left key closure receives the whole prior row in this
shape, so it can inspect any earlier binding or combine several. Binding
order walks the left spine first, matching authored tuple orientation.
*/
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Concat<'r, Left, T> {
    pub left: Left,
    pub right: Leaf<'r, T>,
}

impl<'r, Left: Copy, T: Copy> Copy for Concat<'r, Left, T> {}

impl<'r, Left, T> Concat<'r, Left, T> {
    pub fn new(left: Left, entity: &'r T, index: usize) -> Concat<'r, Left, T> {
        Concat {
            left,
            right: Leaf::new(entity, index),
        }
    }
}

/* Row depth as a compile-time property of the nesting.

Depth keeps operator recursion monomorphized without runtime tags.
`DEPTH` is read by row-shape assertions at operator construction sites.
*/
#[allow(dead_code)]
pub trait Row {
    // Number of bound entities in this row (leaf = 1).
    const DEPTH: usize;
}

impl<'r, T> Row for Leaf<'r, T> {
    const DEPTH: usize = 1;
}

impl<'r, Left: Row, T> Row for Concat<'r, Left, T> {
    const DEPTH: usize = Left::DEPTH + 1;
}
