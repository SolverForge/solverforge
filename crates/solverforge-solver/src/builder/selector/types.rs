type LeafSelector<S, V, DM, IDM> =
    VecUnionSelector<S, NeighborhoodMove<S, V>, NeighborhoodLeaf<S, V, DM, IDM>>;

pub enum NeighborhoodMove<S, V> {
    Scalar(ScalarMoveUnion<S, usize>),
    List(ListMoveUnion<S, V>),
    Composite(SequentialCompositeMove<S, NeighborhoodMove<S, V>>),
}

impl<S, V> Clone for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::Scalar(m) => Self::Scalar(m.clone()),
            Self::List(m) => Self::List(m.clone()),
            Self::Composite(m) => Self::Composite(m.clone()),
        }
    }
}

impl<S, V> Debug for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(m) => write!(f, "NeighborhoodMove::Scalar({m:?})"),
            Self::List(m) => write!(f, "NeighborhoodMove::List({m:?})"),
            Self::Composite(m) => write!(f, "NeighborhoodMove::Composite({m:?})"),
        }
    }
}

impl<S, V> Move<S> for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Scalar(m) => m.is_doable(score_director),
            Self::List(m) => m.is_doable(score_director),
            Self::Composite(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: solverforge_scoring::Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Scalar(m) => m.do_move(score_director),
            Self::List(m) => m.do_move(score_director),
            Self::Composite(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Scalar(m) => m.descriptor_index(),
            Self::List(m) => m.descriptor_index(),
            Self::Composite(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Scalar(m) => m.entity_indices(),
            Self::List(m) => m.entity_indices(),
            Self::Composite(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Scalar(m) => m.variable_name(),
            Self::List(m) => m.variable_name(),
            Self::Composite(m) => m.variable_name(),
        }
    }

    fn tabu_signature<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
    ) -> MoveTabuSignature {
        match self {
            Self::Scalar(m) => m.tabu_signature(score_director),
            Self::List(m) => m.tabu_signature(score_director),
            Self::Composite(m) => m.tabu_signature(score_director),
        }
    }
}

pub enum NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Scalar(ScalarLeafSelector<S>),
    List(ListLeafSelector<S, V, DM, IDM>),
    ConflictRepair(ConflictRepairSelector<S>),
}

fn wrap_scalar_neighborhood_move<S, V>(mov: ScalarMoveUnion<S, usize>) -> NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    NeighborhoodMove::Scalar(mov)
}

fn wrap_list_neighborhood_move<S, V>(mov: ListMoveUnion<S, V>) -> NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    NeighborhoodMove::List(mov)
}

pub enum NeighborhoodLeafCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    Scalar(MappedMoveCursor<
        S,
        ScalarMoveUnion<S, usize>,
        NeighborhoodMove<S, V>,
        <ScalarLeafSelector<S> as MoveSelector<S, ScalarMoveUnion<S, usize>>>::Cursor<'a>,
        fn(ScalarMoveUnion<S, usize>) -> NeighborhoodMove<S, V>,
    >),
    List(MappedMoveCursor<
        S,
        ListMoveUnion<S, V>,
        NeighborhoodMove<S, V>,
        <ListLeafSelector<S, V, DM, IDM> as MoveSelector<S, ListMoveUnion<S, V>>>::Cursor<'a>,
        fn(ListMoveUnion<S, V>) -> NeighborhoodMove<S, V>,
    >),
    ConflictRepair(MappedMoveCursor<
        S,
        ScalarMoveUnion<S, usize>,
        NeighborhoodMove<S, V>,
        <ConflictRepairSelector<S> as MoveSelector<S, ScalarMoveUnion<S, usize>>>::Cursor<'a>,
        fn(ScalarMoveUnion<S, usize>) -> NeighborhoodMove<S, V>,
    >),
}

