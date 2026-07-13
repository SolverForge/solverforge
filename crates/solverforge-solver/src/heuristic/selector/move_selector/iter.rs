use solverforge_config::SelectionOrder;

pub struct MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    cursor: C,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
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

impl<S, M, C> MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn new(cursor: C) -> Self {
        Self {
            cursor,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, C> Iterator for MoveSelectorIter<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    type Item = M;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next_owned_candidate()
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

    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> MoveSelectorIter<S, M, Self::Cursor<'a>> {
        MoveSelectorIter::new(self.open_cursor(score_director))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;

    fn append_moves<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>) {
        let mut cursor = self.open_cursor(score_director);
        for id in collect_cursor_indices::<S, M, _>(&mut cursor) {
            arena.push(cursor.take_candidate(id));
        }
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
