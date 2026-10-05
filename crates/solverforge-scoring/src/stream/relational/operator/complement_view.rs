use super::{ExplainRow, Pair, ProjectView};
use crate::api::analysis::EntityRef;

/// A real target/result pair or an owned default associated with one target row.
pub enum ComplementView<'a, L, R, T> {
    Real(Pair<L, R>),
    Default(ProjectView<'a, L, T>),
}
impl<L: Copy, R: Copy, T> Copy for ComplementView<'_, L, R, T> {}
impl<L: Copy, R: Copy, T> Clone for ComplementView<'_, L, R, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<L: ExplainRow, R: ExplainRow, T> ExplainRow for ComplementView<'_, L, R, T> {
    fn explain(&self, entities: &mut Vec<EntityRef>) {
        match self {
            Self::Real(pair) => pair.explain(entities),
            Self::Default(projected) => projected.explain(entities),
        }
    }
}
