use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::builder::ScalarVariableContext;

pub trait PlanningModelSupport: PlanningSolution + Sized + 'static {
    fn attach_descriptor_scalar_hooks(descriptor: &mut SolutionDescriptor);

    fn attach_runtime_scalar_hooks(
        context: ScalarVariableContext<Self>,
    ) -> ScalarVariableContext<Self>;

    fn validate_model(descriptor: &SolutionDescriptor);

    fn update_entity_shadows(
        solution: &mut Self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> bool;

    fn update_all_shadows(solution: &mut Self) -> bool;
}
