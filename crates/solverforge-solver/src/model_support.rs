use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::builder::{ScalarGroupBinding, ScalarVariableSlot};

pub trait PlanningModelSupport: PlanningSolution + Sized + 'static {
    fn attach_descriptor_hooks(descriptor: &mut SolutionDescriptor);

    fn attach_runtime_scalar_hooks(slot: ScalarVariableSlot<Self>) -> ScalarVariableSlot<Self>;

    fn attach_scalar_groups(
        _scalar_variables: &[ScalarVariableSlot<Self>],
    ) -> Vec<ScalarGroupBinding<Self>> {
        Vec::new()
    }

    fn validate_model(descriptor: &SolutionDescriptor);

    fn update_entity_shadows(
        solution: &mut Self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> bool;

    fn update_all_shadows(solution: &mut Self) -> bool;
}
