use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

pub(super) fn key_hash<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn matching_indexed_indices<K>(
    indexes_by_hash: &HashMap<u64, Vec<usize>>,
    index_to_key: &HashMap<usize, K>,
    key: &K,
) -> Vec<usize>
where
    K: Eq + Hash,
{
    let hash = key_hash(key);
    let Some(indices) = indexes_by_hash.get(&hash) else {
        return Vec::new();
    };
    let mut matches = Vec::new();
    for &idx in indices {
        if index_to_key.get(&idx).is_some_and(|stored| stored == key) {
            matches.push(idx);
        }
    }
    matches
}

pub(super) fn remove_index_from_hash_bucket<K>(
    indexes_by_hash: &mut HashMap<u64, Vec<usize>>,
    key: &K,
    idx: usize,
) where
    K: Hash,
{
    let hash = key_hash(key);
    let mut remove_bucket = false;
    if let Some(indices) = indexes_by_hash.get_mut(&hash) {
        if let Some(pos) = indices.iter().position(|candidate| *candidate == idx) {
            indices.swap_remove(pos);
        }
        remove_bucket = indices.is_empty();
    }
    if remove_bucket {
        indexes_by_hash.remove(&hash);
    }
}

pub(super) fn remove_index_from_group_bucket(
    indexes_by_group: &mut HashMap<usize, Vec<usize>>,
    group_id: usize,
    idx: usize,
) {
    let mut remove_bucket = false;
    if let Some(indices) = indexes_by_group.get_mut(&group_id) {
        if let Some(pos) = indices.iter().position(|candidate| *candidate == idx) {
            indices.swap_remove(pos);
        }
        remove_bucket = indices.is_empty();
    }
    if remove_bucket {
        indexes_by_group.remove(&group_id);
    }
}
