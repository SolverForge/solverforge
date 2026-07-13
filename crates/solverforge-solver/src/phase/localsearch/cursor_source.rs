//! One phase-facing source for move cursors.
//!
//! Ordinary selectors use their existing cursor kernel through the one
//! `SelectorCursorSource` adapter below. Configured selector compositions
//! instead own their persistent state tree and implement this trait directly.
//! Local-search and VND therefore have one cursor-opening operation and no
//! selector-specific execution branch.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    MoveSelector, MoveStreamContext, ResourceMoveCursor, UnitResourceCursor,
};

/// Opens the next phase cursor from mutable solve-owned execution state.
///
/// Implementations must return a cursor that owns all candidate work. A
/// source may lend state only while opening that cursor and must regain it
/// when the cursor drops, which lets pause/resume and subsequent steps retain
/// the exact seeded stream progression.
pub trait MoveCursorSource<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Mutable solve-owned data lent at cursor-open and reachable-pull
    /// boundaries. Ordinary stock selectors use `()`. Compiled runtime
    /// sources receive the runner's one provider registry/reason arena here,
    /// so VND can retain many neighborhood states without cloning or storing
    /// a mutable resource in any one of them.
    type Resources: Send;

    /// The cursor has no mutable resource field. The shared phase loop lends
    /// `Resources` only for each candidate pull through
    /// [`ResourceMoveCursor::next_candidate_with_resources`].
    type Cursor<'a>: ResourceMoveCursor<S, M, Self::Resources>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(
        &'a mut self,
        resources: &mut Self::Resources,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a>;
}

/// Type-level adapter for ordinary selectors and their one existing cursor
/// kernel.
///
/// This is not a fallback or a second execution path: it delegates directly
/// to `MoveSelector::open_cursor_with_context`. Stateful compiled
/// compositions use their explicit execution owners instead, which avoids
/// coherence overlap while preserving one phase loop.
#[doc(hidden)]
pub struct SelectorCursorSource<MS> {
    selector: MS,
}

impl<MS> SelectorCursorSource<MS> {
    pub(crate) fn new(selector: MS) -> Self {
        Self { selector }
    }
}

impl<MS: std::fmt::Debug> std::fmt::Debug for SelectorCursorSource<MS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SelectorCursorSource")
            .field(&self.selector)
            .finish()
    }
}

impl<S, M, MS> MoveCursorSource<S, M> for SelectorCursorSource<MS>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M>,
{
    type Resources = ();

    type Cursor<'a>
        = UnitResourceCursor<MS::Cursor<'a>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(
        &'a mut self,
        _resources: &mut Self::Resources,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        UnitResourceCursor::new(
            self.selector
                .open_cursor_with_context(score_director, context),
        )
    }
}
