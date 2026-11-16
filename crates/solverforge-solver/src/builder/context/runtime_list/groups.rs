use super::*;
use crate::list_groups::{ListElementGroups, ListGroupPlacement};

impl<S, V, DM, IDM> RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    /// The slot's element groups, or `None` when it declares no group hook
    /// or the hook forms no group of two or more members.
    pub(crate) fn element_groups(&self, solution: &S) -> Option<ListElementGroups> {
        let Self::Static {
            metadata_bindings, ..
        } = self
        else {
            return None;
        };
        if !metadata_bindings.element_group_policy.is_explicit() {
            return None;
        }
        let element_group = metadata_bindings.element_relation;
        let mut values = Vec::new();
        let groups = ListElementGroups::from_hook(
            ListAccess::element_count(self, solution),
            |index| ListAccess::index_to_element(self, solution, index),
            |element| ListAccess::element_source_key(self, solution, element),
            |element, members| {
                values.clear();
                element_group(solution, static_element(element.clone()), &mut values);
                members.extend(values.drain(..).map(RuntimeListElement::Static));
            },
        );
        (!groups.is_empty()).then_some(groups)
    }

    /// Locates every grouped element in the current lists.
    pub(crate) fn locate_group_members(
        &self,
        groups: &ListElementGroups,
        solution: &S,
    ) -> ListGroupPlacement {
        ListGroupPlacement::locate(
            groups,
            ListAccess::entity_count(self, solution),
            |entity| ListAccess::list_len(self, solution, entity),
            |entity, position| {
                ListAccess::list_get(self, solution, entity, position)
                    .map(|element| ListAccess::element_source_key(self, solution, &element))
            },
        )
    }
}
