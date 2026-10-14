use super::*;
use smallvec::SmallVec;

struct MembershipUpdate {
    entity: usize,
    value: Option<usize>,
    required: bool,
    capacity_key: Option<usize>,
    sequence_key: Option<usize>,
}

impl ScalarAssignmentState {
    pub(super) fn refresh_rows<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        rows: impl IntoIterator<Item = (usize, Option<usize>)>,
    ) {
        let updates: SmallVec<[MembershipUpdate; 8]> = rows
            .into_iter()
            .map(|(entity, value)| MembershipUpdate {
                entity,
                value,
                required: group.is_required(solution, entity),
                capacity_key: value.and_then(|value| group.capacity_key(solution, entity, value)),
                sequence_key: value.and_then(|value| group.sequence_key(solution, entity, value)),
            })
            .collect();
        // Retract every old row before inserting any final membership. Keys come
        // from the base, never from callbacks evaluated on the changed solution.
        for update in &updates {
            let entity = update.entity;
            if let Some(value) = self.values[entity] {
                remove_member(&mut self.assigned_by_value, value, entity);
                if let Some(sequence) = self.sequence_keys[entity] {
                    remove_member(&mut self.assigned_by_sequence, (value, sequence), entity);
                }
                if let Some(key) = self.capacity_keys[entity] {
                    remove_member(&mut self.occupancy, key, entity);
                }
            }
        }
        for update in updates {
            let entity = update.entity;
            self.values[entity] = update.value;
            self.required[entity] = update.required;
            self.capacity_keys[entity] = update.capacity_key;
            self.sequence_keys[entity] = update.sequence_key;
            if let Some(value) = update.value {
                insert_member(&mut self.assigned_by_value, value, entity);
                if let Some(sequence) = update.sequence_key {
                    insert_member(&mut self.assigned_by_sequence, (value, sequence), entity);
                }
                if let Some(key) = update.capacity_key {
                    insert_member(&mut self.occupancy, key, entity);
                }
            }
        }
    }
}

fn remove_member<K: Eq + std::hash::Hash>(map: &mut FastMap<K, Vec<usize>>, key: K, entity: usize) {
    let Some(members) = map.get_mut(&key) else {
        return;
    };
    if let Ok(position) = members.binary_search(&entity) {
        members.remove(position);
    }
    if members.is_empty() {
        map.remove(&key);
    }
}

fn insert_member<K: Eq + std::hash::Hash>(map: &mut FastMap<K, Vec<usize>>, key: K, entity: usize) {
    let members = map.entry(key).or_default();
    if let Err(position) = members.binary_search(&entity) {
        members.insert(position, entity);
    }
}
