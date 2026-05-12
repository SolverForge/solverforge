pub enum ScalarLeafSelector<S> {
    Change(ScalarChangeSelector<S>),
    Swap(ScalarSwapSelector<S>),
    NearbyChange(NearbyChangeLeafSelector<S>),
    NearbySwap(NearbySwapLeafSelector<S>),
    PillarChange(PillarChangeLeafSelector<S>),
    PillarSwap(PillarSwapLeafSelector<S>),
    RuinRecreate(RuinRecreateLeafSelector<S>),
}

fn wrap_scalar_change_move<S>(mov: ChangeMove<S, usize>) -> ScalarMoveUnion<S, usize>
where
    S: PlanningSolution + 'static,
{
    ScalarMoveUnion::Change(mov)
}

pub enum ScalarLeafCursor<'a, S>
where
    S: PlanningSolution + 'static,
{
    Change(
        MappedMoveCursor<
            S,
            ChangeMove<S, usize>,
            ScalarMoveUnion<S, usize>,
            <ScalarChangeSelector<S> as MoveSelector<S, ChangeMove<S, usize>>>::Cursor<'a>,
            fn(ChangeMove<S, usize>) -> ScalarMoveUnion<S, usize>,
        >,
    ),
    Swap(SwapLeafCursor<S>),
    NearbyChange(NearbyChangeLeafCursor<S>),
    NearbySwap(NearbySwapLeafCursor<S>),
    Direct(ArenaMoveCursor<S, ScalarMoveUnion<S, usize>>),
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for ScalarLeafCursor<'_, S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Change(cursor) => cursor.next_candidate(),
            Self::Swap(cursor) => cursor.next_candidate(),
            Self::NearbyChange(cursor) => cursor.next_candidate(),
            Self::NearbySwap(cursor) => cursor.next_candidate(),
            Self::Direct(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ScalarMoveUnion<S, usize>>> {
        match self {
            Self::Change(cursor) => cursor.candidate(index),
            Self::Swap(cursor) => cursor.candidate(index),
            Self::NearbyChange(cursor) => cursor.candidate(index),
            Self::NearbySwap(cursor) => cursor.candidate(index),
            Self::Direct(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> ScalarMoveUnion<S, usize> {
        match self {
            Self::Change(cursor) => cursor.take_candidate(index),
            Self::Swap(cursor) => cursor.take_candidate(index),
            Self::NearbyChange(cursor) => cursor.take_candidate(index),
            Self::NearbySwap(cursor) => cursor.take_candidate(index),
            Self::Direct(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Change(cursor) => cursor.selector_index(index),
            Self::Swap(cursor) => cursor.selector_index(index),
            Self::NearbyChange(cursor) => cursor.selector_index(index),
            Self::NearbySwap(cursor) => cursor.selector_index(index),
            Self::Direct(cursor) => cursor.selector_index(index),
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[allow(clippy::large_enum_variant)] // Inline storage keeps selector assembly zero-erasure.
pub enum ScalarSelectorNode<S> {
    Leaf(ScalarLeafSelector<S>),
    Cartesian(ScalarCartesianSelector<S>),
}

pub enum ScalarSelectorCursor<'a, S>
where
    S: PlanningSolution + 'static,
{
    Leaf(ScalarLeafCursor<'a, S>),
    Cartesian(CartesianProductCursor<S, ScalarMoveUnion<S, usize>>),
}

impl<S> MoveCursor<S, ScalarMoveUnion<S, usize>> for ScalarSelectorCursor<'_, S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Leaf(cursor) => cursor.next_candidate(),
            Self::Cartesian(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(
        &self,
        index: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, ScalarMoveUnion<S, usize>>> {
        match self {
            Self::Leaf(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> ScalarMoveUnion<S, usize> {
        match self {
            Self::Leaf(cursor) => cursor.take_candidate(index),
            Self::Cartesian(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Leaf(cursor) => cursor.selector_index(index),
            Self::Cartesian(cursor) => cursor.selector_index(index),
        }
    }
}

impl<S> Debug for ScalarSelectorNode<S>
where
    S: PlanningSolution + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leaf(selector) => selector.fmt(f),
            Self::Cartesian(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for ScalarSelectorNode<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = ScalarSelectorCursor<'a, S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Leaf(selector) => {
                ScalarSelectorCursor::Leaf(selector.open_cursor_with_context(score_director, context))
            }
            Self::Cartesian(selector) => {
                ScalarSelectorCursor::Cartesian(selector.open_cursor_with_context(score_director, context))
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Leaf(selector) => selector.size(score_director),
            Self::Cartesian(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        match self {
            Self::Leaf(selector) => selector.append_moves(score_director, arena),
            Self::Cartesian(selector) => selector.append_moves(score_director, arena),
        }
    }
}

impl<S> Debug for ScalarLeafSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(selector) => selector.fmt(f),
            Self::Swap(selector) => selector.fmt(f),
            Self::NearbyChange(selector) => selector.fmt(f),
            Self::NearbySwap(selector) => selector.fmt(f),
            Self::PillarChange(selector) => selector.fmt(f),
            Self::PillarSwap(selector) => selector.fmt(f),
            Self::RuinRecreate(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, ScalarMoveUnion<S, usize>> for ScalarLeafSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = ScalarLeafCursor<'a, S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Change(selector) => ScalarLeafCursor::Change(MappedMoveCursor::new(
                selector.open_cursor_with_context(score_director, context),
                wrap_scalar_change_move::<S>,
            )),
            Self::Swap(selector) => {
                ScalarLeafCursor::Swap(selector.open_cursor_with_context(score_director, context))
            }
            Self::NearbyChange(selector) => ScalarLeafCursor::NearbyChange(
                selector.open_cursor_with_context(score_director, context),
            ),
            Self::NearbySwap(selector) => ScalarLeafCursor::NearbySwap(
                selector.open_cursor_with_context(score_director, context),
            ),
            Self::PillarChange(selector) => {
                ScalarLeafCursor::Direct(selector.open_cursor_with_context(score_director, context))
            }
            Self::PillarSwap(selector) => {
                ScalarLeafCursor::Direct(selector.open_cursor_with_context(score_director, context))
            }
            Self::RuinRecreate(selector) => {
                ScalarLeafCursor::Direct(selector.open_cursor_with_context(score_director, context))
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Change(selector) => selector.size(score_director),
            Self::Swap(selector) => selector.size(score_director),
            Self::NearbyChange(selector) => selector.size(score_director),
            Self::NearbySwap(selector) => selector.size(score_director),
            Self::PillarChange(selector) => selector.size(score_director),
            Self::PillarSwap(selector) => selector.size(score_director),
            Self::RuinRecreate(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<ScalarMoveUnion<S, usize>>,
    ) {
        match self {
            Self::Change(selector) => {
                let mut cursor = MappedMoveCursor::new(
                    selector.open_cursor(score_director),
                    wrap_scalar_change_move::<S>,
                );
                for id in
                    collect_cursor_indices::<S, ScalarMoveUnion<S, usize>, _>(&mut cursor)
                {
                    arena.push(cursor.take_candidate(id));
                }
            }
            Self::Swap(selector) => selector.append_moves(score_director, arena),
            Self::NearbyChange(selector) => selector.append_moves(score_director, arena),
            Self::NearbySwap(selector) => selector.append_moves(score_director, arena),
            Self::PillarChange(selector) => selector.append_moves(score_director, arena),
            Self::PillarSwap(selector) => selector.append_moves(score_director, arena),
            Self::RuinRecreate(selector) => selector.append_moves(score_director, arena),
        }
    }
}

pub(super) fn build_scalar_flat_selector<S>(
    config: Option<&MoveSelectorConfig>,
    scalar_variables: &[ScalarVariableSlot<S>],
    random_seed: Option<u64>,
) -> ScalarFlatSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    let mut leaves = Vec::new();
    collect_scalar_leaf_selectors(config, scalar_variables, random_seed, &mut leaves);
    assert!(
        !leaves.is_empty(),
        "move selector configuration produced no scalar neighborhoods"
    );
    let selection_order = match config {
        Some(MoveSelectorConfig::UnionMoveSelector(union)) => union.selection_order,
        _ => solverforge_config::UnionSelectionOrder::Sequential,
    };
    VecUnionSelector::with_selection_order(leaves, selection_order)
}
