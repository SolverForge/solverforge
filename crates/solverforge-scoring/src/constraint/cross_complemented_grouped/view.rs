use std::collections::HashSet;
use std::hash::Hash;

use crate::constraint::grouped::ComplementedGroupedStateView;
use crate::stream::collector::Accumulator;

use super::indexes::key_hash;
use super::state::{CrossComplementedGroupedEvaluationState, CrossComplementedGroupedNodeState};

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
    CrossComplementedGroupedNodeState<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
    >
where
    Acc: Accumulator<V, R>,
    GK: Eq + Hash,
{
    pub(super) fn find_group(&self, hash: u64, key: &GK) -> Option<usize> {
        let group_ids = self.groups_by_hash.get(&hash)?;
        group_ids
            .iter()
            .copied()
            .find(|group_id| self.groups[*group_id].key == *key)
    }

    fn visit_complement_slot<Visit>(&self, t_idx: usize, visit: &mut Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        let Some(&group_id) = self.t_index_to_group.get(&t_idx) else {
            visit(t_idx, None);
            return;
        };
        let group = &self.groups[group_id];
        if group.count > 0 {
            group
                .accumulator
                .with_result(|result| visit(t_idx, Some((&group.key, result))));
        } else if let Some(default_result) = self.t_defaults.get(&t_idx) {
            visit(t_idx, Some((&group.key, default_result)));
        } else {
            visit(t_idx, None);
        }
    }
}

impl<GK, V, R, Acc> ComplementedGroupedStateView<GK, R>
    for CrossComplementedGroupedEvaluationState<GK, V, R, Acc>
where
    GK: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_complement_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&GK, &R),
    {
        for (key, default_result) in &self.targets {
            if let Some(group) = self.groups.get(key) {
                group.with_result(|result| visit(key, result));
            } else {
                visit(key, default_result);
            }
        }
    }

    fn for_each_complement_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        for (slot, (key, default_result)) in self.targets.iter().enumerate() {
            if let Some(group) = self.groups.get(key) {
                group.with_result(|result| visit(slot, Some((key, result))));
            } else {
                visit(slot, Some((key, default_result)));
            }
        }
    }

    fn for_each_changed_complement_slot_result<Visit>(&self, visit: Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        self.for_each_complement_slot_result(visit);
    }

    fn for_each_key_result<Visit>(&self, key: &GK, mut visit: Visit)
    where
        Visit: FnMut(&R),
    {
        for (target_key, default_result) in &self.targets {
            if target_key != key {
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
        self.targets.len()
    }
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
    ComplementedGroupedStateView<GK, R>
    for CrossComplementedGroupedNodeState<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
    >
where
    GK: Eq + Hash,
    Acc: Accumulator<V, R>,
{
    fn for_each_complement_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(&GK, &R),
    {
        for (&t_idx, &group_id) in &self.t_index_to_group {
            let group = &self.groups[group_id];
            if group.count > 0 {
                group
                    .accumulator
                    .with_result(|result| visit(&group.key, result));
            } else if let Some(default_result) = self.t_defaults.get(&t_idx) {
                visit(&group.key, default_result);
            }
        }
    }

    fn for_each_complement_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        for &t_idx in self.t_index_to_group.keys() {
            self.visit_complement_slot(t_idx, &mut visit);
        }
    }

    fn for_each_changed_complement_slot_result<Visit>(&self, mut visit: Visit)
    where
        Visit: FnMut(usize, Option<(&GK, &R)>),
    {
        let mut visited = HashSet::new();
        for &group_id in &self.changed_groups {
            let Some(indices) = self.t_by_group.get(&group_id) else {
                continue;
            };
            for &t_idx in indices {
                if visited.insert(t_idx) {
                    self.visit_complement_slot(t_idx, &mut visit);
                }
            }
        }
        for &t_idx in &self.changed_complements {
            if visited.insert(t_idx) {
                self.visit_complement_slot(t_idx, &mut visit);
            }
        }
    }

    fn for_each_key_result<Visit>(&self, key: &GK, mut visit: Visit)
    where
        Visit: FnMut(&R),
    {
        let hash = key_hash(key);
        let Some(group_id) = self.find_group(hash, key) else {
            return;
        };
        let Some(indices) = self.t_by_group.get(&group_id) else {
            return;
        };
        let group = &self.groups[group_id];
        for &t_idx in indices {
            if group.count > 0 {
                group.accumulator.with_result(|result| visit(result));
            } else if let Some(default_result) = self.t_defaults.get(&t_idx) {
                visit(default_result);
            }
        }
    }

    fn complement_count(&self) -> usize {
        self.t_index_to_group.len()
    }
}
