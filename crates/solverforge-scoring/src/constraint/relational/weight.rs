/* Terminal weight over borrowed row pairs.

`RelationalWeight` scores one terminal pair from the solution and the two
entity slices. Plain `Fn(&A, &B) -> Sc` closures implement it directly so
call sites pass weight functions without wrappers; richer adapters
(hard-medium-soft, dynamic) arrive with the fluent weighting ergonomics.
Impact sign (penalty negates, reward keeps) applies in the terminal, not
here, so weights stay direction-agnostic.
*/

use solverforge_core::score::Score;

pub trait RelationalWeight<S, A, B, Sc>: Send + Sync {
    fn score(
        &self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc;
}

impl<S, A, B, F, Sc> RelationalWeight<S, A, B, Sc> for F
where
    F: Fn(&A, &B) -> Sc + Send + Sync,
    Sc: Score,
{
    #[inline]
    fn score(
        &self,
        _solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        (self)(&entities_a[a_idx], &entities_b[b_idx])
    }
}

/* Terminal weight over borrowed row triples.

Scores one terminal triple from the solution and the three entity slices.
Plain `Fn(&A, &B, &C) -> Sc` closures implement it directly; the impact
sign applies in the terminal, never here.
*/
pub trait RelationalWeight3<S, A, B, C, Sc>: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn score(
        &self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_c: &[C],
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
    ) -> Sc;
}

impl<S, A, B, C, F, Sc> RelationalWeight3<S, A, B, C, Sc> for F
where
    F: Fn(&A, &B, &C) -> Sc + Send + Sync,
    Sc: Score,
{
    #[inline]
    fn score(
        &self,
        _solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        entities_c: &[C],
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
    ) -> Sc {
        (self)(&entities_a[a_idx], &entities_b[b_idx], &entities_c[c_idx])
    }
}
