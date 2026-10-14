use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::ConstraintRef;
use solverforge_scoring::{ConstraintMetadata, Director, DirectorScoreState};

pub(crate) struct SequentialPreviewDirector<S: PlanningSolution> {
    working_solution: S,
    descriptor: SolutionDescriptor,
    constraint_metadata: Vec<(ConstraintRef, bool)>,
    entity_counts: Vec<Option<usize>>,
    total_entity_count: Option<usize>,
}

impl<S: PlanningSolution> SequentialPreviewDirector<S> {
    pub(crate) fn from_director<D: Director<S>>(score_director: &D) -> Self {
        let descriptor = score_director.solution_descriptor();
        let entity_counts = (0..descriptor.entity_descriptor_count())
            .map(|descriptor_index| score_director.entity_count(descriptor_index))
            .collect();

        Self {
            working_solution: score_director.clone_working_solution(),
            descriptor: descriptor.clone(),
            constraint_metadata: score_director
                .constraint_metadata()
                .into_iter()
                .map(|metadata| (metadata.constraint_ref.clone(), metadata.is_hard))
                .collect(),
            entity_counts,
            total_entity_count: score_director.total_entity_count(),
        }
    }
}

impl<S: PlanningSolution> Director<S> for SequentialPreviewDirector<S> {
    fn working_solution(&self) -> &S {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut S {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> S::Score {
        panic!("preview directors are only for selector generation")
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> S {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {
        self.working_solution.set_score(None);
    }

    fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.working_solution
            .update_entity_shadows(descriptor_index, entity_index);
        self.working_solution.set_score(None);
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.entity_counts.get(descriptor_index).copied().flatten()
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.total_entity_count
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        self.constraint_metadata
            .iter()
            .map(|(constraint_ref, is_hard)| ConstraintMetadata::new(constraint_ref, *is_hard))
            .collect()
    }

    fn is_incremental(&self) -> bool {
        false
    }

    fn snapshot_score_state(&self) -> DirectorScoreState<S::Score> {
        DirectorScoreState {
            solution_score: self.working_solution.score(),
            committed_score: self.working_solution.score(),
            initialized: self.working_solution.score().is_some(),
        }
    }

    fn restore_score_state(&mut self, state: DirectorScoreState<S::Score>) {
        self.working_solution.set_score(state.solution_score);
    }
}
