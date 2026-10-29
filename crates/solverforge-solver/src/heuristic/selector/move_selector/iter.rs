use solverforge_config::SelectionOrder;

pub struct MoveSelectorIter<'a, S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    cursor: C,
    solution: &'a S,
    _phantom: PhantomData<fn() -> M>,
}

impl<'a, S, M, C> MoveSelectorIter<'a, S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn new(cursor: C, solution: &'a S) -> Self {
        Self {
            cursor,
            solution,
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveStreamContext {
    step_index: u64,
    step_seed: u64,
    accepted_count_limit: Option<usize>,
    selection_order: SelectionOrder,
}

impl MoveStreamContext {
    pub const fn new(
        step_index: u64,
        step_seed: u64,
        accepted_count_limit: Option<usize>,
    ) -> Self {
        Self {
            step_index,
            step_seed,
            accepted_count_limit,
            selection_order: SelectionOrder::Original,
        }
    }

    pub const fn with_selection_order(mut self, selection_order: SelectionOrder) -> Self {
        self.selection_order = selection_order;
        self
    }

    pub const fn selection_order(self) -> SelectionOrder {
        self.selection_order
    }

    pub const fn step_index(self) -> u64 {
        self.step_index
    }

    pub const fn step_seed(self) -> u64 {
        self.step_seed
    }

    pub const fn accepted_count_limit(self) -> Option<usize> {
        self.accepted_count_limit
    }

    pub fn start_offset(self, len: usize, salt: u64) -> usize {
        if len <= 1 {
            return 0;
        }
        if self.is_canonical() {
            return 0;
        }
        (self.mixed_seed(salt) as usize) % len
    }

    pub fn stride(self, len: usize, salt: u64) -> usize {
        if len <= 1 {
            return 1;
        }
        if self.is_canonical() {
            return 1;
        }
        let mut stride = (self.mixed_seed(salt) as usize % (len - 1)) + 1;
        while gcd(stride, len) != 1 {
            stride = if stride == len - 1 { 1 } else { stride + 1 };
        }
        stride
    }

    pub fn offset_seed(self, salt: u64) -> usize {
        if self.is_canonical() {
            return 0;
        }
        self.mixed_seed(salt) as usize
    }

    pub(crate) fn random_index(self, len: usize, salt: u64) -> usize {
        if len <= 1 {
            return 0;
        }
        (self.mixed_seed(salt) as usize) % len
    }

    pub(crate) fn random_stride(self, len: usize, salt: u64) -> usize {
        if len <= 1 {
            return 1;
        }
        let mut stride = (self.mixed_seed(salt) as usize % (len - 1)) + 1;
        while gcd(stride, len) != 1 {
            stride = if stride == len - 1 { 1 } else { stride + 1 };
        }
        stride
    }

    pub(crate) fn random_seed(self, salt: u64) -> u64 {
        self.mixed_seed(salt)
    }

    pub(crate) fn selection_index(self, offset: usize, len: usize, salt: u64) -> usize {
        debug_assert!(offset < len);
        match self.selection_order {
            SelectionOrder::Original | SelectionOrder::Sorted | SelectionOrder::Probabilistic => {
                offset
            }
            SelectionOrder::Random => self.random_index(
                len,
                salt ^ (offset as u64).wrapping_mul(0xD1B5_4A32_D192_ED03),
            ),
            SelectionOrder::Shuffled => {
                let start = self.random_index(len, salt);
                let stride = self.random_stride(len, salt ^ 0xA24B_AED4_963E_E407);
                (start + offset * stride) % len
            }
        }
    }

    /// Orders an outer source dimension without revisiting an expensive row.
    /// `Random` remains with-replacement at the candidate coordinate level,
    /// while source rows are traversed once in a seeded permutation.
    pub(crate) fn selection_index_without_replacement(
        self,
        offset: usize,
        len: usize,
        salt: u64,
    ) -> usize {
        debug_assert!(offset < len);
        match self.selection_order {
            SelectionOrder::Original | SelectionOrder::Sorted | SelectionOrder::Probabilistic => {
                offset
            }
            SelectionOrder::Random | SelectionOrder::Shuffled => {
                let start = self.random_index(len, salt);
                let stride = self.random_stride(len, salt ^ 0xA24B_AED4_963E_E407);
                (start + offset * stride) % len
            }
        }
    }

    pub(crate) fn apply_selection_order<T: Clone>(self, values: &mut [T], salt: u64) {
        if self.is_canonical() {
            return;
        }
        let canonical = values.to_vec();
        for (offset, value) in values.iter_mut().enumerate() {
            *value = canonical[self.selection_index(offset, canonical.len(), salt)].clone();
        }
    }

    /// In-place gather for position/value slices without the canonical copy:
    /// the seeded permutation is walked as one rotation per cycle over the
    /// slice. The resulting order is exactly the `apply_selection_order`
    /// rotation.
    #[cfg(test)]
    pub(crate) fn apply_selection_order_usize(self, values: &mut [usize], salt: u64) {
        if self.is_canonical() {
            return;
        }
        let len = values.len();
        if len <= 1 {
            return;
        }
        if !matches!(
            self.selection_order,
            SelectionOrder::Shuffled
        ) {
            // Random selects coordinates WITH REPLACEMENT, so the gather is
            // not a permutation and must read a fixed source snapshot.
            let canonical = values.to_vec();
            for (offset, value) in values.iter_mut().enumerate() {
                *value = canonical[self.selection_index(offset, len, salt)];
            }
            return;
        }
        // Shuffled is a source permutation: the gather new[i] = old[sel(i)]
        // applies by cycling along the inverse chain new[inv[j]] = old[j].
        let mut inverse = vec![0usize; len];
        for offset in 0..len {
            inverse[self.selection_index(offset, len, salt)] = offset;
        }
        let mut visited = vec![false; len];
        for start in 0..len {
            if visited[start] {
                continue;
            }
            let mut i = start;
            let mut carry = values[start];
            loop {
                visited[i] = true;
                let destination = inverse[i];
                let taken = std::mem::replace(&mut values[destination], carry);
                carry = taken;
                i = destination;
                if visited[i] {
                    break;
                }
            }
        }
    }

    pub(crate) fn apply_selection_order_without_replacement<T: Clone>(
        self,
        values: &mut [T],
        salt: u64,
    ) {
        if self.is_canonical() {
            return;
        }
        let canonical = values.to_vec();
        for (offset, value) in values.iter_mut().enumerate() {
            *value = canonical
                [self.selection_index_without_replacement(offset, canonical.len(), salt)]
                .clone();
        }
    }

    pub(crate) const fn is_canonical(self) -> bool {
        matches!(
            self.selection_order,
            SelectionOrder::Original | SelectionOrder::Sorted | SelectionOrder::Probabilistic
        )
    }

    fn mixed_seed(self, salt: u64) -> u64 {
        splitmix64(self.step_seed ^ self.step_index.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt)
    }
}

impl Default for MoveStreamContext {
    fn default() -> Self {
        Self::new(0, 0, None)
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn gcd(mut left: usize, mut right: usize) -> usize {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

impl<'a, S, M, C> Iterator for MoveSelectorIter<'a, S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    type Item = M;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next_owned_candidate(self.solution)
    }
}

/// A zero-erasure move selector that yields stable candidate indices plus borrowable
/// move views. Ownership is transferred only via `take_candidate`.
pub trait MoveSelector<S: PlanningSolution, M: Move<S>>: Send + Debug {
    type Cursor<'a>: MoveCursor<S, M> + 'a
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a>;

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        _context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        self.open_cursor(score_director)
    }

    /// Validates director-dependent selector configuration without producing candidates.
    ///
    /// Composite selectors call this before deferring a child cursor so deterministic
    /// configuration errors retain their cursor-open timing.
    fn validate_cursor<D: Director<S>>(&self, _score_director: &D) {}

    fn iter_moves<'a, 'd, D: Director<S>>(
        &'a self,
        score_director: &'d D,
    ) -> MoveSelectorIter<'d, S, M, Self::Cursor<'a>> {
        MoveSelectorIter::new(
            self.open_cursor(score_director),
            score_director.working_solution(),
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;

    fn is_never_ending(&self) -> bool {
        false
    }
}
