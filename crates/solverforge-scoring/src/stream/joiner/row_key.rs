/* Borrowed-row invocation adapters shared across joiner families.

Row-aware conditions take whole borrowed rows (`&Concat<Leaf<A>, B>`)
instead of single entities. These adapters build that row from live
entity borrows at operator, terminal, and stateless-evaluation sites,
so every family assembles the identical shape. Binding selection stays
positional — the adapters never infer which earlier entity to select
from its Rust type, so multiple same-typed bindings remain distinct.
*/

use super::super::relational::{Concat, Leaf};

/* Assembles the borrowed pair row for one live (A, B) combination.

The row borrows both entities with their semantic source indexes; no
entity is cloned and no solution borrow is retained.
*/
#[inline]
pub fn pair_row<'r, A, B>(
    a: &'r A,
    a_idx: usize,
    b: &'r B,
    b_idx: usize,
) -> Concat<'r, Leaf<'r, A>, B> {
    Concat::new(Leaf::new(a, a_idx), b, b_idx)
}
