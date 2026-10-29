// Tests for pillar grouping, the step the runtime pillar cursors build on.

use crate::heuristic::selector::entity::EntityReference;
use crate::heuristic::selector::pillar::{Pillar, SubPillarConfig};
use crate::heuristic::selector::pillar_support::collect_pillar_groups;

fn whole_pillars() -> SubPillarConfig {
    SubPillarConfig::none()
}

fn groups(values: &[Option<i32>], config: &SubPillarConfig) -> Vec<Pillar> {
    collect_pillar_groups(
        values
            .iter()
            .enumerate()
            .map(|(entity_index, value)| (EntityReference::new(0, entity_index), *value)),
        config,
    )
    .into_iter()
    .map(|group| group.pillar)
    .collect()
}

#[test]
fn pillar_reports_size_and_first_entity() {
    let pillar = Pillar::new(vec![EntityReference::new(0, 0), EntityReference::new(0, 1)]);

    assert_eq!(pillar.size(), 2);
    assert_eq!(pillar.first(), Some(&EntityReference::new(0, 0)));
    assert_eq!(Pillar::new(vec![]).first(), None);
}

#[test]
fn groups_entities_by_shared_value_and_drops_singletons() {
    let pillars = groups(
        &[Some(1), Some(1), Some(2), Some(2), Some(2), Some(3)],
        &whole_pillars(),
    );

    let mut sizes: Vec<_> = pillars.iter().map(Pillar::size).collect();
    sizes.sort();
    assert_eq!(sizes, vec![2, 3]);
}

#[test]
fn minimum_size_filters_small_groups() {
    let config = SubPillarConfig {
        minimum_size: 3,
        ..SubPillarConfig::none()
    };
    let pillars = groups(
        &[Some(1), Some(1), Some(2), Some(2), Some(2), Some(3)],
        &config,
    );

    assert_eq!(pillars.len(), 1);
    assert_eq!(pillars[0].size(), 3);
}

#[test]
fn groups_keep_canonical_first_entity_order() {
    let pillars = groups(
        &[Some(2), Some(2), Some(1), Some(1), Some(1)],
        &whole_pillars(),
    );

    let first_indices: Vec<_> = pillars
        .iter()
        .map(|pillar| pillar.first().expect("pillar first entity").entity_index)
        .collect();
    assert_eq!(first_indices, vec![0, 2]);
    assert_eq!(pillars[0].size(), 2);
    assert_eq!(pillars[1].size(), 3);
}

#[test]
fn unassigned_entities_join_no_pillar() {
    let pillars = groups(&[Some(1), None, None, Some(1)], &whole_pillars());

    assert_eq!(pillars.len(), 1);
    assert_eq!(pillars[0].size(), 2);
}

#[test]
fn no_entities_yield_no_pillars() {
    assert!(groups(&[], &whole_pillars()).is_empty());
}
