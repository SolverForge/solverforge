/* Canonical cross-tri-constraint stream for three-source join patterns.

A `Tri` is the operator tree `(A ⋈ B) ⋈ C`: the first relationship keeps
the Bi stream's typed keys, the second is compiled fresh from its joiner
so the left closure receives the whole borrowed (A, B) row and the key
domain stays independent. One uniform `.join()` on the Bi stream builds
it — no per-arity engine, no shared-key retention at any depth.
*/

mod base;

pub(crate) use base::assemble_tri;
pub use base::Tri;
