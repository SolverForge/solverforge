use super::*;

pub(super) struct CartesianRow<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    left: M,
    right_moves: CandidateStore<S, M>,
    live_pairs: usize,
}

impl<S, M> CartesianRow<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Replaces the row's left move and clears its candidate store; the store
    /// keeps its backing capacity so repeated opens do not re-grow it.
    fn replace_left(&mut self, left: M) -> M {
        self.right_moves.reset();
        self.live_pairs = 0;
        std::mem::replace(&mut self.left, left)
    }
}

struct ActiveCartesianRow<C, U> {
    row_index: usize,
    right_cursor: C,
    left_undo: U,
    left_signature: MoveTabuSignature,
}

pub(super) struct CartesianPair {
    row_index: usize,
    right_id: CandidateId,
    entity_indices: SmallVec<[usize; 8]>,
    pub(super) tabu_signature: MoveTabuSignature,
}

/// Per-selector scratch pool. Rows and pair slots retain their backing
/// capacities across cursor opens; each open drains the pool and the cursor
/// returns the buffers on drop. Rows are reset in place (left move replaced,
/// right store cleared with capacity kept, live_pairs zeroed) so no stale move
/// state survives into the next open.
pub(super) struct CartesianScratch<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(super) rows: Vec<Option<CartesianRow<S, M>>>,
    pub(super) pairs: Vec<Option<CartesianPair>>,
}

impl<S, M> CartesianScratch<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(super) fn new() -> Self {
        Self {
            rows: Vec::new(),
            pairs: Vec::new(),
        }
    }

    pub(super) fn reset(&mut self) {
        for row in self.rows.iter_mut().flatten() {
            row.right_moves.reset();
            row.live_pairs = 0;
        }
        self.pairs.clear();
    }
}

pub(crate) struct CartesianProductCursor<'a, S, M, LeftCursor, Right>
where
    S: PlanningSolution,
    M: Move<S>,
    LeftCursor: MoveCursor<S, M>,
    Right: MoveSelector<S, M>,
{
    require_hard_improvement: bool,
    left_cursor: LeftCursor,
    right_selector: &'a Right,
    rows: Vec<Option<CartesianRow<S, M>>>,
    /* Number of rows created by THIS open. The rows vec is a slab that
    outlives the open through the scratch pool, so indexing must use this
    counter, not rows.len(). */
    live_rows: usize,
    active_row: Option<ActiveCartesianRow<Right::Cursor<'a>, M::Undo>>,
    pairs: Vec<Option<CartesianPair>>,
    context: MoveStreamContext,
    scratch: Option<&'a Mutex<CartesianScratch<S, M>>>,
    _phantom: PhantomData<fn() -> S>,
}

impl<'a, S, M, LeftCursor, Right> Drop for CartesianProductCursor<'a, S, M, LeftCursor, Right>
where
    S: PlanningSolution,
    M: Move<S>,
    LeftCursor: MoveCursor<S, M>,
    Right: MoveSelector<S, M>,
{
    /// Returns the rows and pair slots to the selector's scratch pool with
    /// their backing capacities intact, so the next open_cursor() does not
    /// re-grow the vectors.
    fn drop(&mut self) {
        if let Some(pool) = self.scratch {
            if let Ok(mut pool) = pool.lock() {
                std::mem::swap(&mut pool.rows, &mut self.rows);
                std::mem::swap(&mut pool.pairs, &mut self.pairs);
            }
        }
    }
}

impl<'a, S, M, LeftCursor, Right> CartesianProductCursor<'a, S, M, LeftCursor, Right>
where
    S: PlanningSolution,
    M: Move<S>,
    LeftCursor: MoveCursor<S, M>,
    Right: MoveSelector<S, M>,
{
    pub(super) fn new(
        require_hard_improvement: bool,
        left_cursor: LeftCursor,
        right_selector: &'a Right,
        context: MoveStreamContext,
        rows: Vec<Option<CartesianRow<S, M>>>,
        pairs: Vec<Option<CartesianPair>>,
        scratch: &'a Mutex<CartesianScratch<S, M>>,
    ) -> Self {
        Self {
            require_hard_improvement,
            left_cursor,
            right_selector,
            rows,
            live_rows: 0,
            active_row: None,
            pairs,
            context,
            scratch: Some(scratch),
            _phantom: PhantomData,
        }
    }

    fn build_candidate(&self, pair_id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let pair = self.pairs.get(pair_id.index())?.as_ref()?;
        let row = self.rows.get(pair.row_index)?.as_ref()?;
        let left = &row.left;
        let right = match row.right_moves.candidate(pair.right_id)? {
            MoveCandidateRef::Borrowed(mov) => mov,
            MoveCandidateRef::Sequential(_) => return None,
        };
        let variable_name = if left.variable_name() == right.variable_name() {
            left.variable_name()
        } else {
            "cartesian_product"
        };
        Some(MoveCandidateRef::Sequential(
            SequentialCompositeMoveRef::new(
                left,
                right,
                left.descriptor_index(),
                pair.entity_indices.as_slice(),
                variable_name,
                &pair.tabu_signature,
                self.require_hard_improvement,
            ),
        ))
    }

    fn release_row_if_unused(&mut self, row_index: usize) {
        let should_release = self
            .rows
            .get(row_index)
            .and_then(Option::as_ref)
            .is_some_and(|row| {
                self.active_row
                    .as_ref()
                    .is_none_or(|active| active.row_index != row_index)
                    && row.live_pairs == 0
            });
        if should_release {
            let _row = self.rows[row_index]
                .take()
                .expect("unused cartesian row must remain live");
        }
    }
}

