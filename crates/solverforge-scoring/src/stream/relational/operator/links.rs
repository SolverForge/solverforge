use crate::stream::relational::{HandleMap, RowHandle};

#[inline]
pub(super) fn remove_link(
    links: &mut HandleMap<Vec<RowHandle>>,
    input: RowHandle,
    output: RowHandle,
    position: usize,
) -> Option<RowHandle> {
    let bucket = links.get_mut(input)?;
    debug_assert_eq!(bucket[position], output);
    bucket.swap_remove(position);
    let moved = bucket.get(position).copied();
    if bucket.is_empty() {
        links.remove(input);
    }
    moved
}
