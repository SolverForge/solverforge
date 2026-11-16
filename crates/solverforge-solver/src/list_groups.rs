//! Element groups of list variables: members that share one owner and move
//! together, placed jointly by group moves, ruin and recreate, and scored
//! list construction.

mod index;
mod joint;

#[cfg(test)]
mod tests;

pub(crate) use index::{ListElementGroups, ListGroupPlacement};
#[cfg(test)]
pub(crate) use joint::JointPlacements;
pub(crate) use joint::{
    member_gap, search_joint_placement, JointBounds, JointChoice, JointPositions, JointSearch,
    DEFAULT_GROUP_MEMBER_DISTANCE, DEFAULT_GROUP_OWNERS,
};
