//! Selectors for entities, values, and moves.
//!
//! Selectors enumerate the elements that the solver considers when
//! exploring the solution space.

pub(crate) mod decorator;
#[cfg(test)]
pub(crate) mod dynamic_list_change;
#[cfg(test)]
pub(crate) mod dynamic_scalar_nearby_change;
#[cfg(test)]
pub(crate) mod dynamic_scalar_nearby_swap;
pub(crate) mod entity;
pub(crate) mod k_opt;
#[cfg(test)]
pub(crate) mod list_change;
pub(crate) mod list_kernel;
#[cfg(test)]
pub(crate) mod list_permute;
#[cfg(test)]
pub(crate) mod list_precedence;
#[cfg(test)]
pub(crate) mod list_ruin;
pub(crate) mod list_support;
#[cfg(test)]
pub(crate) mod list_swap;
pub(crate) mod move_selector;
pub(crate) mod nearby_list_change;
pub(crate) mod nearby_list_support;
#[cfg(test)]
pub(crate) mod nearby_list_swap;
pub(crate) mod nearby_support;
pub(crate) mod pillar;
pub(crate) mod pillar_support;
pub(crate) mod precedence_route;
pub(crate) mod rng_advance;
pub(crate) mod scalar_neighborhood;
pub(crate) mod seed;
#[cfg(test)]
pub(crate) mod sublist_change;
mod sublist_support;
#[cfg(test)]
pub(crate) mod sublist_swap;
#[cfg(test)]
pub(crate) mod value_selector;

#[cfg(test)]
mod tests;

pub(crate) use entity::EntityReference;
#[cfg(test)]
pub(crate) use entity::EntitySelector;
pub(crate) use move_selector::MoveSelector;
pub(crate) use nearby_list_change::DefaultCrossEntityDistanceMeter;
#[cfg(test)]
pub(crate) use value_selector::ValueSelector;
