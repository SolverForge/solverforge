/* P4 RED: ordered and interval index strategies.

Ordered probes must honor strict/inclusive direction on both query sides;
interval narrowing must return every overlapping candidate (recall over
precision — the exact predicate filters) with correct deletion. Fails
until `stream::relational::index` gains the ordered and interval
strategies.
*/

use crate::stream::relational::index::{IntervalIndex, OrderedIndex};
use crate::stream::relational::DenseRowStore;
use crate::stream::relational::RowHandle;

fn handles(n: usize) -> Vec<RowHandle> {
    let mut store = DenseRowStore::new();
    (0..n).map(|i| store.insert(i)).collect()
}

#[test]
fn ordered_probes_honor_strict_and_inclusive_bounds() {
    let live = handles(5);
    let mut index: OrderedIndex<i64> = OrderedIndex::new();
    for (handle, key) in live.iter().zip([10i64, 20, 20, 30, 40]) {
        index.insert(*handle, key);
    }
    // Strictly less than 20: only the 10.
    assert_eq!(index.less_than(&20, false), vec![live[0]]);
    // Less-or-equal 20: the 10 and both 20s.
    assert_eq!(index.less_than(&20, true), vec![live[0], live[1], live[2]]);
    // Strictly greater than 20: 30, 40.
    assert_eq!(index.greater_than(&20, false), vec![live[3], live[4]]);
    // Greater-or-equal 20: both 20s, 30, 40.
    assert_eq!(
        index.greater_than(&20, true),
        vec![live[1], live[2], live[3], live[4]]
    );

    // Deletion removes exactly the handle; empty buckets vanish.
    index.remove(live[1]);
    assert_eq!(index.less_than(&20, true), vec![live[0], live[2]]);
    index.remove(live[0]);
    index.remove(live[2]);
    assert_eq!(index.less_than(&20, true), vec![]);
}

#[test]
fn interval_narrowing_keeps_every_overlap_candidate() {
    let live = handles(4);
    let mut index: IntervalIndex<i64> = IntervalIndex::new();
    // [0, 10), [5, 15), [10, 20), [30, 40).
    for (handle, (s, e)) in live
        .iter()
        .zip([(0i64, 10i64), (5, 15), (10, 20), (30, 40)])
    {
        index.insert(*handle, s, e);
    }
    // Query [8, 12): overlaps the first three, not the last.
    let mut got = index.overlapping(8, 12);
    got.sort();
    let mut want = vec![live[0], live[1], live[2]];
    want.sort();
    assert_eq!(got, want);
    // Touching [20, 30): half-open, no overlap with any.
    assert_eq!(index.overlapping(20, 30), vec![]);

    // Deletion narrows exactly.
    index.remove(live[1]);
    let mut got = index.overlapping(8, 12);
    got.sort();
    assert_eq!(got, vec![live[0], live[2]]);
}
