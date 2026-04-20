#[inline(always)]
pub(crate) fn for_each_sublist_segment(
    route_len: usize,
    min_size: usize,
    max_size: usize,
    mut visit: impl FnMut(usize, usize, usize),
) {
    for start in 0..route_len {
        for size in min_size..=max_size {
            let end = start + size;
            if end > route_len {
                break;
            }
            visit(start, end, size);
        }
    }
}

#[inline(always)]
pub(crate) fn for_each_following_sublist_segment(
    route_len: usize,
    start_at: usize,
    min_size: usize,
    max_size: usize,
    mut visit: impl FnMut(usize, usize, usize),
) {
    for start in start_at..route_len {
        for size in min_size..=max_size {
            let end = start + size;
            if end > route_len {
                break;
            }
            visit(start, end, size);
        }
    }
}

#[inline]
pub(crate) fn count_sublist_segments(route_len: usize, min_size: usize, max_size: usize) -> usize {
    let mut count = 0;
    for_each_sublist_segment(route_len, min_size, max_size, |_, _, _| {
        count += 1;
    });
    count
}

#[inline]
pub(crate) fn count_sublist_change_moves_for_len(
    route_len: usize,
    inter_destinations: usize,
    min_size: usize,
    max_size: usize,
) -> usize {
    let mut count = 0;
    for_each_sublist_segment(route_len, min_size, max_size, |_, _, segment_size| {
        count += route_len - segment_size + inter_destinations;
    });
    count
}

#[inline]
pub(crate) fn count_intra_sublist_swap_moves_for_len(
    route_len: usize,
    min_size: usize,
    max_size: usize,
) -> usize {
    let mut count = 0;
    for_each_sublist_segment(route_len, min_size, max_size, |_, first_end, _| {
        for_each_following_sublist_segment(route_len, first_end, min_size, max_size, |_, _, _| {
            count += 1;
        });
    });
    count
}
