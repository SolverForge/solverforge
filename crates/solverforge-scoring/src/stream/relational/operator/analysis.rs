// Cold explanation boundary, separate from borrowed hot-path row traversal.
use super::{super::Leaf, Pair};
use crate::api::analysis::EntityRef;
use std::fmt::Debug;

#[doc(hidden)]
pub trait ExplainRow {
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
