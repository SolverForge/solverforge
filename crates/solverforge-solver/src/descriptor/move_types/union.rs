pub enum DescriptorMoveUnion<S> {
    Change(DescriptorChangeMove<S>),
    Swap(DescriptorSwapMove<S>),
    PillarChange(DescriptorPillarChangeMove<S>),
    PillarSwap(DescriptorPillarSwapMove<S>),
    RuinRecreate(DescriptorRuinRecreateMove<S>),
}

pub enum DescriptorMoveUnionUndo<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    Change(<DescriptorChangeMove<S> as Move<S>>::Undo),
    Swap(<DescriptorSwapMove<S> as Move<S>>::Undo),
    PillarChange(<DescriptorPillarChangeMove<S> as Move<S>>::Undo),
    PillarSwap(<DescriptorPillarSwapMove<S> as Move<S>>::Undo),
    RuinRecreate(<DescriptorRuinRecreateMove<S> as Move<S>>::Undo),
}

impl<S> Clone for DescriptorMoveUnion<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn clone(&self) -> Self {
        match self {
            Self::Change(m) => Self::Change(m.clone()),
            Self::Swap(m) => Self::Swap(m.clone()),
            Self::PillarChange(m) => Self::PillarChange(m.clone()),
            Self::PillarSwap(m) => Self::PillarSwap(m.clone()),
            Self::RuinRecreate(m) => Self::RuinRecreate(m.clone()),
        }
    }
}

impl<S> Debug for DescriptorMoveUnion<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
            Self::PillarChange(m) => m.fmt(f),
            Self::PillarSwap(m) => m.fmt(f),
            Self::RuinRecreate(m) => m.fmt(f),
        }
    }
}

impl<S> Move<S> for DescriptorMoveUnion<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    type Undo = DescriptorMoveUnionUndo<S>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
            Self::PillarChange(m) => m.is_doable(score_director),
            Self::PillarSwap(m) => m.is_doable(score_director),
            Self::RuinRecreate(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        match self {
            Self::Change(m) => DescriptorMoveUnionUndo::Change(m.do_move(score_director)),
            Self::Swap(m) => DescriptorMoveUnionUndo::Swap(m.do_move(score_director)),
            Self::PillarChange(m) => {
                DescriptorMoveUnionUndo::PillarChange(m.do_move(score_director))
            }
            Self::PillarSwap(m) => {
                DescriptorMoveUnionUndo::PillarSwap(m.do_move(score_director))
            }
            Self::RuinRecreate(m) => {
                DescriptorMoveUnionUndo::RuinRecreate(m.do_move(score_director))
            }
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        match (self, undo) {
            (Self::Change(m), DescriptorMoveUnionUndo::Change(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::Swap(m), DescriptorMoveUnionUndo::Swap(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::PillarChange(m), DescriptorMoveUnionUndo::PillarChange(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::PillarSwap(m), DescriptorMoveUnionUndo::PillarSwap(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::RuinRecreate(m), DescriptorMoveUnionUndo::RuinRecreate(undo)) => {
                m.undo_move(score_director, undo)
            }
            _ => panic!("descriptor move undo shape must match move shape"),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Change(m) => m.descriptor_index(),
            Self::Swap(m) => m.descriptor_index(),
            Self::PillarChange(m) => m.descriptor_index(),
            Self::PillarSwap(m) => m.descriptor_index(),
            Self::RuinRecreate(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Change(m) => m.entity_indices(),
            Self::Swap(m) => m.entity_indices(),
            Self::PillarChange(m) => m.entity_indices(),
            Self::PillarSwap(m) => m.entity_indices(),
            Self::RuinRecreate(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Change(m) => m.variable_name(),
            Self::Swap(m) => m.variable_name(),
            Self::PillarChange(m) => m.variable_name(),
            Self::PillarSwap(m) => m.variable_name(),
            Self::RuinRecreate(m) => m.variable_name(),
        }
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        match self {
            Self::Change(m) => m.tabu_signature(score_director),
            Self::Swap(m) => m.tabu_signature(score_director),
            Self::PillarChange(m) => m.tabu_signature(score_director),
            Self::PillarSwap(m) => m.tabu_signature(score_director),
            Self::RuinRecreate(m) => m.tabu_signature(score_director),
        }
    }
}
