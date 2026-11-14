//! Planning-id rendering for constraint analysis: how a stream source names
//! the record behind each of its items.
//!
//! The macros pick the rendering for each planning-model collection:
//! entity and fact collections render through [`ModelPlanningIds`], which
//! reads the [`AnalysisPlanningId`] that `#[planning_entity]` and
//! `#[problem_fact]` implement for every type they annotate; every other
//! source renders through [`NoPlanningIds`]. Both are zero-sized, so a source
//! carries its rendering in its type and no stream element needs a new bound.
//! Nothing here runs while scoring.

use std::fmt::{Debug, Display};
use std::marker::PhantomData;

#[cfg(test)]
mod tests;

/// The planning id an annotated model type reports in constraint analysis:
/// its `#[planning_id]` field rendered as a string, or `None` when the type
/// has no `#[planning_id]` field.
///
/// `#[planning_entity]` and `#[problem_fact]` implement it for every type
/// they annotate.
#[doc(hidden)]
pub trait AnalysisPlanningId {
    /// The rendered planning id, or `None` when the type has none.
    fn analysis_planning_id(&self) -> Option<String>;
}

/// How a source renders the planning id of one of its items.
#[doc(hidden)]
pub trait SourcePlanningIds<A>: Send + Sync + 'static {
    /// The item's rendered planning id, or `None`.
    fn planning_id(item: &A) -> Option<String>;
}

/// Rendering for sources whose items have no planning id.
#[doc(hidden)]
pub struct NoPlanningIds(PhantomData<fn()>);

impl<A> SourcePlanningIds<A> for NoPlanningIds {
    #[inline]
    fn planning_id(_item: &A) -> Option<String> {
        None
    }
}

/// Rendering for planning-model entity and fact collections: the item
/// type's [`AnalysisPlanningId`].
#[doc(hidden)]
pub struct ModelPlanningIds(PhantomData<fn()>);

impl<A: AnalysisPlanningId> SourcePlanningIds<A> for ModelPlanningIds {
    #[inline]
    fn planning_id(item: &A) -> Option<String> {
        item.analysis_planning_id()
    }
}

/// Renders a planning id for [`AnalysisPlanningId`]: through `Display` when
/// the id type implements it, otherwise through `Debug`, otherwise not at
/// all (`None`), since `PlanningId::Id` requires neither.
///
/// The macros call `(&&RenderPlanningId(&id)).render_planning_id()` on the
/// concrete id type, with the three rendering traits in scope. Method
/// resolution tries the receiver with the fewest references first: the
/// `Display` impl is on `&RenderPlanningId`, the `Debug` impl on
/// `&&RenderPlanningId`, and the fallback on `&&&RenderPlanningId`, so the
/// first rendering the id type supports wins.
#[doc(hidden)]
pub struct RenderPlanningId<'a, T: ?Sized>(pub &'a T);

/// The `Display` rendering of [`RenderPlanningId`].
#[doc(hidden)]
pub trait RenderPlanningIdDisplay {
    /// Renders the id with `Display`.
    fn render_planning_id(&self) -> Option<String>;
}

impl<T: Display + ?Sized> RenderPlanningIdDisplay for &&RenderPlanningId<'_, T> {
    fn render_planning_id(&self) -> Option<String> {
        Some(self.0.to_string())
    }
}

/// The `Debug` rendering of [`RenderPlanningId`], for ids without `Display`.
#[doc(hidden)]
pub trait RenderPlanningIdDebug {
    /// Renders the id with `Debug`.
    fn render_planning_id(&self) -> Option<String>;
}

impl<T: Debug + ?Sized> RenderPlanningIdDebug for &RenderPlanningId<'_, T> {
    fn render_planning_id(&self) -> Option<String> {
        Some(format!("{:?}", self.0))
    }
}

/// The fallback of [`RenderPlanningId`] for ids with neither `Display` nor
/// `Debug`: no rendering.
#[doc(hidden)]
pub trait RenderPlanningIdNone {
    /// Reports no rendering.
    fn render_planning_id(&self) -> Option<String>;
}

impl<T: ?Sized> RenderPlanningIdNone for RenderPlanningId<'_, T> {
    fn render_planning_id(&self) -> Option<String> {
        None
    }
}
