use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use super::entity::EntityReference;
use super::pillar::{Pillar, SubPillarConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PillarGroup<V> {
    pub(crate) shared_value: V,
    pub(crate) pillar: Pillar,
}

impl<V> PillarGroup<V> {
    pub(crate) fn new(shared_value: V, pillar: Pillar) -> Self {
        Self {
            shared_value,
            pillar,
        }
    }
}

pub(crate) fn collect_pillar_groups<V, I>(
    assigned_entities: I,
    sub_pillar_config: &SubPillarConfig,
) -> Vec<PillarGroup<V>>
where
    V: Clone + Eq + Hash,
    I: IntoIterator<Item = (EntityReference, Option<V>)>,
{
    let mut grouped: HashMap<V, Vec<EntityReference>> = HashMap::new();
    for (entity_ref, value) in assigned_entities {
        let Some(value) = value else {
            continue;
        };
        grouped.entry(value).or_default().push(entity_ref);
    }

    let minimum_size = sub_pillar_config.minimum_size.max(2);
    let mut pillars: Vec<PillarGroup<V>> = grouped
        .into_iter()
        .map(|(shared_value, mut entities)| {
            entities.sort_by_key(|entity| entity.entity_index);
            PillarGroup::new(shared_value, Pillar::new(entities))
        })
        .filter(|group| group.pillar.size() >= minimum_size)
        .collect();
    pillars.sort_by_key(|group| {
        group
            .pillar
            .first()
            .map(|entity| entity.entity_index)
            .unwrap_or(usize::MAX)
    });

    if !sub_pillar_config.enabled {
        return pillars;
    }

    let maximum_size = sub_pillar_config.maximum_size.max(minimum_size);
    let mut sub_pillars = Vec::new();
    for group in pillars {
        let entities = &group.pillar.entities;
        let max_window = maximum_size.min(entities.len());
        if minimum_size > max_window {
            continue;
        }
        for window_size in minimum_size..=max_window {
            for start in 0..=entities.len() - window_size {
                sub_pillars.push(PillarGroup::new(
                    group.shared_value.clone(),
                    Pillar::new(entities[start..start + window_size].to_vec()),
                ));
            }
        }
    }
    sub_pillars
}

pub(crate) fn intersect_legal_values_for_pillar<I, F>(
    pillar: &Pillar,
    mut values_for_entity: F,
) -> Vec<usize>
where
    F: FnMut(usize) -> I,
    I: IntoIterator<Item = usize>,
{
    let Some(first_entity) = pillar.first() else {
        return Vec::new();
    };

    let mut intersection = Vec::new();
    for value in values_for_entity(first_entity.entity_index) {
        if !intersection.contains(&value) {
            intersection.push(value);
        }
    }

    for entity in pillar.iter().skip(1) {
        let legal_values: HashSet<usize> =
            values_for_entity(entity.entity_index).into_iter().collect();
        intersection.retain(|value| legal_values.contains(value));
        if intersection.is_empty() {
            break;
        }
    }

    intersection
}

pub(crate) fn pillar_accepts_value<I, F>(
    pillar: &Pillar,
    candidate_value: usize,
    values_for_entity: &mut F,
) -> bool
where
    F: FnMut(usize) -> I,
    I: IntoIterator<Item = usize>,
{
    pillar.iter().all(|entity| {
        values_for_entity(entity.entity_index)
            .into_iter()
            .any(|value| value == candidate_value)
    })
}

pub(crate) fn pillars_are_swap_compatible<I, F>(
    left: &PillarGroup<usize>,
    right: &PillarGroup<usize>,
    mut values_for_entity: F,
) -> bool
where
    F: FnMut(usize) -> I,
    I: IntoIterator<Item = usize>,
{
    pillar_accepts_value(&left.pillar, right.shared_value, &mut values_for_entity)
        && pillar_accepts_value(&right.pillar, left.shared_value, &mut values_for_entity)
}
