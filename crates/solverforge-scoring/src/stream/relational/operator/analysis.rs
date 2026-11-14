// Cold explanation boundary, separate from borrowed hot-path row traversal.
use super::super::RowHandle;
use super::Operator;
use crate::api::analysis::EntityRef;

/// Where an explained row came from: a retained row by its handle, or a row
/// visited from a full evaluation.
pub enum RowOrigin<'a, E> {
    /// A retained row ([`Operator::resolve`]) with this handle.
    Retained(RowHandle),
    /// A row visited from this evaluation ([`Operator::visit_evaluation`]).
    Evaluation(&'a E),
}

impl<E> Copy for RowOrigin<'_, E> {}

impl<E> Clone for RowOrigin<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, E> RowOrigin<'a, E> {
    /// The origin of an input row: the same handle for a retained row, or
    /// the part of the evaluation `part` selects.
    pub fn map<F>(self, part: impl FnOnce(&'a E) -> &'a F) -> RowOrigin<'a, F> {
        match self {
            Self::Retained(handle) => RowOrigin::Retained(handle),
            Self::Evaluation(evaluation) => RowOrigin::Evaluation(part(evaluation)),
        }
    }
}

/// An operator that can name the entities each of its rows was built from,
/// for score explanation.
///
/// Terminals call it on the cold explanation path only (`get_matches`,
/// detailed analysis), never while scoring. Explanation belongs to the
/// operator rather than the row view because the operator owns the sources:
/// a collection leaf reads its source's planning-id rendering, so each
/// [`EntityRef`] carries the record's planning id when its type has one.
/// Joins push both sides in order, and grouped, projected, flattened,
/// filtered and complemented operators push the rows they derive from by
/// asking their inputs, mapping a retained handle to the input handles the
/// row was built from. Implement it for a custom [`Operator`] so terminals
/// over that operator can justify their matches.
pub trait ExplainOperator<S: 'static>: Operator<S> {
    /// Appends a reference to every entity `row` was built from to
    /// `entities`, in row order. `origin` says where `row` came from.
    fn explain(
        &self,
        solution: &S,
        row: &Self::View<'_>,
        origin: RowOrigin<'_, Self::Evaluation>,
        entities: &mut Vec<EntityRef>,
    );
}
