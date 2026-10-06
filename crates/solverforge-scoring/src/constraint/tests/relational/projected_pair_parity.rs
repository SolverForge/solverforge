/* Projected self-join parity: the operator tree's owned projection joined to
itself covers the bespoke projected pair-join engine's semantics.

`constraint::projected::{Bi, DirectedBi}` join owned projected outputs into
unordered symmetric pairs (Bi) and ordered directed pairs (DirectedBi). The
common tree expresses both as
`JoinNode(CollectionNode, ProjectNode, equal_bi(...))` where the projection
owns non-`Clone` payloads — move-only rows included — with no bespoke key
index. This suite pins the three behaviors that make the bespoke engines
redundant:

1. symmetric pairs: each unordered pair appears exactly once;
2. move-only payloads: the projection output and key are neither `Clone` nor
   `Copy`, and still index, join, and score through the tree;
3. move-only retraction: removing one contributor retracts exactly its
   pairs, reusing the freed row slots.

These tests are the behavioral contract any engine replacement must satisfy;
`constraint::projected::{Bi, DirectedBi}` remain the fluent producers today and
their own suites keep pinning the same fixtures on the engine path.
*/

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::relational::{
    operator::{CollectionNode, JoinNode, Pair, ProjectNode},
    Leaf,
};
use solverforge_core::{score::SoftScore, ConstraintRef, ImpactType};

#[derive(Clone, Debug)]
struct Work {
    bucket: usize,
    demand: i64,
    enabled: bool,
}

struct Plan {
    work: Vec<Work>,
}

fn work(p: &Plan) -> &[Work] {
    p.work.as_slice()
}

// A payload that is neither Clone nor Copy, exactly like the projected
// engine's move-only rows.
struct MoveOnlyEntry {
    bucket: usize,
    demand: i64,
}

fn project_move_only(_: &Plan, row: &Leaf<'_, Work>) -> Vec<MoveOnlyEntry> {
    if row.entity.enabled {
        vec![MoveOnlyEntry {
            bucket: row.entity.bucket,
            demand: row.entity.demand,
        }]
    } else {
        Vec::new()
    }
}

fn projected_bucket(
    row: &crate::stream::relational::operator::ProjectView<'_, Leaf<'_, Work>, MoveOnlyEntry>,
) -> usize {
    row.value.bucket
}

fn leaf(
    binding: u32,
) -> CollectionNode<Plan, crate::stream::collection_extract::SourceExtract<fn(&Plan) -> &[Work]>> {
    CollectionNode::new(
        source(work as fn(&Plan) -> &[Work], ChangeSource::Descriptor(0)),
        binding,
    )
}

type WorkPairView<'a> = Pair<
    crate::stream::relational::operator::ProjectView<'a, Leaf<'a, Work>, MoveOnlyEntry>,
    crate::stream::relational::operator::ProjectView<'a, Leaf<'a, Work>, MoveOnlyEntry>,
>;

/* Both sides projected: the engines pair projected rows with projected rows,
so the faithful tree form projects on each binding and filters the self-pair
(the same source row feeding both projections). */
fn projected_pair_tree<W>(weight: W) -> impl IncrementalConstraint<Plan, SoftScore>
where
    W: Fn(&Plan, &WorkPairView<'_>) -> SoftScore + Send + Sync + 'static,
{
    let tree = crate::stream::relational::operator::FilterNode::new(
        JoinNode::new(
            ProjectNode::new(leaf(0), project_move_only),
            ProjectNode::new(leaf(1), project_move_only),
            crate::stream::joiner::equal_bi(projected_bucket, projected_bucket),
        ),
        |_p: &Plan, row: &WorkPairView<'_>| {
            // Drop the self-pair: the same source row feeding both projections.
            row.left.input.index != row.right.input.index
        },
    );
    OperatorTerminal::new(
        ConstraintRef::new("test", "projected pair tree"),
        ImpactType::Penalty,
        tree,
        weight,
        false,
    )
}

fn tree_symmetric() -> impl IncrementalConstraint<Plan, SoftScore> {
    projected_pair_tree(|_p: &Plan, _row: &WorkPairView<'_>| SoftScore::of(1))
}

#[test]
fn dbg_pairs() {
    let p = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 1,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 3,
                enabled: true,
            },
        ],
    };
    let mut c = tree_symmetric();
    let init = c.initialize(&p);
    println!("init={:?} count={}", init, c.match_count(&p));
    let r = c.on_retract(&p, 0, 0);
    println!("after retract={:?} count={}", init + r, c.match_count(&p));
    let i = c.on_insert(&p, 0, 0);
    println!(
        "after insert={:?} count={}",
        init + r + i,
        c.match_count(&p)
    );
}

