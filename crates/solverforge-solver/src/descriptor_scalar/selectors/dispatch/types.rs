pub enum DescriptorLeafSelector<S> {
    Change(DescriptorChangeMoveSelector<S>),
    Swap(DescriptorSwapMoveSelector<S>),
    NearbyChange(DescriptorNearbyChangeMoveSelector<S>),
    NearbySwap(DescriptorNearbySwapMoveSelector<S>),
    PillarChange(DescriptorPillarChangeMoveSelector<S>),
    PillarSwap(DescriptorPillarSwapMoveSelector<S>),
    RuinRecreate(DescriptorRuinRecreateMoveSelector<S>),
}

impl<S> Debug for DescriptorLeafSelector<S>
where
    S: PlanningSolution,
{
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

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorLeafSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        match self {
            Self::Change(selector) => selector.open_cursor(score_director),
            Self::Swap(selector) => selector.open_cursor(score_director),
            Self::NearbyChange(selector) => selector.open_cursor(score_director),
            Self::NearbySwap(selector) => selector.open_cursor(score_director),
            Self::PillarChange(selector) => selector.open_cursor(score_director),
            Self::PillarSwap(selector) => selector.open_cursor(score_director),
            Self::RuinRecreate(selector) => selector.open_cursor(score_director),
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
}

#[allow(clippy::large_enum_variant)] // Inline storage keeps selector assembly zero-erasure.
pub enum DescriptorSelectorNode<S> {
    Leaf(DescriptorLeafSelector<S>),
    Cartesian(DescriptorCartesianSelector<S>),
}

pub enum DescriptorSelectorCursor<S>
where
    S: PlanningSolution + 'static,
{
    Leaf(ArenaMoveCursor<S, DescriptorScalarMoveUnion<S>>),
    Cartesian(CartesianProductCursor<S, DescriptorScalarMoveUnion<S>>),
}

impl<S> MoveCursor<S, DescriptorScalarMoveUnion<S>> for DescriptorSelectorCursor<S>
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
    ) -> Option<MoveCandidateRef<'_, S, DescriptorScalarMoveUnion<S>>> {
        match self {
            Self::Leaf(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> DescriptorScalarMoveUnion<S> {
        match self {
            Self::Leaf(cursor) => cursor.take_candidate(index),
            Self::Cartesian(cursor) => cursor.take_candidate(index),
        }
    }
}

impl<S> Debug for DescriptorSelectorNode<S>
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

impl<S> MoveSelector<S, DescriptorScalarMoveUnion<S>> for DescriptorSelectorNode<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorSelectorCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        match self {
            Self::Leaf(selector) => {
                DescriptorSelectorCursor::Leaf(selector.open_cursor(score_director))
            }
            Self::Cartesian(selector) => {
                DescriptorSelectorCursor::Cartesian(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Leaf(selector) => selector.size(score_director),
            Self::Cartesian(selector) => selector.size(score_director),
        }
    }
}

fn wrap_descriptor_composite<S>(
    mov: crate::heuristic::r#move::SequentialCompositeMove<S, DescriptorScalarMoveUnion<S>>,
) -> DescriptorScalarMoveUnion<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    DescriptorScalarMoveUnion::Composite(mov)
}
