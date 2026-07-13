//! Runtime-list storage adapter shared by scored construction kernels.

use std::fmt;

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::list_access::ListAccess;
use crate::builder::context::{RuntimeListElement, RuntimeListSlot, RuntimeListSourceIndex};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_placement::OwnerRestriction;
use crate::manager::ScoredListConstructionAccess;

/// The one compiled-slot implementation for cheapest and regret insertion.
///
/// It resolves optional list metadata through already validated slot policies
/// and translates precedence successors to frozen source indexes. Individual
/// construction phase wrappers retain only prepared source state and invoke
/// their canonical algorithm.
impl<S, V, DM, IDM> ScoredListConstructionAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = RuntimeListElement<V>;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn entity_count(&self, solution: &S) -> usize {
        ListAccess::entity_count(self, solution)
    }

    fn list_len(&self, solution: &S, entity_index: usize) -> usize {
        ListAccess::list_len(self, solution, entity_index)
    }

    fn insert_element(
        &self,
        solution: &mut S,
        entity_index: usize,
        position: usize,
        element: Self::Element,
    ) {
        ListAccess::list_insert(self, solution, entity_index, position, element);
    }

    fn remove_element(&self, solution: &mut S, entity_index: usize, position: usize) {
        ListAccess::list_remove(self, solution, entity_index, position)
            .expect("compiled scored list insertion must remove the just-inserted element");
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction {
        match ListAccess::element_owner(self, solution, element)
            .expect("compiled scored list ownership policy must be callable")
        {
            None => OwnerRestriction::Unrestricted,
            Some(owner) if owner < entity_count => OwnerRestriction::Fixed(owner),
            Some(_) => OwnerRestriction::Invalid,
        }
    }

    fn construction_order_key(&self, solution: &S, element: &Self::Element) -> i64 {
        ListAccess::construction_order_key(self, solution, element.clone())
            .expect("compiled scored list construction order must be validated before execution")
    }

    fn precedence_duration(&self, solution: &S, element: &Self::Element) -> Option<usize> {
        ListAccess::precedence_duration(self, solution, element.clone()).ok()
    }

    fn extend_precedence_successor_source_indices(
        &self,
        solution: &S,
        element: &Self::Element,
        source_index: &RuntimeListSourceIndex<Self::Element>,
        successors: &mut Vec<usize>,
    ) -> bool {
        let mut values = Vec::new();
        if ListAccess::extend_precedence_successors(self, solution, element.clone(), &mut values)
            .is_err()
        {
            return false;
        }
        successors.extend(values.into_iter().filter_map(|successor| {
            source_index
                .source_index_for_key(ListAccess::element_source_key(self, solution, &successor))
        }));
        true
    }
}
