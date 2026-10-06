use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::stream::collection_extract::{ChangeSource, CollectionExtract};
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::relational::operator::{CollectionNode, JoinNode};
use crate::stream::relational::view_plan::{EntityKey, ViewEqualPlan};

use super::indexes::key_hash;

// Local alias so the long accumulator-token type stays readable in signatures.
pub(super) type CollectorRetraction<Acc, V, R> = <Acc as Accumulator<V, R>>::Retraction;

pub(super) struct GroupState<K, Acc> {
    pub(super) key: K,
    pub(super) accumulator: Acc,
    pub(super) count: usize,
}

/* One retained match: its (A index, B index) domain coordinates, the group it
feeds, and the accumulator token that retracts exactly this contributor. */
pub(super) struct MatchRow<Retraction> {
    pub(super) pair: (usize, usize),
    pub(super) group_id: usize,
    pub(super) retraction: Retraction,
}

/* Completed shared state for the cross-complemented-grouped family.

Match and filter retention belongs to the common operator tree: a `JoinNode`
over the A and B leaf collections, keyed by a typed view-equality plan, with
the authored residual predicate applied to each candidate pair. The
complement-side layer — target default values, target-to-group membership, and
group accumulators — stays local, because it is a real derived-state concern
shared by several dependent functions, not a duplicate join engine.
*/
pub struct ComplementedGroupedNodeState<
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
> where
    Acc: Accumulator<V, R>,
    JK: Eq + Hash + Clone,
{
    pub(super) extractor_t: ET,
    pub(super) filter: F,
    pub(super) group_key_fn: GF,
    pub(super) key_t: KT,
    pub(super) collector: C,
    pub(super) default_fn: D,
    pub(super) a_source: ChangeSource,
    pub(super) b_source: ChangeSource,
    pub(super) t_source: ChangeSource,
    /* Common join operator owns typed key indexing and candidate enumeration. */
    pub(super) join: JoinNode<
        S,
        CollectionNode<S, EA>,
        CollectionNode<S, EB>,
        ViewEqualPlan<JK, EntityKey<KA>, EntityKey<KB>>,
    >,
    pub(super) match_rows: Vec<MatchRow<CollectorRetraction<Acc, V, R>>>,
    /* Reverse contributor -> match-row links, used to retract a changed
    entity's rows without scanning the retained match set. */
    pub(super) a_to_matches: HashMap<usize, Vec<usize>>,
    pub(super) b_to_matches: HashMap<usize, Vec<usize>>,
    /* Complement layer: target membership and default results. */
    pub(super) t_by_group: HashMap<usize, Vec<usize>>,
    pub(super) t_index_to_group: HashMap<usize, usize>,
    pub(super) t_defaults: HashMap<usize, R>,
    pub(super) groups: Vec<GroupState<GK, Acc>>,
    pub(super) groups_by_hash: HashMap<u64, Vec<usize>>,
    pub(super) changed_groups: Vec<usize>,
    pub(super) changed_complements: Vec<usize>,
    pub(super) update_count: usize,
    pub(super) changed_key_count: usize,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> T, fn() -> V)>,
}

pub struct ComplementedGroupedEvaluationState<GK, V, R, Acc>
where
    Acc: Accumulator<V, R>,
{
    pub(super) groups: HashMap<GK, Acc>,
    pub(super) targets: Vec<(GK, R)>,
    pub(super) _phantom: PhantomData<fn() -> V>,
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
    ComplementedGroupedNodeState<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
where
    S: Send + Sync + 'static,
    A: 'static,
    B: 'static,
    T: Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Send + Sync,
    EA: CollectionExtract<S, Item = A> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    ET: CollectionExtract<S, Item = T> + 'static,
    KA: Fn(&A) -> JK + Send + Sync + 'static,
    KB: Fn(&B) -> JK + Send + Sync + 'static,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    KT: Fn(&T) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync,
    R: Send + Sync,
    Acc: Accumulator<V, R> + Send + Sync,
    D: Fn(&T) -> R + Send + Sync,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        extractor_a: EA,
        extractor_b: EB,
        extractor_t: ET,
        key_a: KA,
        key_b: KB,
        filter: F,
        group_key_fn: GF,
        key_t: KT,
        collector: C,
        default_fn: D,
    ) -> Self {
        let a_source = extractor_a.change_source();
        let b_source = extractor_b.change_source();
        let t_source = extractor_t.change_source();
        let join = JoinNode::new(
            CollectionNode::new(extractor_a, 1),
            CollectionNode::new(extractor_b, 0),
            ViewEqualPlan::new(EntityKey::new(key_a), EntityKey::new(key_b)),
        );
        Self {
            extractor_t,
            filter,
            group_key_fn,
            key_t,
            collector,
            default_fn,
            a_source,
            b_source,
            t_source,
            join,
            match_rows: Vec::new(),
            a_to_matches: HashMap::new(),
            b_to_matches: HashMap::new(),
            t_by_group: HashMap::new(),
            t_index_to_group: HashMap::new(),
            t_defaults: HashMap::new(),
            groups: Vec::new(),
            groups_by_hash: HashMap::new(),
            changed_groups: Vec::new(),
            changed_complements: Vec::new(),
            update_count: 0,
            changed_key_count: 0,
            _phantom: PhantomData,
        }
    }

    pub(super) fn mark_changed(&mut self, group_id: usize) {
        if !self.changed_groups.contains(&group_id) {
            self.changed_groups.push(group_id);
        }
    }

    pub(super) fn mark_complement_changed(&mut self, t_idx: usize) {
        if !self.changed_complements.contains(&t_idx) {
            self.changed_complements.push(t_idx);
        }
    }

    pub(super) fn insert_value(
        &mut self,
        key: GK,
        value: V,
    ) -> (usize, CollectorRetraction<Acc, V, R>) {
        let group_id = self.group_id_for_key(key);
        let group = &mut self.groups[group_id];
        if group.count == 0 {
            group.accumulator.reset();
        }
        let retraction = group.accumulator.accumulate(value);
        group.count += 1;
        self.mark_changed(group_id);
        (group_id, retraction)
    }

    pub(super) fn retract_value(
        &mut self,
        group_id: usize,
        retraction: CollectorRetraction<Acc, V, R>,
    ) {
        let Some(group) = self.groups.get_mut(group_id) else {
            return;
        };
        group.accumulator.retract(retraction);
        group.count = group.count.saturating_sub(1);
        self.mark_changed(group_id);
    }

    pub(super) fn group_id_for_key(&mut self, key: GK) -> usize {
        let hash = key_hash(&key);
        if let Some(group_id) = self.find_group(hash, &key) {
            return group_id;
        }
        let group_id = self.groups.len();
        self.groups.push(GroupState {
            key,
            accumulator: self.collector.create_accumulator(),
            count: 0,
        });
        self.groups_by_hash.entry(hash).or_default().push(group_id);
        group_id
    }
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
    ComplementedGroupedNodeState<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D>
where
    Acc: Accumulator<V, R>,
    GK: Eq + Hash,
    JK: Eq + Hash + Clone,
{
    pub(super) fn find_group(&self, hash: u64, key: &GK) -> Option<usize> {
        let group_ids = self.groups_by_hash.get(&hash)?;
        group_ids
            .iter()
            .copied()
            .find(|group_id| self.groups[*group_id].key == *key)
    }
}
