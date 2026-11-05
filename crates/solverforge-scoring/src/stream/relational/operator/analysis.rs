// Cold explanation boundary, separate from borrowed hot-path row traversal.
use super::{super::Leaf, Pair};
use crate::api::analysis::EntityRef;
use std::fmt::Debug;

/// A row view that can name the entities it was built from, for score
/// explanation.
///
/// Terminals call it on the cold explanation path only (`get_matches`,
/// detailed analysis), never while scoring. Leaves push their entity, pairs
/// and arrays push each part in order, and grouped, projected, flattened and
/// complemented views push the rows they derive from. Implement it for the
/// `View` of a custom [`Operator`](super::Operator) so terminals over that
/// operator can justify their matches.
pub trait ExplainRow {
    /// Appends a reference to every entity this row was built from to
    /// `entities`, in row order.
    fn explain(&self, entities: &mut Vec<EntityRef>);
}
impl<T: Clone + Debug + Send + Sync + 'static> ExplainRow for Leaf<'_, T> {
    fn explain(&self, entities: &mut Vec<EntityRef>) {
        entities.push(EntityRef::new(self.entity));
    }
}
impl<L: ExplainRow, R: ExplainRow> ExplainRow for Pair<L, R> {
    fn explain(&self, entities: &mut Vec<EntityRef>) {
        self.left.explain(entities);
        self.right.explain(entities);
    }
}

impl<E: ExplainRow, const N: usize> ExplainRow for [E; N] {
    fn explain(&self, entities: &mut Vec<EntityRef>) {
        for row in self {
            row.explain(entities);
        }
    }
}
