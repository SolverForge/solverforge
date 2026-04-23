pub(crate) fn truncate_nearby_candidates<T>(
    candidates: &mut Vec<(T, f64, usize)>,
    max_nearby: usize,
) {
    candidates.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.2.cmp(&right.2))
    });
    candidates.truncate(max_nearby);
}