impl<S, M, LeftCursor, Right> MoveCursor<S, M>
    for CartesianProductCursor<'_, S, M, LeftCursor, Right>
where
    S: PlanningSolution,
    M: Move<S>,
    LeftCursor: MoveCursor<S, M>,
    Right: MoveSelector<S, M>,
{
    fn next_candidate(&mut self, _solution: &S) -> Option<CandidateId> {
        panic!(
            "cartesian product cursors must be pulled through next_candidate_with_director(); the left preview needs the live working director"
        );
    }

    fn next_candidate_with_director<D: Director<S>>(
        &mut self,
        director: &mut D,
    ) -> Option<CandidateId> {
        loop {
            if self.active_row.is_some() {
                let inspected = {
                    let active = self
                        .active_row
                        .as_mut()
                        .expect("active cartesian row must remain live");
                    let row = self.rows[active.row_index]
                        .as_mut()
                        .expect("active cartesian row must remain live");
                    let left = &row.left;
                    let left_signature = &active.left_signature;
                    let inspected = loop {
                        let Some(right_id) = active
                            .right_cursor
                            .next_candidate(director.working_solution())
                        else {
                            break None;
                        };
                        let MoveCandidateRef::Borrowed(right) = active
                            .right_cursor
                            .candidate(right_id)
                            .expect("right cartesian candidate must remain borrowable")
                        else {
                            panic!("nested cartesian right candidates are not supported")
                        };
                        let signature = right.tabu_signature(director);
                        let doable = right.is_doable_with_working_preview(director);
                        // The combined entity indices are built only for candidates
                        // that pass the preview gate; rejected candidates allocate
                        // nothing on this path.
                        let combined = if doable {
                            Some(combined_entity_indices(
                                left.entity_indices(),
                                right.entity_indices(),
                            ))
                        } else {
                            None
                        };
                        if let Some(entity_indices) = combined {
                            let tabu_signature = compose_sequential_tabu_signature(
                                "cartesian_product",
                                left_signature,
                                &signature,
                            );
                            let moved = (
                                active.right_cursor.take_candidate(right_id),
                                (entity_indices, tabu_signature),
                            );
                            break Some(moved);
                        }
                        assert!(active.right_cursor.release_candidate(right_id));
                    };
                    inspected
                };
                if let Some((right, (entity_indices, tabu_signature))) = inspected {
                    let row_index = self
                        .active_row
                        .as_ref()
                        .expect("active cartesian row must remain live")
                        .row_index;
                    let right_id = self.rows[row_index]
                        .as_mut()
                        .expect("active cartesian row must remain live")
                        .right_moves
                        .push(right);
                    let pair_id = CandidateId::new(self.pairs.len());
                    self.pairs.push(Some(CartesianPair {
                        row_index,
                        right_id,
                        entity_indices,
                        tabu_signature,
                    }));
                    self.rows[row_index]
                        .as_mut()
                        .expect("active cartesian row must remain live")
                        .live_pairs += 1;
                    return Some(pair_id);
                }

                let active = self
                    .active_row
                    .take()
                    .expect("active cartesian row must remain live");
                let row_index = active.row_index;
                self.rows[row_index]
                    .as_ref()
                    .expect("active cartesian row must remain live")
                    .left
                    .undo_move(director, active.left_undo);
                self.rows[row_index]
                    .as_ref()
                    .expect("active cartesian row must remain live")
                    .left
                    .for_each_affected_entity(&mut |affected| {
                        director.working_solution_mut().update_entity_shadows(
                            affected.descriptor_index,
                            affected.entity_index,
                        );
                    });
                self.release_row_if_unused(row_index);
            }

            let left_id = self
                .left_cursor
                .next_candidate(director.working_solution())?;
            let left = self
                .left_cursor
                .candidate(left_id)
                .expect("left cartesian candidate must remain valid");
            let left = match left {
                MoveCandidateRef::Borrowed(mov) => mov,
                MoveCandidateRef::Sequential(_) => {
                    panic!("nested cartesian left candidates are not supported")
                }
            };
            if !left.is_doable_with_working_preview(director) {
                assert!(self.left_cursor.release_candidate(left_id));
                continue;
            }
            let left_signature = left.tabu_signature(director).into_owned();
            let left = self.left_cursor.take_candidate(left_id);
            let left_undo = left.do_move(director);
            // Preview contract (mirrors SequentialPreviewDirector): the
            // previewed left leg refreshes shadows for the rows it touched so
            // shadow-backed right children build against the previewed state.
            left.for_each_affected_entity(&mut |affected| {
                director
                    .working_solution_mut()
                    .update_entity_shadows(affected.descriptor_index, affected.entity_index);
            });
            let right_cursor = self
                .right_selector
                .open_cursor_with_context(&*director, self.context);
            let row_index = self.live_rows;
            self.live_rows += 1;
            let row = if let Some(slot) = self.rows.get_mut(row_index) {
                // Pooled slot from a previous open: reuse its candidate-store
                // capacity and drop the stale left move. Released rows come
                // back as None slots and rebuild fresh.
                match slot.take() {
                    Some(mut reused) => {
                        let _stale_left = reused.replace_left(left);
                        reused
                    }
                    None => CartesianRow {
                        left,
                        right_moves: CandidateStore::new(),
                        live_pairs: 0,
                    },
                }
            } else {
                CartesianRow {
                    left,
                    right_moves: CandidateStore::new(),
                    live_pairs: 0,
                }
            };
            if row_index < self.rows.len() {
                self.rows[row_index] = Some(row);
            } else {
                self.rows.push(Some(row));
            }
            self.active_row = Some(ActiveCartesianRow {
                row_index,
                right_cursor,
                left_undo,
                left_signature,
            });
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.build_candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        let _ = index;
        panic!(
            "cartesian product cursors expose borrowed sequential candidates; apply the selected candidate before dropping the cursor",
        );
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        let pair = self
            .pairs
            .get_mut(index.index())
            .and_then(Option::take)
            .expect("selected cartesian pair must remain live");
        let mut row = self.rows[pair.row_index]
            .take()
            .expect("selected cartesian row must remain live");
        if self
            .active_row
            .as_ref()
            .is_some_and(|active| active.row_index == pair.row_index)
        {
            self.active_row = None;
        }
        for sibling in &mut self.pairs {
            if sibling
                .as_ref()
                .is_some_and(|sibling| sibling.row_index == pair.row_index)
            {
                let sibling = sibling
                    .take()
                    .expect("selected cartesian sibling must remain live");
                assert!(row.right_moves.release_candidate(sibling.right_id));
            }
        }
        let left = row.left;
        let right = row.right_moves.take_candidate(pair.right_id);
        let variable_name = if left.variable_name() == right.variable_name() {
            left.variable_name().to_string()
        } else {
            "cartesian_product".to_string()
        };
        let descriptor_index = left.descriptor_index();
        let selected = SequentialCompositeMove::new(
            left,
            right,
            descriptor_index,
            pair.entity_indices,
            variable_name,
            pair.tabu_signature,
        )
        .with_require_hard_improvement(self.require_hard_improvement);
        selected.do_move(score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        let Some(pair) = self.pairs.get_mut(index.index()).and_then(Option::take) else {
            return false;
        };
        let row = self.rows[pair.row_index]
            .as_mut()
            .expect("released cartesian row must remain live");
        assert!(row.right_moves.release_candidate(pair.right_id));
        row.live_pairs = row
            .live_pairs
            .checked_sub(1)
            .expect("cartesian row live-pair count must remain positive");
        self.release_row_if_unused(pair.row_index);
        true
    }
}

fn combined_entity_indices(left: &[usize], right: &[usize]) -> SmallVec<[usize; 8]> {
    let mut entity_indices = SmallVec::<[usize; 8]>::new();
    for &entity_index in left.iter().chain(right.iter()) {
        if !entity_indices.contains(&entity_index) {
            entity_indices.push(entity_index);
        }
    }
    entity_indices
}
