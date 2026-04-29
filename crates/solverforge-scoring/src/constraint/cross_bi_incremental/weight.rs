pub trait CrossBiWeight<S, A, B, Sc>: Send + Sync {
    fn score(
        &self,
        solution: &S,
        entities_a: &[A],
        entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc;
}

pub struct IndexWeight<W>(W);

impl<W> IndexWeight<W> {
    #[inline]
    pub(crate) fn new(weight: W) -> Self {
        Self(weight)
    }
}

impl<S, A, B, W, Sc> CrossBiWeight<S, A, B, Sc> for IndexWeight<W>
where
    W: Fn(&S, usize, usize) -> Sc + Send + Sync,
{
    #[inline]
    fn score(
        &self,
        solution: &S,
        _entities_a: &[A],
        _entities_b: &[B],
        a_idx: usize,
        b_idx: usize,
    ) -> Sc {
        (self.0)(solution, a_idx, b_idx)
    }
}

pub struct PairWeight<W>(W);

impl<W> PairWeight<W> {
    #[inline]
    pub(crate) fn new(weight: W) -> Self {
        Self(weight)
    }
}

impl<S, A, B, W, Sc> CrossBiWeight<S, A, B, Sc> for PairWeight<W>
where
    W: Fn(&A, &B) -> Sc + Send + Sync,
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
        (self.0)(&entities_a[a_idx], &entities_b[b_idx])
    }
}