impl<S, V, DM, IDM> MoveCursor<S, NeighborhoodMove<S, V>>
    for NeighborhoodLeafCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Scalar(cursor) => cursor.next_candidate(),
            Self::List(cursor) => cursor.next_candidate(),
            Self::ConflictRepair(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>> {
        match self {
            Self::Scalar(cursor) => cursor.candidate(index),
            Self::List(cursor) => cursor.candidate(index),
            Self::ConflictRepair(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> NeighborhoodMove<S, V> {
        match self {
            Self::Scalar(cursor) => cursor.take_candidate(index),
            Self::List(cursor) => cursor.take_candidate(index),
            Self::ConflictRepair(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Scalar(cursor) => cursor.selector_index(index),
            Self::List(cursor) => cursor.selector_index(index),
            Self::ConflictRepair(cursor) => cursor.selector_index(index),
        }
    }
}

impl<S, V, DM, IDM> Debug for NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(selector) => write!(f, "NeighborhoodLeaf::Scalar({selector:?})"),
            Self::List(selector) => write!(f, "NeighborhoodLeaf::List({selector:?})"),
            Self::ConflictRepair(selector) => {
                write!(f, "NeighborhoodLeaf::ConflictRepair({selector:?})")
            }
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>> for NeighborhoodLeaf<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = NeighborhoodLeafCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Scalar(selector) => NeighborhoodLeafCursor::Scalar(MappedMoveCursor::new(
                selector.open_cursor(score_director),
                wrap_scalar_neighborhood_move::<S, V>,
            )),
            Self::List(selector) => NeighborhoodLeafCursor::List(MappedMoveCursor::new(
                selector.open_cursor(score_director),
                wrap_list_neighborhood_move::<S, V>,
            )),
            Self::ConflictRepair(selector) => {
                NeighborhoodLeafCursor::ConflictRepair(MappedMoveCursor::new(
                    selector.open_cursor(score_director),
                    wrap_scalar_neighborhood_move::<S, V>,
                ))
            }
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Scalar(selector) => selector.size(score_director),
            Self::List(selector) => selector.size(score_director),
            Self::ConflictRepair(selector) => selector.size(score_director),
        }
    }

    fn append_moves<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<NeighborhoodMove<S, V>>,
    ) {
        let mut cursor = self.open_cursor(score_director);
        for id in collect_cursor_indices::<S, NeighborhoodMove<S, V>, _>(&mut cursor) {
            arena.push(cursor.take_candidate(id));
        }
    }
}

pub enum Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Flat(LeafSelector<S, V, DM, IDM>),
    Limited {
        selector: LeafSelector<S, V, DM, IDM>,
        selected_count_limit: usize,
    },
    Cartesian(CartesianNeighborhoodSelector<S, V, DM, IDM>),
}

pub enum CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    Flat(LeafSelector<S, V, DM, IDM>),
    Limited {
        selector: LeafSelector<S, V, DM, IDM>,
        selected_count_limit: usize,
    },
}

type CartesianNeighborhoodSelector<S, V, DM, IDM> = CartesianProductSelector<
    S,
    NeighborhoodMove<S, V>,
    CartesianChildSelector<S, V, DM, IDM>,
    CartesianChildSelector<S, V, DM, IDM>,
>;

pub enum CartesianChildCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    Flat(<LeafSelector<S, V, DM, IDM> as MoveSelector<S, NeighborhoodMove<S, V>>>::Cursor<'a>),
    Limited(
        LimitedMoveCursor<
            <LeafSelector<S, V, DM, IDM> as MoveSelector<S, NeighborhoodMove<S, V>>>::Cursor<'a>,
        >,
    ),
}

