pub(crate) type NearbyCandidate = (usize, usize, f64);

pub(crate) fn sort_and_limit_nearby_candidates(
    candidates: &mut Vec<NearbyCandidate>,
    max_nearby: usize,
) {
    candidates.sort_by(|left, right| {
        left.2
            .partial_cmp(&right.2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(max_nearby);
}
