#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SavingsEntry {
    pub(crate) saving: i64,
    pub(crate) owner_idx: usize,
    pub(crate) left_idx: usize,
    pub(crate) right_idx: usize,
}

pub(crate) fn sort_savings(entries: &mut [SavingsEntry]) {
    entries.sort_unstable_by(|left, right| {
        right
            .saving
            .cmp(&left.saving)
            .then_with(|| left.owner_idx.cmp(&right.owner_idx))
            .then_with(|| left.left_idx.cmp(&right.left_idx))
            .then_with(|| left.right_idx.cmp(&right.right_idx))
    });
}
