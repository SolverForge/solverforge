pub(crate) type NearbyCandidate = (usize, usize, f64);

pub(crate) fn sort_and_limit_nearby_candidates(
    candidates: &mut Vec<NearbyCandidate>,
    max_nearby: usize,
) {
    use std::cmp::Ordering;

    if max_nearby == 0 {
        candidates.clear();
        return;
    }

    let mut retained = 0;
    for read in 0..candidates.len() {
        let candidate = candidates[read];
        let insertion = candidates[..retained].partition_point(|existing| {
            existing
                .2
                .partial_cmp(&candidate.2)
                .unwrap_or(Ordering::Equal)
                != Ordering::Greater
        });
        if insertion >= max_nearby {
            continue;
        }

        let next_retained = (retained + 1).min(max_nearby);
        candidates.copy_within(insertion..next_retained - 1, insertion + 1);
        candidates[insertion] = candidate;
        retained = next_retained;
    }
    candidates.truncate(retained);
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{sort_and_limit_nearby_candidates, NearbyCandidate};

    fn stable_sort_and_limit(
        mut candidates: Vec<NearbyCandidate>,
        max_nearby: usize,
    ) -> Vec<NearbyCandidate> {
        candidates.sort_by(|left, right| left.2.partial_cmp(&right.2).unwrap_or(Ordering::Equal));
        candidates.truncate(max_nearby);
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
            for max_nearby in 0..=len + 2 {
                let expected = stable_sort_and_limit(candidates.clone(), max_nearby);
                let mut actual = candidates.clone();
                sort_and_limit_nearby_candidates(&mut actual, max_nearby);
                assert_eq!(actual, expected, "len={len}, max_nearby={max_nearby}");
            }
        }
    }
}
