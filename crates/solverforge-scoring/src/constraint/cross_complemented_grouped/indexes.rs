use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

/* Hash for a group key. Match/join indexing now belongs to the common join
operator, so this is the only join-side index helper still needed. */
pub(super) fn key_hash<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/* Removes one target index from its group's complement bucket. */
pub(super) fn remove_index_from_group_bucket(
    by_group: &mut HashMap<usize, Vec<usize>>,
    group_id: usize,
    t_idx: usize,
) {
    let mut remove_bucket = false;
    if let Some(indices) = by_group.get_mut(&group_id) {
        if let Some(position) = indices.iter().position(|candidate| *candidate == t_idx) {
            indices.swap_remove(position);
        }
        remove_bucket = indices.is_empty();
    }
    if remove_bucket {
        by_group.remove(&group_id);
    }
}
