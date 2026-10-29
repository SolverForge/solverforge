use solverforge_core::domain::PlanningSolution;

use crate::builder::context::list_access::ListAccess;
use crate::builder::context::{RuntimeListElement, RuntimeListSlot};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_placement::OwnerRestriction;

#[cfg(test)]
mod facade;
mod kernel;

#[cfg(test)]
pub(crate) use facade::ListConstructionPhase;
pub(crate) use kernel::{run_round_robin, RoundRobinAccess};

impl<S, V, DM, IDM> RoundRobinAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = RuntimeListElement<V>;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn entity_count(&self, solution: &S) -> usize {
        ListAccess::entity_count(self, solution)
    }

    fn construction_order_key(&self, solution: &S, element: &Self::Element) -> i64 {
        ListAccess::construction_order_key(self, solution, element.clone())
            .expect("compiled round-robin construction order must be validated before execution")
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction {
        match ListAccess::element_owner(self, solution, element)
            .expect("compiled round-robin ownership policy must be callable")
        {
            None => OwnerRestriction::Unrestricted,
            Some(owner_idx) if owner_idx < entity_count => OwnerRestriction::Fixed(owner_idx),
            Some(_) => OwnerRestriction::Invalid,
        }
    }

    fn append_element(&self, solution: &mut S, entity_index: usize, element: Self::Element) {
        let insert_position = ListAccess::list_len(self, solution, entity_index);
        ListAccess::list_insert(self, solution, entity_index, insert_position, element);
    }
}

#[cfg(test)]
#[path = "round_robin/tests.rs"]
mod tests;
