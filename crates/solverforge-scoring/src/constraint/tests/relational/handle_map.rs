use crate::stream::relational::{DenseRowStore, HandleMap};

#[test]
fn slot_map_checks_generations_and_replaces_without_hashing() {
    let mut rows = DenseRowStore::new();
    let old = rows.insert(());
    rows.retract(old);
    let new = rows.insert(());
    let mut map = HandleMap::new();
    assert_eq!(map.insert(old, 10), None);
    assert_eq!(map.get(old), Some(&10));
    assert_eq!(map.insert(new, 20), None);
    assert_eq!(map.get(old), None);
    assert_eq!(map.remove(old), None);
    assert_eq!(map.get(new), Some(&20));
    assert_eq!(map.insert(new, 30), Some(20));
    assert_eq!(map.remove(new), Some(30));
    assert_eq!(map.remove(new), None);
    map.insert(old, 40);
    map.clear();
    assert_eq!(map.get(old), None);
}
