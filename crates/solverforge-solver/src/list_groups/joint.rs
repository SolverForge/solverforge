//! Joint placement of a group's members into one list.
//!
//! A placement is the members' final positions in the destination list,
//! strictly increasing in declared order: inserting the members in that
//! order at those positions produces exactly that list. Other elements may
//! sit between members.

use std::num::NonZeroU8;

use smallvec::SmallVec;

/// Final positions of a group's members in one destination list.
pub(crate) type JointPositions = SmallVec<[usize; 4]>;

/// Member distance of the default grouped ruin and recreate.
pub(crate) const DEFAULT_GROUP_MEMBER_DISTANCE: NonZeroU8 = NonZeroU8::new(7).unwrap();
/// Owner shortlist of the default grouped ruin and recreate, and of the
/// default construction of a list with element groups.
pub(crate) const DEFAULT_GROUP_OWNERS: NonZeroU8 = NonZeroU8::new(4).unwrap();

/// Bounds on the joint placements tried for one group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
///
/// Stored as the configured bytes, so specs and plans holding it keep their
/// size.
pub(crate) struct JointBounds {
    /// Most positions from one member to the next; `None` tries every
    /// placement.
    max_member_distance: Option<NonZeroU8>,
    /// Most destination owners tried with every placement. When it is
    /// smaller than the candidate owners, each owner first gets the group
    /// placed contiguously at every position, and only the owners with the
    /// best contiguous score go on to the full search.
    max_owners: Option<NonZeroU8>,
}

impl JointBounds {
    /// Bounds from the configured member distance and owner shortlist.
    pub(crate) fn from_config(
        max_member_distance: Option<NonZeroU8>,
        max_group_owners: Option<NonZeroU8>,
    ) -> Self {
        Self {
            max_member_distance,
            max_owners: max_group_owners,
        }
    }

    /// Most other elements between two consecutive members.
    pub(crate) fn max_member_gap(self) -> Option<usize> {
        self.max_member_distance.map(member_gap)
    }

    /// Most owners tried with every placement.
    pub(crate) fn max_owners(self) -> Option<usize> {
        self.max_owners.map(|owners| usize::from(owners.get()))
    }
}

/// The other elements allowed between two members `distance` positions apart.
pub(crate) fn member_gap(distance: NonZeroU8) -> usize {
    usize::from(distance.get()) - 1
}

/// Every joint placement of `member_count` members into a list holding
/// `list_len` other elements, in lexicographic order of positions.
#[derive(Debug)]
pub(crate) struct JointPlacements {
    list_len: usize,
    max_gap: usize,
    positions: JointPositions,
    started: bool,
}

impl JointPlacements {
    pub(crate) fn new(member_count: usize, list_len: usize, max_member_gap: Option<usize>) -> Self {
        Self {
            list_len,
            max_gap: max_member_gap.unwrap_or(usize::MAX),
            positions: (0..member_count).collect(),
            started: false,
        }
    }

    /// The next placement, or `None` once every placement was yielded.
    #[allow(clippy::should_implement_trait)]
    pub(crate) fn next(&mut self) -> Option<&[usize]> {
        let count = self.positions.len();
        if count == 0 {
            return None;
        }
        if !self.started {
            self.started = true;
            return Some(&self.positions);
        }
        // The last member may sit at most `list_len + i` for member `i`.
        let mut index = count;
        while index > 0 {
            index -= 1;
            let raised = self.positions[index] + 1;
            let within_list = raised <= self.list_len + index;
            let within_gap = index == 0 || raised - self.positions[index - 1] - 1 <= self.max_gap;
            if within_list && within_gap {
                self.positions[index] = raised;
                for later in index + 1..count {
                    self.positions[later] = self.positions[later - 1] + 1;
                }
                return Some(&self.positions);
            }
        }
        None
    }