impl<S, V, DM, IDM> MoveCursor<S, NeighborhoodMove<S, V>>
    for CartesianChildCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Flat(cursor) => cursor.next_candidate(),
            Self::Limited(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>> {
        match self {
            Self::Flat(cursor) => cursor.candidate(index),
            Self::Limited(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> NeighborhoodMove<S, V> {
        match self {
            Self::Flat(cursor) => cursor.take_candidate(index),
            Self::Limited(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Flat(cursor) => cursor.selector_index(index),
            Self::Limited(cursor) => cursor.selector_index(index),
        }
    }
}

pub enum NeighborhoodCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    Flat(<LeafSelector<S, V, DM, IDM> as MoveSelector<S, NeighborhoodMove<S, V>>>::Cursor<'a>),
    Limited(
        LimitedMoveCursor<
            <LeafSelector<S, V, DM, IDM> as MoveSelector<S, NeighborhoodMove<S, V>>>::Cursor<'a>,
        >,
    ),
    Cartesian(CartesianProductCursor<S, NeighborhoodMove<S, V>>),
}

impl<S, V, DM, IDM> MoveCursor<S, NeighborhoodMove<S, V>>
    for NeighborhoodCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Flat(cursor) => cursor.next_candidate(),
            Self::Limited(cursor) => cursor.next_candidate(),
            Self::Cartesian(cursor) => cursor.next_candidate(),
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, NeighborhoodMove<S, V>>> {
        match self {
            Self::Flat(cursor) => cursor.candidate(index),
            Self::Limited(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> NeighborhoodMove<S, V> {
        match self {
            Self::Flat(cursor) => cursor.take_candidate(index),
            Self::Limited(cursor) => cursor.take_candidate(index),
            Self::Cartesian(cursor) => cursor.take_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Flat(cursor) => cursor.selector_index(index),
            Self::Limited(cursor) => cursor.selector_index(index),
            Self::Cartesian(cursor) => cursor.selector_index(index),
        }
    }
}

impl<S, V, DM, IDM> Debug for Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(selector) => write!(f, "Neighborhood::Flat({selector:?})"),
            Self::Limited {
                selector,
                selected_count_limit,
            } => f
                .debug_struct("Neighborhood::Limited")
                .field("selector", selector)
                .field("selected_count_limit", selected_count_limit)
                .finish(),
            Self::Cartesian(selector) => write!(f, "Neighborhood::Cartesian({selector:?})"),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>> for Neighborhood<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = NeighborhoodCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Flat(selector) => NeighborhoodCursor::Flat(selector.open_cursor(score_director)),
            Self::Limited {
                selector,
                selected_count_limit,
            } => NeighborhoodCursor::Limited(LimitedMoveCursor::new(
                selector.open_cursor(score_director),
                *selected_count_limit,
            )),
            Self::Cartesian(selector) => {
                NeighborhoodCursor::Cartesian(selector.open_cursor(score_director))
            }
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Flat(selector) => selector.size(score_director),
            Self::Limited {
                selector,
                selected_count_limit,
            } => selector.size(score_director).min(*selected_count_limit),
            Self::Cartesian(selector) => selector.size(score_director),
        }
    }
}

impl<S, V, DM, IDM> Debug for CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(selector) => write!(f, "CartesianChildSelector::Flat({selector:?})"),
            Self::Limited {
                selector,
                selected_count_limit,
            } => f
                .debug_struct("CartesianChildSelector::Limited")
                .field("selector", selector)
                .field("selected_count_limit", selected_count_limit)
                .finish(),
        }
    }
}

impl<S, V, DM, IDM> MoveSelector<S, NeighborhoodMove<S, V>>
    for CartesianChildSelector<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    type Cursor<'a>
        = CartesianChildCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<S>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        match self {
            Self::Flat(selector) => {
                CartesianChildCursor::Flat(selector.open_cursor(score_director))
            }
            Self::Limited {
                selector,
                selected_count_limit,
            } => CartesianChildCursor::Limited(LimitedMoveCursor::new(
                selector.open_cursor(score_director),
                *selected_count_limit,
            )),
        }
    }

    fn size<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Flat(selector) => selector.size(score_director),
            Self::Limited {
                selector,
                selected_count_limit,
            } => selector.size(score_director).min(*selected_count_limit),
        }
    }
}
