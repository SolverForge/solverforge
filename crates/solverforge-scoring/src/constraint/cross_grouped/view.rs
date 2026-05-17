use std::hash::Hash;

use crate::constraint::grouped::GroupedStateView;
use crate::stream::collector::Accumulator;

use super::indexes::key_hash;
use super::state::{CrossGroupedEvaluationState, CrossGroupedNodeState};

impl<GK, V, R, Acc> GroupedStateView<GK, R> for CrossGroupedEvaluationState<GK, V, R, Acc>
where
    GK: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_group_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&GK, &R),
    {
        for (key, accumulator) in &self.groups {
            accumulator.with_result(|result| visit(key, result));
        }
    }

    fn for_each_group_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        for (group_id, (key, accumulator)) in self.groups.iter().enumerate() {
            accumulator.with_result(|result| visit(group_id, Some((key, result))));
        }
    }

    fn for_each_changed_group_slot_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        self.for_each_group_slot_result(visit);
    }

    fn with_group_result<T, Present, Missing>(
        &self,
        key: &GK,
        present: Present,
        missing: Missing,
    ) -> T
    where
        Present: FnOnce(&R) -> T,
        Missing: FnOnce() -> T,
    {
        match self.groups.get(key) {
            Some(accumulator) => accumulator.with_result(present),
            None => missing(),
        }
    }

    fn group_count(&self) -> usize {
        self.groups.len()
    }
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc> GroupedStateView<GK, R>
    for CrossGroupedNodeState<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc>
where
    GK: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_group_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&GK, &R),
    {
        for group in &self.groups {
            if group.count > 0 {
                group
                    .accumulator
                    .with_result(|result| visit(&group.key, result));
            }
        }
    }

    fn for_each_group_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        for (group_id, group) in self.groups.iter().enumerate() {
            if group.count == 0 {
                visit(group_id, None);
                continue;
            }
            group
                .accumulator
                .with_result(|result| visit(group_id, Some((&group.key, result))));
        }
    }

    fn for_each_changed_group_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        for &group_id in &self.changed_groups {
            let Some(group) = self.groups.get(group_id) else {
                continue;
            };
            if group.count == 0 {
                visit(group_id, None);
                continue;
            }
            group
                .accumulator
                .with_result(|result| visit(group_id, Some((&group.key, result))));
        }
    }

    fn with_group_result<T, Present, Missing>(
        &self,
        key: &GK,
        present: Present,
        missing: Missing,
    ) -> T
    where
        Present: FnOnce(&R) -> T,
        Missing: FnOnce() -> T,
    {
        let hash = key_hash(key);
        let Some(group_id) = self.find_group(hash, key) else {
            return missing();
        };
        let group = &self.groups[group_id];
        if group.count == 0 {
            return missing();
        }
        group.accumulator.with_result(present)
    }

    fn group_count(&self) -> usize {
        self.groups.iter().filter(|group| group.count > 0).count()
    }
}
