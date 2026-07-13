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

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorLeafSelector<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorLeafCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        match self {
            Self::Change(selector) => DescriptorLeafCursor::Change(selector.open_cursor(score_director)),
            Self::Swap(selector) => DescriptorLeafCursor::Swap(selector.open_cursor(score_director)),
            Self::NearbyChange(selector) => DescriptorLeafCursor::Change(selector.open_cursor(score_director)),
            Self::NearbySwap(selector) => DescriptorLeafCursor::Swap(selector.open_cursor(score_director)),
            Self::PillarChange(selector) => {
                DescriptorLeafCursor::PillarChange(selector.open_cursor(score_director))
            }
            Self::PillarSwap(selector) => {
                DescriptorLeafCursor::PillarSwap(selector.open_cursor(score_director))
            }
            Self::RuinRecreate(selector) => {
                DescriptorLeafCursor::RuinRecreate(selector.open_cursor(score_director))
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
}

#[allow(clippy::large_enum_variant)]
pub enum DescriptorLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    Change(DescriptorChangeMoveCursor<S>),
    Swap(DescriptorSwapMoveCursor<S>),
    PillarChange(DescriptorPillarChangeMoveCursor<S>),
    PillarSwap(DescriptorPillarSwapMoveCursor<S>),
    RuinRecreate(DescriptorRuinRecreateMoveCursor<S>),
}

impl<S> MoveCursor<S, DescriptorMoveUnion<S>> for DescriptorLeafCursor<S>
where
    S: PlanningSolution + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Change(cursor) => cursor.next_candidate(),
            Self::Swap(cursor) => cursor.next_candidate(),
            Self::PillarChange(cursor) => cursor.next_candidate(),
            Self::PillarSwap(cursor) => cursor.next_candidate(),
            Self::RuinRecreate(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, DescriptorMoveUnion<S>>> {
        match self {
            Self::Change(cursor) => cursor.candidate(id),
            Self::Swap(cursor) => cursor.candidate(id),
            Self::PillarChange(cursor) => cursor.candidate(id),
            Self::PillarSwap(cursor) => cursor.candidate(id),
            Self::RuinRecreate(cursor) => cursor.candidate(id),
        }
    }

    fn take_candidate(&mut self, id: CandidateId) -> DescriptorMoveUnion<S> {
        match self {
            Self::Change(cursor) => cursor.take_candidate(id),
            Self::Swap(cursor) => cursor.take_candidate(id),
            Self::PillarChange(cursor) => cursor.take_candidate(id),
            Self::PillarSwap(cursor) => cursor.take_candidate(id),
            Self::RuinRecreate(cursor) => cursor.take_candidate(id),
        }
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        match self {
            Self::Change(cursor) => cursor.release_candidate(id),
            Self::Swap(cursor) => cursor.release_candidate(id),
            Self::PillarChange(cursor) => cursor.release_candidate(id),
            Self::PillarSwap(cursor) => cursor.release_candidate(id),
            Self::RuinRecreate(cursor) => cursor.release_candidate(id),
        }
    }
}

#[allow(clippy::large_enum_variant)] // Inline storage keeps selector assembly zero-erasure.
pub enum DescriptorSelectorNode<S> {
    Leaf(DescriptorLeafSelector<S>),
    Cartesian(DescriptorCartesianSelector<S>),
}

#[allow(clippy::large_enum_variant)]
pub enum DescriptorSelectorCursor<'a, S>
where
    S: PlanningSolution + 'static,
{
    Leaf(DescriptorLeafCursor<S>),
    Cartesian(DescriptorCartesianCursor<'a, S>),
}

impl<S> MoveCursor<S, DescriptorMoveUnion<S>> for DescriptorSelectorCursor<'_, S>
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
    ) -> Option<MoveCandidateRef<'_, S, DescriptorMoveUnion<S>>> {
        match self {
            Self::Leaf(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> DescriptorMoveUnion<S> {
        match self {
            Self::Leaf(cursor) => cursor.take_candidate(index),
            Self::Cartesian(cursor) => cursor.take_candidate(index),
        }
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        match self {
            Self::Leaf(cursor) => cursor.apply_owned_candidate(index, score_director),
            Self::Cartesian(cursor) => cursor.apply_owned_candidate(index, score_director),
        }
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        match self {
            Self::Leaf(cursor) => cursor.release_candidate(index),
            Self::Cartesian(cursor) => cursor.release_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Leaf(cursor) => cursor.selector_index(index),
            Self::Cartesian(cursor) => cursor.selector_index(index),
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

impl<S> MoveSelector<S, DescriptorMoveUnion<S>> for DescriptorSelectorNode<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Cursor<'a>
        = DescriptorSelectorCursor<'a, S>
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
