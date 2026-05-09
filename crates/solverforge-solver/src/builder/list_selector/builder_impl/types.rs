type ListFlatSelector<S, V, DM, IDM> =
    VecUnionSelector<S, ListMoveUnion<S, V>, ListLeafSelector<S, V, DM, IDM>>;
#[cfg_attr(not(test), allow(dead_code))]
type ListCartesianSelector<S, V, DM, IDM> = CartesianProductSelector<
    S,
    ListMoveUnion<S, V>,
    ListFlatSelector<S, V, DM, IDM>,
    ListFlatSelector<S, V, DM, IDM>,
>;

#[cfg_attr(not(test), allow(dead_code))]
#[allow(clippy::large_enum_variant)] // Inline storage keeps selector assembly zero-erasure.
pub enum ListSelectorNode<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Leaf(ListLeafSelector<S, V, DM, IDM>),
    Cartesian(ListCartesianSelector<S, V, DM, IDM>),
}

pub enum ListSelectorCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    Leaf(ListLeafCursor<'a, S, V, DM, IDM>),
    Cartesian(CartesianProductCursor<S, ListMoveUnion<S, V>>),
}

impl<S, V, DM, IDM> MoveCursor<S, ListMoveUnion<S, V>>
    for ListSelectorCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Leaf(cursor) => cursor.next_candidate(),
            Self::Cartesian(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, ListMoveUnion<S, V>>> {
        match self {
            Self::Leaf(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> ListMoveUnion<S, V> {
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

impl<S, V, DM, IDM> Debug for ListSelectorNode<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Leaf(selector) => selector.fmt(f),
            Self::Cartesian(selector) => selector.fmt(f),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, ListMoveUnion<S, V>> for ListSelectorNode<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = ListSelectorCursor<'a, S, V, DM, IDM>
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
                ListSelectorCursor::Leaf(selector.open_cursor_with_context(score_director, context))
            }
            Self::Cartesian(selector) => {
                ListSelectorCursor::Cartesian(selector.open_cursor_with_context(score_director, context))
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
        arena: &mut MoveArena<ListMoveUnion<S, V>>,
    ) {
        match self {
            Self::Leaf(selector) => selector.append_moves(score_director, arena),
            Self::Cartesian(selector) => selector.append_moves(score_director, arena),
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn wrap_list_composite<S, V>(
    mov: SequentialCompositeMove<S, ListMoveUnion<S, V>>,
) -> ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListMoveUnion::Composite(mov)
}
