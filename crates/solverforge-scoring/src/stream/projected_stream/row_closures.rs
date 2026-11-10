//! Closures over projected rows, built in functions generic only over what
//! they capture so their types do not repeat the projected stream's type (see
//! `stream::closures`).

use super::super::filter::UniFilter;
use super::source::ProjectedRow;

/// A uni filter tested on a projected row's value.
pub(crate) fn projected_row_filter<S, Out, F>(
    filter: F,
) -> impl Fn(&S, &ProjectedRow<'_, Out>) -> bool + Send + Sync
where
    F: UniFilter<S, Out>,
{
    move |solution: &S, row: &ProjectedRow<'_, Out>| filter.test(solution, row.value)
}

/// `key` applied to a projected row's value.
pub(crate) fn projected_row_key<Out, K, G>(
    key: G,
) -> impl Fn(&ProjectedRow<'_, Out>) -> K + Send + Sync
where
    G: Fn(&Out) -> K + Send + Sync,
{
    move |row: &ProjectedRow<'_, Out>| key(row.value)
}
