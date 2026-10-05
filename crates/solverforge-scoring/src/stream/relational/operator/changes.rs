use crate::stream::relational::RowHandle;

/// One root notification can both retract old rows and insert replacement rows.
/// Consumers remove all old indexes before probing newly published rows.
#[derive(Default, Debug)]
pub struct RowChanges {
    pub removed: Vec<RowHandle>,
    pub inserted: Vec<RowHandle>,
}
impl RowChanges {
    pub fn is_empty(&self) -> bool {
        self.removed.is_empty() && self.inserted.is_empty()
    }
    pub fn len(&self) -> usize {
        self.removed.len() + self.inserted.len()
    }
}
