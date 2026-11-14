use super::{Pair, ProjectView};

/// A real target/result pair or an owned default associated with one target row.
pub enum ComplementView<'a, L, R, T> {
    /// A target joined with a matching result row.
    Real(Pair<L, R>),
    /// A target without a matching result, carrying its owned default value.
    Default(ProjectView<'a, L, T>),
}
impl<L: Copy, R: Copy, T> Copy for ComplementView<'_, L, R, T> {}
impl<L: Copy, R: Copy, T> Clone for ComplementView<'_, L, R, T> {
    fn clone(&self) -> Self {
        *self
    }
}
