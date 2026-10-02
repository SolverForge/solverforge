pub trait CrossTriWeight<S, A, B, C, Sc>: Send + Sync {
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

pub struct IndexWeight<W>(W);

impl<W> IndexWeight<W> {
    #[inline]
    pub(crate) fn new(weight: W) -> Self {
        Self(weight)
    }
}

impl<S, A, B, C, W, Sc> CrossTriWeight<S, A, B, C, Sc> for IndexWeight<W>
where
    W: Fn(&S, usize, usize, usize) -> Sc + Send + Sync,
{
    #[inline]
    fn score(
        &self,
        solution: &S,
        _entities_a: &[A],
        _entities_b: &[B],
        _entities_c: &[C],
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
    ) -> Sc {
        (self.0)(solution, a_idx, b_idx, c_idx)
    }
}

pub struct TripleWeight<W>(W);

impl<W> TripleWeight<W> {
    #[inline]
    pub(crate) fn new(weight: W) -> Self {
        Self(weight)
    }
}

impl<S, A, B, C, W, Sc> CrossTriWeight<S, A, B, C, Sc> for TripleWeight<W>
where
    W: Fn(&A, &B, &C) -> Sc + Send + Sync,
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
        (self.0)(&entities_a[a_idx], &entities_b[b_idx], &entities_c[c_idx])
    }
}
