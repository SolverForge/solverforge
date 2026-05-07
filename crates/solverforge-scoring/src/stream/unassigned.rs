use solverforge_core::score::Score;

use super::collection_extract::CollectionExtract;
use super::filter::{AndUniFilter, FnUniFilter, UniFilter};
use super::UniConstraintStream;

#[doc(hidden)]
pub trait UnassignedEntity<S>: Clone + Send + Sync + 'static {
    fn is_unassigned(solution: &S, entity: &Self) -> bool;
}

impl<S, A, E, F, Sc> UniConstraintStream<S, A, E, F, Sc>
where
    S: Send + Sync + 'static,
    A: UnassignedEntity<S>,
    E: CollectionExtract<S, Item = A>,
    F: UniFilter<S, A>,
    Sc: Score + 'static,
{
    pub fn unassigned(
        self,
    ) -> UniConstraintStream<S, A, E, AndUniFilter<F, FnUniFilter<fn(&S, &A) -> bool>>, Sc> {
        let (extractor, filter) = self.into_parts();
        UniConstraintStream::from_parts(
            extractor,
            AndUniFilter::new(
                filter,
                FnUniFilter::new(A::is_unassigned as fn(&S, &A) -> bool),
            ),
        )
    }
}
