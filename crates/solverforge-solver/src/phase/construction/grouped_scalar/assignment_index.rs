use std::collections::HashMap;

pub(super) fn push_indexed_entity<K>(
    index: &mut HashMap<K, Vec<usize>>,
    key: K,
    entity_index: usize,
) where
    K: Eq + std::hash::Hash,
{
    let entities = index.entry(key).or_default();
    if !entities.contains(&entity_index) {
        entities.push(entity_index);
    }
}

pub(super) fn remove_indexed_entity<K>(
    index: &mut HashMap<K, Vec<usize>>,
    key: K,
    entity_index: usize,
) where
    K: Eq + std::hash::Hash,
{
    if let Some(entities) = index.get_mut(&key) {
        entities.retain(|occupant| *occupant != entity_index);
        if entities.is_empty() {
            index.remove(&key);
        }
    }
}
