#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SegmentRange {
    pub start: usize,
    pub end: usize,
}

impl SegmentRange {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end - self.start
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SegmentRelocationCoords {
    pub source_entity_index: usize,
    pub source_range: SegmentRange,
    pub dest_entity_index: usize,
    pub dest_position: usize,
}

impl SegmentRelocationCoords {
    pub const fn new(
        source_entity_index: usize,
        source_start: usize,
        source_end: usize,
        dest_entity_index: usize,
        dest_position: usize,
    ) -> Self {
        Self {
            source_entity_index,
            source_range: SegmentRange::new(source_start, source_end),
            dest_entity_index,
            dest_position,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SegmentRelocationLayout {
    pub exact: SegmentRelocationCoords,
    pub inverse: SegmentRelocationCoords,
}

pub(crate) fn derive_segment_relocation_layout(
    source_entity_index: usize,
    source_start: usize,
    source_end: usize,
    dest_entity_index: usize,
    dest_position: usize,
) -> SegmentRelocationLayout {
    let exact = SegmentRelocationCoords::new(
        source_entity_index,
        source_start,
        source_end,
        dest_entity_index,
        dest_position,
    );
    let moved_len = exact.source_range.len();
    let inverse = SegmentRelocationCoords::new(
        dest_entity_index,
        dest_position,
        dest_position + moved_len,
        source_entity_index,
        source_start,
    );

    SegmentRelocationLayout { exact, inverse }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SegmentSwapCoords {
    pub first_entity_index: usize,
    pub first_range: SegmentRange,
    pub second_entity_index: usize,
    pub second_range: SegmentRange,
}

impl SegmentSwapCoords {
    pub const fn new(
        first_entity_index: usize,
        first_start: usize,
        first_end: usize,
        second_entity_index: usize,
        second_start: usize,
        second_end: usize,
    ) -> Self {
        Self {
            first_entity_index,
            first_range: SegmentRange::new(first_start, first_end),
            second_entity_index,
            second_range: SegmentRange::new(second_start, second_end),
        }
    }

    pub const fn is_intra_list(self) -> bool {
        self.first_entity_index == self.second_entity_index
    }

    pub fn ordered_ranges(self) -> (SegmentRange, SegmentRange) {
        if self.first_range.start <= self.second_range.start {
            (self.first_range, self.second_range)
        } else {
            (self.second_range, self.first_range)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SegmentSwapLayout {
    pub exact: SegmentSwapCoords,
    pub inverse: SegmentSwapCoords,
}

pub(crate) fn derive_segment_swap_layout(
    first_entity_index: usize,
    first_start: usize,
    first_end: usize,
    second_entity_index: usize,
    second_start: usize,
    second_end: usize,
) -> SegmentSwapLayout {
    let exact = SegmentSwapCoords::new(
        first_entity_index,
        first_start,
        first_end,
        second_entity_index,
        second_start,
        second_end,
    );
    let first_len = exact.first_range.len();
    let second_len = exact.second_range.len();

    let inverse = if !exact.is_intra_list() {
        SegmentSwapCoords::new(
            first_entity_index,
            first_start,
            first_start + second_len,
            second_entity_index,
            second_start,
            second_start + first_len,
        )
    } else if first_start < second_start {
        SegmentSwapCoords::new(
            first_entity_index,
            first_start,
            first_start + second_len,
            second_entity_index,
            second_start - first_len + second_len,
            second_start - first_len + second_len + first_len,
        )
    } else {
        SegmentSwapCoords::new(
            first_entity_index,
            first_start - second_len + first_len,
            first_start - second_len + first_len + second_len,
            second_entity_index,
            second_start,
            second_start + first_len,
        )
    };

    SegmentSwapLayout { exact, inverse }
}
