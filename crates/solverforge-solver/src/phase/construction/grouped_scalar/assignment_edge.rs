#[derive(Clone, Copy)]
pub(super) struct ForcedAssignment {
    pub(super) entity_index: usize,
    pub(super) value: usize,
}

#[derive(Clone, Copy)]
pub(super) struct SequenceEdge {
    pub(super) value: usize,
    pub(super) left_sequence: usize,
    pub(super) right_sequence: usize,
    pub(super) forced_left: Option<ForcedAssignment>,
    pub(super) forced_right: Option<ForcedAssignment>,
}

impl SequenceEdge {
    pub(super) fn new(value: usize, left_sequence: usize, right_sequence: usize) -> Self {
        Self {
            value,
            left_sequence,
            right_sequence,
            forced_left: None,
            forced_right: None,
        }
    }

    pub(super) fn with_forced_left(mut self, forced_left: ForcedAssignment) -> Self {
        self.forced_left = Some(forced_left);
        self
    }

    pub(super) fn with_forced_right(mut self, forced_right: ForcedAssignment) -> Self {
        self.forced_right = Some(forced_right);
        self
    }
}
