use std::cmp::Ordering;

use super::{BoundedNearbyCandidates, NearbyCandidate};

fn stable_sort_and_limit(
    mut candidates: Vec<NearbyCandidate>,
    limit: usize,
) -> Vec<NearbyCandidate> {
    candidates.sort_by(|left, right| left.2.partial_cmp(&right.2).unwrap_or(Ordering::Equal));
    candidates.truncate(limit);
    candidates
}

#[test]
fn bounded_order_matches_stable_full_sort() {
    for len in 0_usize..96 {
        let mut state = 0x9E37_79B9_u32 ^ len as u32;
        let candidates = (0..len)
            .map(|index| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let distance = if index % 17 == 0 {
                    -0.0
                } else {
                    f64::from(state % 11)
                };
                (index / 7, index, distance)
            })
            .collect::<Vec<_>>();
        for limit in 0..=len + 2 {
            let expected = stable_sort_and_limit(candidates.clone(), limit);
            let mut bounded = BoundedNearbyCandidates::new(limit);
            for &candidate in &candidates {
                bounded.offer(candidate);
            }
            assert_eq!(bounded.retained, expected, "len={len}, limit={limit}");
            assert_eq!(bounded.len(), expected.len());
            for (index, &(entity, position, _)) in expected.iter().enumerate() {
                assert_eq!(bounded.destination(index), (entity, position));
            }
        }
    }
}

#[test]
fn clear_reuses_capacity_without_growing_past_limit() {
    let mut bounded = BoundedNearbyCandidates::new(4);
    let mut capacity = None;
    for round in 0..3 {
        bounded.clear();
        assert!(bounded.is_empty());
        for index in 0..64 {
            bounded.offer((round, index, f64::from((index * 37 % 64) as u32)));
        }
        assert_eq!(bounded.len(), 4);
        let after_round = bounded.retained.capacity();
        assert!(
            after_round <= 8,
            "capacity {after_round} grew past the limit"
        );
        assert_eq!(*capacity.get_or_insert(after_round), after_round);
    }
    assert_eq!(
        bounded.retained,
        [(2, 0, 0.0), (2, 45, 1.0), (2, 26, 2.0), (2, 7, 3.0)]
    );
}