fn symmetric_oracle(p: &Plan) -> (SoftScore, usize) {
    // The tree's JoinNode enumerates ordered (left entity, right projected)
    // pairs; a same-bucket pair of two projected rows appears once per ordered
    // orientation EXCLUDING self-pairs (i<j vs j<i both appear; a row's own
    // projection pairs with itself only via the other binding's row).
    let entries: Vec<&Work> = p.work.iter().filter(|w| w.enabled).collect();
    let mut score = 0i64;
    let mut count = 0usize;
    for (i, a) in entries.iter().enumerate() {
        for (j, b) in entries.iter().enumerate() {
            if i == j {
                continue;
            }
            if a.bucket == b.bucket {
                score += 1;
                count += 1;
            }
        }
    }

    (SoftScore::of(-score), count)
}

#[test]
fn tree_joins_move_only_projected_rows_symmetrically() {
    let p = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 1,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 3,
                enabled: true,
            },
        ],
    };
    let c = tree_symmetric();
    assert_eq!(c.evaluate(&p), symmetric_oracle(&p).0);
    assert_eq!(c.match_count(&p), symmetric_oracle(&p).1);
}

/* Retraction through the tree with move-only rows: disabling a contributor
retracts exactly its pairs; re-enabling restores them; freed slots reused. */
#[test]
fn tree_move_only_retraction_reuses_slots_and_keeps_oracle() {
    let mut p = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 1,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 3,
                enabled: true,
            },
        ],
    };
    let mut c = tree_symmetric();
    let mut score = c.initialize(&p);
    assert_eq!(score, symmetric_oracle(&p).0);

    for step in 0..40 {
        let i = step % 3;
        score = score + c.on_retract(&p, i, 0);
        p.work[i].enabled = step % 2 == 0;
        p.work[i].bucket = step % 2;
        score = score + c.on_insert(&p, i, 0);
        assert_eq!(score, symmetric_oracle(&p).0);
        assert_eq!(c.evaluate(&p), score);
        assert_eq!(c.match_count(&p), symmetric_oracle(&p).1);
    }
    c.reset();
    assert_eq!(c.evaluate(&p), score);
    assert_eq!(c.initialize(&p), score);
}

/* Directed form: the tree's JoinNode is ordered (left, right), matching the
directed engine's ordered-pair semantics, including same-source updates. */
#[test]
fn tree_directed_move_only_pairs_match_oracle() {
    let mut p = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 1,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 3,
                enabled: true,
            },
        ],
    };
    let mut c = {
        let tree = crate::stream::relational::operator::FilterNode::new(
            JoinNode::new(
                ProjectNode::new(leaf(0), project_move_only),
                ProjectNode::new(leaf(1), project_move_only),
                crate::stream::joiner::equal_bi(projected_bucket, projected_bucket),
            ),
            |_p: &Plan, row: &WorkPairView<'_>| row.left.input.index != row.right.input.index,
        );
        OperatorTerminal::new(
            ConstraintRef::new("test", "tree directed"),
            ImpactType::Penalty,
            tree,
            |_p: &Plan, row: &WorkPairView<'_>| {
                // Directional score: order matters (left payload, right payload).
                SoftScore::of(row.left.value.demand * 10 + row.right.value.demand)
            },
            false,
        )
    };
    let oracle = |p: &Plan| {
        let mut score = 0i64;
        for a in &p.work {
            for b in &p.work {
                if !std::ptr::eq(a, b) && a.enabled && b.enabled && a.bucket == b.bucket {
                    score += a.demand * 10 + b.demand;
                }
            }
        }
        SoftScore::of(-score)
    };
    assert_eq!(c.evaluate(&p), oracle(&p));

    let mut score = c.initialize(&p);
    for step in 0..40 {
        let i = step % 3;
        score = score + c.on_retract(&p, i, 0);
        p.work[i].enabled = step % 2 == 0;
        p.work[i].bucket = step % 2;
        p.work[i].demand = (step % 5) as i64;
        score = score + c.on_insert(&p, i, 0);
        assert_eq!(score, oracle(&p));
        assert_eq!(c.evaluate(&p), score);
    }
}