    /// Whether a placement keeps every member next to the previous one.
    pub(crate) fn is_contiguous(positions: &[usize]) -> bool {
        positions.windows(2).all(|pair| pair[1] == pair[0] + 1)
    }
}

/// The best placement found for a group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JointChoice<Sc> {
    pub(crate) entity: usize,
    pub(crate) positions: JointPositions,
    pub(crate) score: Sc,
}

/// Outcome of a joint placement search.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum JointSearch<Sc> {
    /// `eval` asked to stop.
    Interrupted,
    /// Every bounded placement was scored: the best one, and the best score
    /// of every other placement (for regret).
    Done {
        best: Option<JointChoice<Sc>>,
        second: Option<Sc>,
    },
}

struct Tracker<Sc> {
    best: Option<JointChoice<Sc>>,
    second: Option<Sc>,
}

impl<Sc: Copy + Ord> Tracker<Sc> {
    fn record(&mut self, entity: usize, positions: &[usize], score: Sc) {
        match &self.best {
            Some(best) if score <= best.score => {
                if self.second.is_none_or(|second| score > second) {
                    self.second = Some(score);
                }
            }
            _ => {
                if let Some(previous) = self.best.take() {
                    self.second = Some(previous.score);
                }
                self.best = Some(JointChoice {
                    entity,
                    positions: SmallVec::from_slice(positions),
                    score,
                });
            }
        }
    }
}

/// Scores the bounded joint placements of a `member_count`-member group on
/// `owners`, best first by score; ties keep the earliest placement.
///
/// `eval(entity, positions)` scores one placement, or returns `None` to stop
/// the search.
pub(crate) fn search_joint_placement<Sc: Copy + Ord>(
    owners: &[usize],
    member_count: usize,
    mut list_len: impl FnMut(usize) -> usize,
    bounds: JointBounds,
    mut eval: impl FnMut(usize, &[usize]) -> Option<Sc>,
) -> JointSearch<Sc> {
    let mut tracker = Tracker {
        best: None,
        second: None,
    };
    let shortlist = bounds
        .max_owners()
        .filter(|&max_owners| member_count > 1 && max_owners < owners.len());
    let Some(max_owners) = shortlist else {
        for &entity in owners {
            let mut placements =
                JointPlacements::new(member_count, list_len(entity), bounds.max_member_gap());
            while let Some(positions) = placements.next() {
                let Some(score) = eval(entity, positions) else {
                    return JointSearch::Interrupted;
                };
                tracker.record(entity, positions, score);
            }
        }
        return JointSearch::Done {
            best: tracker.best,
            second: tracker.second,
        };
    };

    // Contiguous placements rank the owners.
    let mut ranked: SmallVec<[(Sc, usize); 16]> = SmallVec::new();
    let mut positions = JointPositions::new();
    for (rank, &entity) in owners.iter().enumerate() {
        let len = list_len(entity);
        let mut owner_best = None;
        for start in 0..=len {
            positions.clear();
            positions.extend(start..start + member_count);
            let Some(score) = eval(entity, &positions) else {
                return JointSearch::Interrupted;
            };
            tracker.record(entity, &positions, score);
            if owner_best.is_none_or(|best| score > best) {
                owner_best = Some(score);
            }
        }
        if let Some(score) = owner_best {
            ranked.push((score, rank));
        }
    }
    ranked.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    ranked.truncate(max_owners);
    ranked.sort_by_key(|&(_, rank)| rank);
    for (_, rank) in ranked {
        let entity = owners[rank];
        let mut placements =
            JointPlacements::new(member_count, list_len(entity), bounds.max_member_gap());
        while let Some(positions) = placements.next() {
            if JointPlacements::is_contiguous(positions) {
                continue;
            }
            let Some(score) = eval(entity, positions) else {
                return JointSearch::Interrupted;
            };
            tracker.record(entity, positions, score);
        }
    }
    JointSearch::Done {
        best: tracker.best,
        second: tracker.second,
    }
}
