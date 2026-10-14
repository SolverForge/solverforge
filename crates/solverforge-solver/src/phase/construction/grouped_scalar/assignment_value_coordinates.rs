use super::assignment_candidate::ScalarAssignmentMoveOptions;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ValuePairKind {
    Unordered,
    Directed,
}

/// Position state of a value-pair walk. Positions index the assignment
/// index's `values` slice, which is lent per pull instead of copied.
pub(super) struct ValuePairCoordinates {
    kind: ValuePairKind,
    n: usize,
    left: usize,
    right: usize,
    remaining: u128,
}

impl ValuePairCoordinates {
    pub(super) fn new(
        values: &[usize],
        kind: ValuePairKind,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        let n = values.len();
        let total = n as u128 * n.saturating_sub(1) as u128
            / if kind == ValuePairKind::Unordered {
                2
            } else {
                1
            };
        let mut rank = if total == 0 {
            0
        } else {
            options.entity_offset as u128 % total
        };
        let mut left = 0;
        let right = if kind == ValuePairKind::Directed && total != 0 {
            left = (rank / (n - 1) as u128) as usize;
            rank %= (n - 1) as u128;
            rank as usize + usize::from(rank >= left as u128)
        } else {
            while total != 0 && rank >= (n - left - 1) as u128 {
                rank -= (n - left - 1) as u128;
                left += 1;
            }
            left + 1 + rank as usize
        };
        Self {
            kind,
            n,
            left,
            right,
            remaining: total,
        }
    }

    pub(super) fn current(&self, values: &[usize]) -> Option<(usize, usize)> {
        debug_assert_eq!(values.len(), self.n);
        if self.remaining == 0 {
            return None;
        }
        Some((values[self.left], values[self.right]))
    }

    pub(super) fn advance(&mut self) {
        if self.remaining == 0 {
            return;
        }
        self.remaining -= 1;
        self.right += 1;
        let n = self.n;
        if self.kind == ValuePairKind::Directed {
            if self.right == self.left {
                self.right += 1;
            }
            if self.right == n {
                self.left = (self.left + 1) % n;
                self.right = usize::from(self.left == 0);
            }
        } else if self.right == n {
            self.left += 1;
            if self.left + 1 == n {
                self.left = 0;
            }
            self.right = self.left + 1;
        }
    }
}

/// Position state of a value-cycle walk; the values slice is lent per pull.
pub(super) struct ValueCycleCoordinates {
    n: usize,
    first: usize,
    second: usize,
    third: usize,
    reverse: bool,
    remaining: u128,
}

impl ValueCycleCoordinates {
    pub(super) fn new(values: &[usize], options: ScalarAssignmentMoveOptions) -> Self {
        let n = values.len();
        let total = n as u128 * n.saturating_sub(1) as u128 * n.saturating_sub(2) as u128 / 3;
        let offset = if total == 0 {
            0
        } else {
            options.entity_offset as u128 % total
        };
        let mut rank = offset / 2;
        let mut first = 0;
        while total != 0 && rank >= (n - first - 1) as u128 * (n - first - 2) as u128 / 2 {
            rank -= (n - first - 1) as u128 * (n - first - 2) as u128 / 2;
            first += 1;
        }
        let mut second = first + 1;
        while total != 0 && rank >= (n - second - 1) as u128 {
            rank -= (n - second - 1) as u128;
            second += 1;
        }
        Self {
            n,
            first,
            second,
            third: second + 1 + rank as usize,
            reverse: offset % 2 != 0,
            remaining: total,
        }
    }

    pub(super) fn current(&self, values: &[usize]) -> Option<(usize, usize, usize)> {
        debug_assert_eq!(values.len(), self.n);
        if self.remaining == 0 {
            return None;
        }
        let (second, third) = if self.reverse {
            (self.third, self.second)
        } else {
            (self.second, self.third)
        };
        Some((values[self.first], values[second], values[third]))
    }

    pub(super) fn advance(&mut self) {
        if self.remaining == 0 {
            return;
        }
        self.remaining -= 1;
        self.reverse = !self.reverse;
        if self.reverse {
            return;
        }
        self.third += 1;
        let n = self.n;
        if self.third == n {
            self.second += 1;
            if self.second + 1 == n {
                self.first += 1;
                if self.first + 2 == n {
                    self.first = 0;
                }
                self.second = self.first + 1;
            }
            self.third = self.second + 1;
        }
    }
}

#[cfg(test)]
#[path = "tests/assignment_value_coordinates.rs"]
mod tests;
