use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::builder::{ListVariableSlot, ScalarGroupBinding, ScalarVariableSlot};

/// Model-level hooks `planning_model!` generates for a planning solution.
/// Not meant to be implemented by hand.
pub trait PlanningModelSupport: PlanningSolution + Sized + 'static {
    /// Attaches the model's scalar hooks to the variable descriptors.
    fn attach_descriptor_hooks(descriptor: &mut SolutionDescriptor);

    /// Attaches the model's scalar hooks to a runtime scalar slot.
    fn attach_runtime_scalar_hooks(slot: ScalarVariableSlot<Self>) -> ScalarVariableSlot<Self>;

    /// Attaches the model's list hooks to a runtime list slot; returns it
    /// unchanged by default.
    fn attach_runtime_list_hooks<DM, IDM>(
        slot: ListVariableSlot<Self, usize, DM, IDM>,
    ) -> ListVariableSlot<Self, usize, DM, IDM> {
        slot
    }

    /// The entity whose list must hold `element` for the named list variable;
    /// `None` by default (any entity).
    fn list_element_owner(
        _entity_type_name: &'static str,
        _variable_name: &'static str,
        _solution: &Self,
        _element: &usize,
    ) -> Option<usize> {
        None
    }

    /// The model's scalar groups bound to its scalar slots; none by default.
    fn attach_scalar_groups(
        _scalar_variables: &[ScalarVariableSlot<Self>],
    ) -> Vec<ScalarGroupBinding<Self>> {
        Vec::new()
    }

    /// Validates the model declaration against `descriptor`, panicking when it
    /// is invalid.
    fn validate_model(descriptor: &SolutionDescriptor);

    /// Updates the shadow variables of one entity; returns whether anything
    /// was updated.
    fn update_entity_shadows(
        solution: &mut Self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> bool;

    /// Recomputes every shadow variable; returns whether the model has shadow
    /// updates.
    fn update_all_shadows(solution: &mut Self) -> bool;

    /// The element collection index and the list of one list owner whose
    /// elements carry shadows; `None` by default.
    fn list_shadow_elements(
        _solution: &Self,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> Option<(usize, &[usize])> {
        None
    }

    /// Clears the list shadows of `element`; a no-op by default.
    fn clear_list_shadows(_solution: &mut Self, _element: usize) {}

    /// Index of the element collection named `collection` when its elements
    /// carry list shadows; `None` by default.
    fn list_shadow_source(_collection: &str) -> Option<usize> {
        None
    }

    /// Updates the list structure shadows (inverse, index, previous, next)
    /// of one owner without the cascading shadow; returns whether anything
    /// was updated. Defaults to `update_entity_shadows`.
    fn update_list_structure_shadows(
        solution: &mut Self,
        descriptor_index: usize,
        entity_index: usize,
    ) -> bool {
        <Self as PlanningModelSupport>::update_entity_shadows(
            solution,
            descriptor_index,
            entity_index,
        )
    }

    /// Number of elements carrying the cascading shadow; `0` by default.
    fn cascading_element_count(_solution: &Self) -> usize {
        0
    }

    /// Whether `element` is held by a list; `false` by default.
    fn cascading_assigned(_solution: &Self, _element: usize) -> bool {
        false
    }

    /// The element after `element` in its list; `None` by default.
    fn cascading_next(_solution: &Self, _element: usize) -> Option<usize> {
        None
    }

    /// Appends the declared dependencies of `element`; none by default.
    fn cascading_dependencies(_solution: &Self, _element: usize, _dependencies: &mut Vec<usize>) {}

    /// Whether `element`'s cascading value is unset; `true` by default.
    fn cascading_unset(_solution: &Self, _element: usize) -> bool {
        true
    }

    /// Recomputes (or, when `computable` is `false`, unsets) `element`'s
    /// cascading value, calling `before_write` before writing a changed
    /// value; returns whether it changed. `false` by default.
    fn update_cascading_shadow<W: FnOnce(&Self)>(
        _solution: &mut Self,
        _element: usize,
        _computable: bool,
        _before_write: W,
    ) -> bool {
        false
    }
}
