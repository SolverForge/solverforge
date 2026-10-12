use solverforge_core::fast_hash::FastMap;

pub(super) fn push_indexed_entity<K>(
    index: &mut FastMap<K, Vec<usize>>,
    pool: &mut Vec<Vec<usize>>,
    key: K,
    entity_index: usize,
) where
    K: Eq + std::hash::Hash,
{
    if let Some(entities) = index.get_mut(&key) {
        if !entities.contains(&entity_index) {
            entities.push(entity_index);
        }
        return;
    }
    let mut entities = pool.pop().unwrap_or_default();
    entities.push(entity_index);
    index.insert(key, entities);
}

pub(super) fn remove_indexed_entity<K>(
    index: &mut FastMap<K, Vec<usize>>,
    pool: &mut Vec<Vec<usize>>,
    key: K,
    entity_index: usize,
) where
    K: Eq + std::hash::Hash,
{
    if let Some(entities) = index.get_mut(&key) {
        entities.retain(|occupant| *occupant != entity_index);
        if entities.is_empty() {
            let entities = index
                .remove(&key)
                .expect("checked empty indexed row must exist");
            pool.push(entities);
        }
    }
}
