//! Frozen ownership state for shared list cursors.
//!
//! Raw owner-hook interpretation remains exclusively in list_placement.  This
//! type stores its already-resolved result so candidate pulls cannot re-enter a
//! callback or change their order.

use crate::list_placement::{OwnerRestriction, SelectedOwnerRestrictions};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SelectedListOwners {
    Absent,
    FixedToCurrent,
    Mixed(Vec<Vec<OwnerRestriction>>),
}

impl SelectedListOwners {
    pub(crate) fn from_selected_restrictions(
        restrictions: Option<SelectedOwnerRestrictions>,
    ) -> Self {
        match restrictions {
            None => Self::Absent,
            Some(SelectedOwnerRestrictions::FixedToCurrent) => Self::FixedToCurrent,
            Some(SelectedOwnerRestrictions::Mixed(owners)) => Self::Mixed(owners),
        }
    }

    pub(crate) fn is_fixed_to_current(&self) -> bool {
        matches!(self, Self::FixedToCurrent)
    }

    pub(crate) fn is_present(&self) -> bool {
        !matches!(self, Self::Absent)
    }

    pub(crate) fn has_matrix(&self) -> bool {
        matches!(self, Self::Mixed(_))
    }

    pub(crate) fn allows(
        &self,
        selected_entity_index: usize,
        position: usize,
        destination_entity: usize,
    ) -> bool {
        match self {
            Self::Absent | Self::FixedToCurrent => true,
            Self::Mixed(owners) => owners
                .get(selected_entity_index)
                .and_then(|route| route.get(position))
                .copied()
                .is_some_and(|restriction| restriction.allows(destination_entity)),
        }
    }

    pub(crate) fn restriction_at(
        &self,
        selected_entity_index: usize,
        position: usize,
    ) -> Option<OwnerRestriction> {
        match self {
            Self::Mixed(owners) => owners
                .get(selected_entity_index)
                .and_then(|route| route.get(position))
                .copied(),
            Self::Absent | Self::FixedToCurrent => None,
        }
    }

    pub(crate) fn segment_allows(
        &self,
        selected_entity_index: usize,
        start: usize,
        end: usize,
        destination_entity: usize,
    ) -> bool {
        match self {
            Self::Absent | Self::FixedToCurrent => true,
            Self::Mixed(owners) => owners.get(selected_entity_index).is_some_and(|route| {
                (start..end).all(|position| {
                    route
                        .get(position)
                        .copied()
                        .is_some_and(|restriction| restriction.allows(destination_entity))
                })
            }),
        }
    }
}
