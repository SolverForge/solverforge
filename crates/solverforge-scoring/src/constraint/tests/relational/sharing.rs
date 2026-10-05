/* Compiler-owned derived-consumer sharing: one accumulation update per root event.

Multiple operator terminals consume one shared producer tree. The producer
(GroupNode or any derived row producer) owns its state exactly once; each
terminal keeps only its retained signed scores. One root `on_retract`/
`on_insert` routes through the shared producer once, then feeds every
terminal's score book from the same `RowChanges`. Scores, counts, and
explanations must agree with full recomputation, and weight behavior stays
independent per terminal.

No `Rc`/`Arc` and no public share API: sharing is a concrete struct owning
the producer by value plus a tuple of per-terminal score books. The macro
layer emits this shape for grouped bindings used by several terminals.
*/

use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::ConstraintSet as _;
use crate::constraint::relational::{OperatorConsumer, SharedOperatorSet};
use crate::stream::collection_extract::{source, ChangeSource, SourceExtract};
use crate::stream::collector::{count, CountAccumulator, CountCollector};
use crate::stream::relational::operator::{CollectionNode, GroupNode, Operator};
use crate::stream::relational::Leaf;

use super::fixtures::{RelAssignment, RelSchedule, RelShift};

type ShiftLeaf = CollectionNode<RelSchedule, SourceExtract<fn(&RelSchedule) -> &[RelShift]>>;
type ShiftGroup = GroupNode<
    ShiftLeaf,
    fn(&Leaf<'_, RelShift>) -> u32,
    CountCollector,
    u32,
    CountAccumulator,
    (),
    usize,
>;

fn leaf() -> ShiftLeaf {
    CollectionNode::new(
        source(
            rel_shifts_pub as fn(&RelSchedule) -> &[RelShift],
            ChangeSource::Descriptor(0),
        ),
        0,
    )
}

pub(super) fn rel_shifts_pub(schedule: &RelSchedule) -> &[RelShift] {
    schedule.shifts.as_slice()
}

fn leaf_key(row: &Leaf<'_, RelShift>) -> u32 {
    row.entity.id
}

fn count_weight(
    _: &RelSchedule,
    row: &<ShiftGroup as Operator<RelSchedule>>::View<'_>,
) -> SoftScore {
    row.with_result(|c| SoftScore::of(*c as i64))
}

fn double_weight(
    _: &RelSchedule,
    row: &<ShiftGroup as Operator<RelSchedule>>::View<'_>,
) -> SoftScore {
    row.with_result(|c| SoftScore::of(*c as i64 * 2))
}

fn two_shift_schedule() -> RelSchedule {
    RelSchedule {
        shifts: vec![
            RelShift { id: 1, night: true },
            RelShift { id: 1, night: true },
            RelShift { id: 2, night: true },
        ],
        assignments: Vec::<RelAssignment>::new(),
        employees: Vec::new(),
        score: None,
    }
}

/* The accumulation updates exactly once: one root event changes the count
group once, and both terminals' books observe the same final replacement.
Full recomputation must agree after every step. */
#[test]
fn shared_operator_set_updates_once_for_two_terminals() {
    let group = GroupNode::new(leaf(), leaf_key, count());
    let consumers = (
        OperatorConsumer::new(
            ConstraintRef::new("", "shared-a"),
            ImpactType::Reward,
            count_weight,
            false,
        ),
        OperatorConsumer::new(
            ConstraintRef::new("", "shared-b"),
            ImpactType::Reward,
            count_weight,
            false,
        ),
    );
    let mut set = SharedOperatorSet::new(group, consumers);
    let mut schedule = two_shift_schedule();

    // Group key 1 -> count 2 (weight 2), key 2 -> count 1. Total (2+1) * 2 terminals.
    assert_eq!(set.evaluate_all(&schedule), SoftScore::of(6));
    assert_eq!(set.initialize_all(&schedule), SoftScore::of(6));
    assert!(set.is_initialized());

    // Retract key-1 contributor at index 1: group 1 drops to count 1. One
    // replacement row per terminal: -1 each.
    let delta = set.on_retract_all(&schedule, 1, 0);
    assert_eq!(delta, SoftScore::of(-2));

    // Variable-change contract: mutate the retracted slot, re-insert it.
    // Key 1 -> key 9, so groups become {1:1, 2:1, 9:1}.
    schedule.shifts[1].id = 9;
    let delta = set.on_insert_all(&schedule, 1, 0);
    assert_eq!(delta, SoftScore::of(2));
    assert_eq!(set.evaluate_all(&schedule), SoftScore::of(6));
    assert_eq!(set.evaluate_each(&schedule)[0].match_count, 3);
    assert_eq!(set.constraint_count(), 2);
    assert_eq!(set.constraint_metadata().len(), 2);

    // Explanations carry one entity per group contributor.
    let detailed = set.evaluate_detailed(&schedule);
    assert_eq!(detailed.len(), 2);
    assert_eq!(detailed[0].matches.len(), 3);
    assert!(detailed[0]
        .matches
        .iter()
        .all(|m| m.justification.entities.len() == 1));

    set.reset_all();
    assert!(!set.is_initialized());
    assert_eq!(set.evaluate_all(&schedule), SoftScore::of(6));

    // Reset -> initialize parity.
    assert_eq!(set.initialize_all(&schedule), SoftScore::of(6));
}

/* Each terminal keeps independent weight behavior over the same rows. */
#[test]
fn shared_operator_set_terminals_score_independently() {
    let group = GroupNode::new(leaf(), leaf_key, count());
    let consumers = (
        OperatorConsumer::new(
            ConstraintRef::new("", "count"),
            ImpactType::Reward,
            count_weight,
            false,
        ),
        OperatorConsumer::new(
            ConstraintRef::new("", "double"),
            ImpactType::Reward,
            double_weight,
            false,
        ),
    );
    let mut set = SharedOperatorSet::new(group, consumers);
    let mut schedule = RelSchedule {
        shifts: vec![
            RelShift { id: 7, night: true },
            RelShift { id: 7, night: true },
        ],
        assignments: Vec::<RelAssignment>::new(),
        employees: Vec::new(),
        score: None,
    };
    // Terminal A counts 2; terminal B doubles it to 4.
    assert_eq!(set.initialize_all(&schedule), SoftScore::of(6));
    let results = set.evaluate_each(&schedule);
    assert_eq!(results[0].score, SoftScore::of(2));
    assert_eq!(results[1].score, SoftScore::of(4));

    // Retract one contributor: one shared replacement, -1 and -2.
    assert_eq!(set.on_retract_all(&schedule, 1, 0), SoftScore::of(-3));
    schedule.shifts.remove(1);
    let results = set.evaluate_each(&schedule);
    assert_eq!(results[0].score, SoftScore::of(1));
    assert_eq!(results[1].score, SoftScore::of(2));
    assert_eq!(set.evaluate_all(&schedule), SoftScore::of(3));

    // Explanations still carry the surviving contributor's entity.
    let detailed = set.evaluate_detailed(&schedule);
    assert_eq!(detailed[0].matches.len(), 1);
    assert_eq!(detailed[0].matches[0].justification.entities.len(), 1);
    let _ = ImpactType::Penalty;
}
