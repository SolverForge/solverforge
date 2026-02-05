//! Tests for supply infrastructure.

use super::*;

// ============================================================================
// InverseSupply Tests
// ============================================================================

mod inverse_supply {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        supply.insert(2, 1);

        assert_eq!(supply.get(&1), Some(0));
        assert_eq!(supply.get(&2), Some(1));
        assert_eq!(supply.get(&3), None);
    }

    #[test]
    fn test_remove() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        assert_eq!(supply.len(), 1);

        let removed = supply.remove(&1);
        assert_eq!(removed, Some(0));
        assert_eq!(supply.len(), 0);
        assert_eq!(supply.get(&1), None);
    }

    #[test]
    fn test_update() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        supply.update(Some(&1), 2, 0);

        assert_eq!(supply.get(&1), None);
        assert_eq!(supply.get(&2), Some(0));
    }

    #[test]
    fn test_clear() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        supply.insert(2, 1);
        supply.insert(3, 2);

        assert_eq!(supply.len(), 3);

        supply.clear();

        assert!(supply.is_empty());
    }

    #[test]
    fn test_overwrite() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        let old = supply.insert(1, 5);

        assert_eq!(old, Some(0));
        assert_eq!(supply.get(&1), Some(5));
        assert_eq!(supply.len(), 1);
    }

    #[test]
    fn test_iter() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(10, 0);
        supply.insert(20, 1);
        supply.insert(30, 2);

        let mut pairs: Vec<_> = supply.iter().map(|(&v, &i)| (v, i)).collect();
        pairs.sort();

        assert_eq!(pairs, vec![(10, 0), (20, 1), (30, 2)]);
    }
}

// ============================================================================
// AnchorSupply Tests
// ============================================================================

mod anchor_supply {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut supply = AnchorSupply::new();

        supply.set(0, 10);
        supply.set(1, 10);
        supply.set(2, 20);

        assert_eq!(supply.get(0), Some(10));
        assert_eq!(supply.get(1), Some(10));
        assert_eq!(supply.get(2), Some(20));
        assert_eq!(supply.get(99), None);
    }

    #[test]
    fn test_remove() {
        let mut supply = AnchorSupply::new();

        supply.set(0, 10);
        assert_eq!(supply.len(), 1);

        let removed = supply.remove(0);
        assert_eq!(removed, Some(10));
        assert!(supply.is_empty());
    }

    #[test]
    fn test_cascade() {
        let mut supply = AnchorSupply::new();

        supply.cascade([0, 1, 2, 3], 5);

        assert_eq!(supply.get(0), Some(5));
        assert_eq!(supply.get(1), Some(5));
        assert_eq!(supply.get(2), Some(5));
        assert_eq!(supply.get(3), Some(5));
        assert_eq!(supply.len(), 4);
    }

    #[test]
    fn test_update_anchor() {
        let mut supply = AnchorSupply::new();

        supply.set(0, 10);
        assert_eq!(supply.get(0), Some(10));

        supply.set(0, 20);
        assert_eq!(supply.get(0), Some(20));
        assert_eq!(supply.len(), 1);
    }

    #[test]
    fn test_entities_for_anchor() {
        let mut supply = AnchorSupply::new();

        supply.set(0, 10);
        supply.set(1, 10);
        supply.set(2, 20);
        supply.set(3, 10);

        let mut entities = supply.entities_for_anchor(10);
        entities.sort();

        assert_eq!(entities, vec![0, 1, 3]);
    }

    #[test]
    fn test_clear() {
        let mut supply = AnchorSupply::new();

        supply.cascade(0..10, 0);
        assert_eq!(supply.len(), 10);

        supply.clear();
        assert!(supply.is_empty());
    }
}

// ============================================================================
// ListStateSupply Tests
// ============================================================================

mod list_state_supply {
    use super::*;

    #[test]
    fn test_assign_and_get() {
        let mut supply: ListStateSupply<usize> = ListStateSupply::with_unassigned(3);

        supply.assign(0, 10, 0);
        supply.assign(1, 10, 1);
        supply.assign(2, 20, 0);

        assert_eq!(
            supply.get_position(&0),
            Some(ElementPosition {
                entity_idx: 10,
                list_idx: 0
            })
        );
        assert_eq!(supply.get_entity(&1), Some(10));
        assert_eq!(supply.get_list_index(&2), Some(0));
        assert_eq!(supply.unassigned_count(), 0);
    }

    #[test]
    fn test_unassign() {
        let mut supply: ListStateSupply<usize> = ListStateSupply::with_unassigned(2);

        supply.assign(0, 10, 0);
        supply.assign(1, 10, 1);
        assert_eq!(supply.unassigned_count(), 0);

        let old = supply.unassign(&0);
        assert_eq!(
            old,
            Some(ElementPosition {
                entity_idx: 10,
                list_idx: 0
            })
        );
        assert_eq!(supply.unassigned_count(), 1);
        assert!(!supply.is_assigned(&0));
    }

    #[test]
    fn test_update() {
        let mut supply: ListStateSupply<usize> = ListStateSupply::with_unassigned(1);

        supply.assign(0, 10, 0);

        // Update to new position
        let changed = supply.update(&0, 10, 5);
        assert!(changed);
        assert_eq!(supply.get_list_index(&0), Some(5));

        // Update with same values - no change
        let changed = supply.update(&0, 10, 5);
        assert!(!changed);

        // Update to different entity
        let changed = supply.update(&0, 20, 0);
        assert!(changed);
        assert_eq!(supply.get_entity(&0), Some(20));
    }

    #[test]
    fn test_elements_for_entity() {
        let mut supply: ListStateSupply<usize> = ListStateSupply::new();

        supply.assign(0, 10, 0);
        supply.assign(1, 10, 1);
        supply.assign(2, 20, 0);
        supply.assign(3, 10, 2);

        let mut elements: Vec<_> = supply
            .elements_for_entity(10)
            .into_iter()
            .copied()
            .collect();
        elements.sort();

        assert_eq!(elements, vec![0, 1, 3]);
    }

    #[test]
    fn test_clear() {
        let mut supply: ListStateSupply<usize> = ListStateSupply::with_unassigned(5);

        supply.assign(0, 10, 0);
        supply.assign(1, 10, 1);
        assert_eq!(supply.unassigned_count(), 3);
        assert_eq!(supply.assigned_count(), 2);

        supply.clear();
        assert_eq!(supply.unassigned_count(), 5);
        assert_eq!(supply.assigned_count(), 0);
    }

    #[test]
    fn test_is_assigned() {
        let mut supply: ListStateSupply<usize> = ListStateSupply::new();

        assert!(!supply.is_assigned(&0));

        supply.assign(0, 10, 0);
        assert!(supply.is_assigned(&0));

        supply.unassign(&0);
        assert!(!supply.is_assigned(&0));
    }
}
