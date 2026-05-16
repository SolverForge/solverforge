use std::collections::HashSet;
use std::hash::Hash;

use crate::constraint::grouped::ComplementedGroupedStateView;
use crate::stream::collector::Accumulator;
use crate::stream::ProjectedSource;

use super::indexes::key_hash;
use super::state::{
    ProjectedComplementedGroupedEvaluationState, ProjectedComplementedGroupedNodeState,
};

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
    ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    K: Eq + Hash,
{
    pub(super) fn find_group(&self, hash: u64, key: &K) -> Option<usize> {
        let group_ids = self.groups_by_hash.get(&hash)?;
        group_ids
            .iter()
            .copied()
            .find(|group_id| self.groups[*group_id].key == *key)
    }

    fn visit_complement_slot<Visit>(&self, b_idx: usize, visit: &mut Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        let Some(&group_id) = self.complement_groups.get(&b_idx) else {
            visit(b_idx, None);
            return;
        };
        let group = &self.groups[group_id];
        if group.count > 0 {
            group
                .accumulator
                .with_result(|result| visit(b_idx, Some((&group.key, result))));
        } else if let Some(default_result) = self.complement_defaults.get(&b_idx) {
            visit(b_idx, Some((&group.key, default_result)));
        } else {
            visit(b_idx, None);
        }
    }
}

impl<K, V, R, Acc> ComplementedGroupedStateView<K, R>
    for ProjectedComplementedGroupedEvaluationState<K, V, R, Acc>
where
    K: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_complement_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&K, &R),
    {
        for (key, default_result) in &self.complements {
            if let Some(group) = self.groups.get(key) {
                group.with_result(|result| visit(key, result));
            } else {
                visit(key, default_result);
            }
        }
    }

    fn for_each_complement_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        for (slot, (key, default_result)) in self.complements.iter().enumerate() {
            if let Some(group) = self.groups.get(key) {
                group.with_result(|result| visit(slot, Some((key, result))));
            } else {
                visit(slot, Some((key, default_result)));
            }
        }
    }

    fn for_each_changed_complement_slot_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        self.for_each_complement_slot_result(visit);
    }

    fn for_each_key_result<Visit>(&self, key: &K, mut visit: Visit)
    where
        Visit: FnMut(&R),
    {
        for (entity_key, default_result) in &self.complements {
            if entity_key != key {
                continue;
            }
            if let Some(group) = self.groups.get(key) {
                group.with_result(|result| visit(result));
            } else {
                visit(default_result);
            }
        }
    }

    fn complement_count(&self) -> usize {
        self.complements.len()
    }
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D> ComplementedGroupedStateView<K, R>
    for ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
where
    Src: ProjectedSource<S, Out>,
    K: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_complement_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&K, &R),
    {
        for (&b_idx, &group_id) in &self.complement_groups {
            let group = &self.groups[group_id];
            if group.count > 0 {
                group
                    .accumulator
                    .with_result(|result| visit(&group.key, result));
            } else if let Some(default_result) = self.complement_defaults.get(&b_idx) {
                visit(&group.key, default_result);
            }
        }
    }

    fn for_each_complement_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        for &b_idx in self.complement_groups.keys() {
            self.visit_complement_slot(b_idx, &mut visit);
        }
    }

    fn for_each_changed_complement_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&K, &R)>),
    {
        let mut visited = HashSet::new();
        for &group_id in &self.changed_groups {
            let Some(indices) = self.complements_by_group.get(&group_id) else {
                continue;
            };
            for &b_idx in indices {
                if visited.insert(b_idx) {
                    self.visit_complement_slot(b_idx, &mut visit);
                }
            }
        }
        for &b_idx in &self.changed_complements {
            if visited.insert(b_idx) {
                self.visit_complement_slot(b_idx, &mut visit);
            }
        }
    }

    fn for_each_key_result<Visit>(&self, key: &K, mut visit: Visit)
    where
        Visit: FnMut(&R),
    {
        let hash = key_hash(key);
        let Some(group_id) = self.find_group(hash, key) else {
            return;
        };
        let Some(indices) = self.complements_by_group.get(&group_id) else {
            return;
        };
        let group = &self.groups[group_id];
        for &b_idx in indices {
            if group.count > 0 {
                group.accumulator.with_result(|result| visit(result));
            } else if let Some(default_result) = self.complement_defaults.get(&b_idx) {
                visit(default_result);
            }
        }
    }

    fn complement_count(&self) -> usize {
        self.complement_groups.len()
    }
}
