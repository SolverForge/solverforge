use super::*;
use crate::stream::relational::operator::links::remove_link;

impl<S: 'static, L: Operator<S>, R: Operator<S>, P, F, T> ComplementNode<S, L, R, P, F, T>
where
    P: IndexedPlan + KeyOf<Key = <P as RightKeyOf>::Key> + RightKeyOf + 'static,
    for<'a> P: ExecutablePlan<L::View<'a>, R::View<'a>>,
    F: for<'a> Fn(&S, &L::View<'a>) -> T,
{
    pub(super) fn remove_output(&mut self, h: RowHandle) {
        if let Some(row) = self.rows.retract(h) {
            match row.value {
                OutputValue::Real(id) => {
                    self.real_outputs.remove(&id);
                    if let Some(moved) =
                        remove_link(&mut self.left_outputs, id.left(), h, row.left_position)
                    {
                        self.rows
                            .get_mut(moved)
                            .expect("moved complement target")
                            .left_position = row.left_position;
                    }
                    if let Some(moved) =
                        remove_link(&mut self.right_outputs, id.right(), h, row.right_position)
                    {
                        self.rows
                            .get_mut(moved)
                            .expect("moved complement result")
                            .right_position = row.right_position;
                    }
                }
                OutputValue::Default(input, _) => {
                    self.default_outputs.remove(input);
                    if let Some(moved) =
                        remove_link(&mut self.left_outputs, input, h, row.left_position)
                    {
                        self.rows
                            .get_mut(moved)
                            .expect("moved complement target")
                            .left_position = row.left_position;
                    }
                }
            }
        }
    }

    pub(super) fn emit_real(
        rows: &mut DenseRowStore<Output<T>>,
        outputs: &mut FastMap<JoinedIdentity, RowHandle>,
        left_outputs: &mut HandleMap<Vec<RowHandle>>,
        right_outputs: &mut HandleMap<Vec<RowHandle>>,
        left: RowHandle,
        right: RowHandle,
        inserted: &mut Vec<RowHandle>,
    ) -> Option<RowHandle> {
        let id = JoinedIdentity::new(left, right, 0);
        if outputs.contains_key(&id) {
            return None;
        }
        let left_bucket = left_outputs.get_or_insert_with(left, Vec::new);
        let right_bucket = right_outputs.get_or_insert_with(right, Vec::new);
        let h = rows.insert(Output {
            value: OutputValue::Real(id),
            left_position: left_bucket.len(),
            right_position: right_bucket.len(),
        });
        outputs.insert(id, h);
        left_bucket.push(h);
        right_bucket.push(h);
        inserted.push(h);
        Some(h)
    }

    pub(super) fn emit_default(
        &mut self,
        solution: &S,
        input: RowHandle,
        inserted: &mut Vec<RowHandle>,
    ) {
        assert!(
            self.default_outputs.get(input).is_none(),
            "duplicate default output"
        );
        let row = self
            .targets
            .resolve(solution, input)
            .expect("live complement target");
        let value = (self.default)(solution, &row);
        let bucket = self.left_outputs.get_or_insert_with(input, Vec::new);
        let h = self.rows.insert(Output {
            value: OutputValue::Default(input, value),
            left_position: bucket.len(),
            right_position: 0,
        });
        self.default_outputs.insert(input, h);
        bucket.push(h);
        inserted.push(h);
    }
}
