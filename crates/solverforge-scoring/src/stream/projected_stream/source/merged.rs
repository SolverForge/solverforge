use crate::stream::collection_extract::ChangeSource;

use super::{ProjectedRowCoordinate, ProjectedSource};

pub struct MergedProjectedSource<Left, Right> {
    left: Left,
    right: Right,
}

impl<Left, Right> MergedProjectedSource<Left, Right> {
    pub(crate) fn new(left: Left, right: Right) -> Self {
        Self { left, right }
    }
}

impl<S, Out, Left, Right> ProjectedSource<S, Out> for MergedProjectedSource<Left, Right>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    Left: ProjectedSource<S, Out>,
    Right: ProjectedSource<S, Out>,
{
    type State = (Left::State, Right::State);

    const MAX_EMITS: usize = Left::MAX_EMITS + Right::MAX_EMITS;

    fn source_count(&self) -> usize {
        self.left.source_count() + self.right.source_count()
    }

    fn change_source(&self, slot: usize) -> ChangeSource {
        let left_count = self.left.source_count();
        if slot < left_count {
            self.left.change_source(slot)
        } else {
            self.right.change_source(slot - left_count)
        }
    }

    fn build_state(&self, solution: &S) -> Self::State {
        (
            self.left.build_state(solution),
            self.right.build_state(solution),
        )
    }

    fn collect_all<V>(&self, solution: &S, state: &Self::State, mut visit: V)
    where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        self.left.collect_all(solution, &state.0, &mut visit);
        let left_count = self.left.source_count();
        self.right
            .collect_all(solution, &state.1, |coordinate, output| {
                visit(coordinate.offset_source_slots(left_count), output);
            });
    }

    fn collect_entity<V>(
        &self,
        solution: &S,
        state: &Self::State,
        slot: usize,
        entity_index: usize,
        visit: V,
    ) where
        V: FnMut(ProjectedRowCoordinate, Out),
    {
        let left_count = self.left.source_count();
        if slot < left_count {
            self.left
                .collect_entity(solution, &state.0, slot, entity_index, visit);
        } else {
            let mut visit = visit;
            self.right.collect_entity(
                solution,
                &state.1,
                slot - left_count,
                entity_index,
                |coordinate, output| {
                    visit(coordinate.offset_source_slots(left_count), output);
                },
            );
        }
    }

    fn insert_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    ) {
        let left_count = self.left.source_count();
        if slot < left_count {
            self.left
                .insert_entity_state(solution, &mut state.0, slot, entity_index);
        } else {
            self.right
                .insert_entity_state(solution, &mut state.1, slot - left_count, entity_index);
        }
    }

    fn retract_entity_state(
        &self,
        solution: &S,
        state: &mut Self::State,
        slot: usize,
        entity_index: usize,
    ) {
        let left_count = self.left.source_count();
        if slot < left_count {
            self.left
                .retract_entity_state(solution, &mut state.0, slot, entity_index);
        } else {
            self.right.retract_entity_state(
                solution,
                &mut state.1,
                slot - left_count,
                entity_index,
            );
        }
    }
}
