//! Common declaration-resolved access for scored list construction kernels.

use crate::builder::context::RuntimeListSourceIndex;
use crate::list_placement::OwnerRestriction;

/// Storage operations shared by canonical cheapest and regret insertion.
///
/// Implementations expose only frozen slot behavior.  Both algorithm kernels
/// retain positional source identity and own candidate order, scoring trials,
/// trace transitions, and mandatory-construction control flow.
pub(crate) trait ScoredListConstructionAccess<S> {
    type Element: Clone + Send + Sync + 'static;

    fn descriptor_index(&self) -> usize;
    fn entity_count(&self, solution: &S) -> usize;
    fn list_len(&self, solution: &S, entity_index: usize) -> usize;
    fn insert_element(
        &self,
        solution: &mut S,
        entity_index: usize,
        position: usize,
        element: Self::Element,
    );
    fn remove_element(&self, solution: &mut S, entity_index: usize, position: usize);
    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction;
    fn construction_order_key(&self, solution: &S, element: &Self::Element) -> i64;
    fn precedence_duration(&self, solution: &S, element: &Self::Element) -> Option<usize>;
    fn extend_precedence_successor_source_indices(
        &self,
        solution: &S,
        element: &Self::Element,
        source_index: &RuntimeListSourceIndex<Self::Element>,
        successors: &mut Vec<usize>,
    ) -> bool;
}
