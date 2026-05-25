#[derive(Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnerRestriction {
    Unrestricted,
    Fixed(usize),
    Invalid,
}

impl Clone for OwnerRestriction {
    fn clone(&self) -> Self {
        *self
    }
}

impl OwnerRestriction {
    pub(crate) fn allows(self, entity_idx: usize) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::Fixed(owner_idx) => owner_idx == entity_idx,
            Self::Invalid => false,
        }
    }

    fn candidates(self, entity_count: usize) -> CandidateEntityIter {
        match self {
            Self::Unrestricted => CandidateEntityIter::All(0..entity_count),
            Self::Fixed(owner_idx) => CandidateEntityIter::One(owner_idx),
            Self::Invalid => CandidateEntityIter::Empty,
        }
    }
}

pub(crate) enum CandidateEntityIter {
    All(std::ops::Range<usize>),
    One(usize),
    Empty,
}

impl Iterator for CandidateEntityIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::All(range) => range.next(),
            Self::One(owner_idx) => {
                let owner_idx = *owner_idx;
                *self = Self::Empty;
                Some(owner_idx)
            }
            Self::Empty => None,
        }
    }
}

pub(crate) fn owner_restriction<S, V>(
    owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    solution: &S,
    entity_count: usize,
    element: &V,
) -> OwnerRestriction {
    let Some(owner_fn) = owner_fn else {
        return OwnerRestriction::Unrestricted;
    };

    match owner_fn(solution, element) {
        None => OwnerRestriction::Unrestricted,
        Some(owner_idx) if owner_idx < entity_count => OwnerRestriction::Fixed(owner_idx),
        Some(_) => OwnerRestriction::Invalid,
    }
}

pub(crate) fn candidate_entity_indices<S, V>(
    owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    solution: &S,
    entity_count: usize,
    element: &V,
) -> CandidateEntityIter {
    owner_restriction(owner_fn, solution, entity_count, element).candidates(entity_count)
}

pub(crate) fn owner_allows<S, V>(
    owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    solution: &S,
    entity_count: usize,
    entity_idx: usize,
    element: &V,
) -> bool {
    owner_restriction(owner_fn, solution, entity_count, element).allows(entity_idx)
}

pub(crate) fn route_owner_allows<S, V>(
    owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    solution: &S,
    entity_count: usize,
    entity_idx: usize,
    route: &[V],
) -> bool {
    let Some(owner_fn) = owner_fn else {
        return true;
    };

    route
        .iter()
        .all(|element| owner_allows(Some(owner_fn), solution, entity_count, entity_idx, element))
}

pub(crate) fn selected_owner_restrictions<S, V>(
    owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    solution: &S,
    entity_count: usize,
    entities: &[usize],
    route_lens: &[usize],
    list_get: fn(&S, usize, usize) -> Option<V>,
) -> Option<Vec<Vec<OwnerRestriction>>> {
    let owner_fn = owner_fn?;
    Some(
        entities
            .iter()
            .zip(route_lens.iter())
            .map(|(&entity_idx, &len)| {
                (0..len)
                    .map(|pos| {
                        list_get(solution, entity_idx, pos).map_or(
                            OwnerRestriction::Invalid,
                            |element| {
                                owner_restriction(Some(owner_fn), solution, entity_count, &element)
                            },
                        )
                    })
                    .collect()
            })
            .collect(),
    )
}

#[inline]
pub(crate) fn selected_owner_allows(
    owners: &[Vec<OwnerRestriction>],
    selected_entity_idx: usize,
    position: usize,
    dst_entity: usize,
) -> bool {
    owners
        .get(selected_entity_idx)
        .and_then(|entity_owners| entity_owners.get(position))
        .copied()
        .is_some_and(|restriction| restriction.allows(dst_entity))
}

#[inline]
pub(crate) fn selected_segment_allows(
    owners: &[Vec<OwnerRestriction>],
    selected_entity_idx: usize,
    start: usize,
    end: usize,
    dst_entity: usize,
) -> bool {
    let Some(entity_owners) = owners.get(selected_entity_idx) else {
        return false;
    };
    (start..end).all(|position| {
        entity_owners
            .get(position)
            .copied()
            .is_some_and(|restriction| restriction.allows(dst_entity))
    })
}

#[cfg(test)]
mod tests {
    use super::{candidate_entity_indices, owner_restriction, OwnerRestriction};

    fn fixed_owner(_: &(), element: &usize) -> Option<usize> {
        (*element == 7).then_some(1)
    }

    fn invalid_owner(_: &(), _: &usize) -> Option<usize> {
        Some(4)
    }

    #[test]
    fn absent_owner_hook_is_unrestricted() {
        assert_eq!(
            owner_restriction::<(), usize>(None, &(), 3, &7),
            OwnerRestriction::Unrestricted
        );
        assert_eq!(
            candidate_entity_indices::<(), usize>(None, &(), 3, &7).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn hook_returned_none_is_unrestricted() {
        assert_eq!(
            owner_restriction(Some(fixed_owner), &(), 3, &8),
            OwnerRestriction::Unrestricted
        );
        assert_eq!(
            candidate_entity_indices(Some(fixed_owner), &(), 3, &8).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn valid_owner_hook_result_is_fixed() {
        assert_eq!(
            owner_restriction(Some(fixed_owner), &(), 3, &7),
            OwnerRestriction::Fixed(1)
        );
        assert_eq!(
            candidate_entity_indices(Some(fixed_owner), &(), 3, &7).collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn out_of_range_owner_hook_result_is_invalid() {
        assert_eq!(
            owner_restriction(Some(invalid_owner), &(), 3, &7),
            OwnerRestriction::Invalid
        );
        assert_eq!(
            candidate_entity_indices(Some(invalid_owner), &(), 3, &7).collect::<Vec<_>>(),
            Vec::<usize>::new()
        );
    }
}
