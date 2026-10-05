/* Stable row identity stress: handles and dense storage.

Covers the `stream::relational` identity/storage contract: generational
handles reject stale generations after slot reuse, equal payloads stay
distinct rows, and every live handle resolves after `swap_remove` repair.
Composed joined/branch/binding identities arrive with the P2 operators.
*/

use std::collections::HashSet;

use crate::stream::relational::{DenseRowStore, RowHandle};

#[test]
fn stable_handles_reject_stale_generations_across_slot_reuse() {
    let mut store = DenseRowStore::new();
    let keep = store.insert(10);
    let doomed = store.insert(20);
    let other = store.insert(30);
    assert_eq!(store.get(keep), Some(&10));
    assert_eq!(store.get(doomed), Some(&20));

    assert_eq!(store.retract(doomed), Some(20));
    // Double retract resolves to nothing; the stale handle never revives.
    assert_eq!(store.retract(doomed), None);
    assert_eq!(store.get(doomed), None);

    // Slot reuse mints a bumped generation; the old handle stays dead.
    let reused = store.insert(40);
    assert_ne!(reused, doomed);
    assert_eq!(store.get(reused), Some(&40));
    assert_eq!(store.get(doomed), None);

    // Survivors are intact across the swap repair.
    assert_eq!(store.get(keep), Some(&10));
    assert_eq!(store.get(other), Some(&30));
    assert_eq!(store.len(), 3);
    assert!(!store.is_empty());
}

#[test]
fn equal_values_are_distinct_rows_with_bag_multiplicity() {
    let mut store = DenseRowStore::new();
    let first = store.insert(7);
    let second = store.insert(7);
    assert_ne!(first, second);
    assert_eq!(store.len(), 2);

    assert_eq!(store.retract(first), Some(7));
    assert_eq!(store.get(second), Some(&7));
    assert_eq!(store.len(), 1);
}

#[test]
fn swap_repair_keeps_every_live_handle_resolvable() {
    let mut store = DenseRowStore::new();
    let handles: Vec<RowHandle> = (0..16).map(|value| store.insert(value)).collect();

    // Retract a scattered pattern, including the dense tail and head.
    let mut live_mask = [true; 16];
    for doomed in [0, 3, 5, 9, 12, 15] {
        assert_eq!(store.retract(handles[doomed]), Some(doomed as i32));
        live_mask[doomed] = false;
    }
    // Double retract resolves to nothing; the stale handle never revives.
    assert_eq!(store.retract(handles[0]), None);

    // Insert through the freed slots; all generations stay unique.
    let mut seen = HashSet::new();
    for (handle, alive) in handles.iter().zip(live_mask) {
        if alive {
            assert!(seen.insert(*handle));
        }
    }
    let mut live: Vec<RowHandle> = handles
        .iter()
        .zip(live_mask)
        .filter(|(_, alive)| *alive)
        .map(|(handle, _)| *handle)
        .collect();
    for value in 100..104 {
        let fresh = store.insert(value);
        assert!(seen.insert(fresh));
        assert_eq!(store.get(fresh), Some(&value));
        live.push(fresh);
    }

    // Every live handle resolves to its exact payload; the multiset matches.
    let mut resolved: Vec<(RowHandle, i32)> = live
        .iter()
        .map(|handle| (*handle, *store.get(*handle).expect("live handle resolves")))
        .collect();
    resolved.sort_by_key(|(_, value)| *value);
    let mut expected: Vec<i32> = (0..16)
        .filter(|v| ![0, 3, 5, 9, 12, 15].contains(v))
        .collect();
    expected.extend(100..104);
    let actual: Vec<i32> = resolved.iter().map(|(_, value)| *value).collect();
    assert_eq!(actual, expected);
    assert_eq!(store.len(), expected.len());

    // Full traversal agrees with point lookups.
    let mut traversed: Vec<i32> = store.iter().map(|(_, value)| *value).collect();
    traversed.sort_unstable();
    assert_eq!(traversed, expected);
}
