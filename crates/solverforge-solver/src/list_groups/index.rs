//! Declared element groups of one list variable, keyed by element source key.

use std::collections::HashMap;

/// The groups a list variable's `element_group_fn` declares.
///
/// Only groups of two or more members are kept: an element whose hook
/// answer is empty, or only the element itself, moves alone exactly as
/// without groups. Members keep the hook's order.
#[derive(Debug, Default)]
pub(crate) struct ListElementGroups {
    group_by_key: HashMap<usize, usize>,
    member_offsets: Vec<usize>,
    member_keys: Vec<usize>,
}

impl ListElementGroups {
    /// Reads every element's group through `members_of`.
    ///
    /// `element_at` yields the element of a source index, `key_of` its source
    /// key, and `members_of` appends the members of its group. Panics when
    /// two elements disagree about a group, because moves keyed on one
    /// answer would then split or duplicate members of the other.
    pub(crate) fn from_hook<E>(
        element_count: usize,
        mut element_at: impl FnMut(usize) -> Option<E>,
        mut key_of: impl FnMut(&E) -> usize,
        mut members_of: impl FnMut(&E, &mut Vec<E>),
    ) -> Self {
        let mut groups = Self::default();
        let mut scratch = Vec::new();
        let mut keys = Vec::new();
        for index in 0..element_count {
            let Some(element) = element_at(index) else {
                continue;
            };
            let key = key_of(&element);
            scratch.clear();
            members_of(&element, &mut scratch);
            keys.clear();
            keys.extend(scratch.iter().map(&mut key_of));
            if let Some(&group) = groups.group_by_key.get(&key) {
                assert!(
                    keys.as_slice() == groups.members(group),
                    "element group of element {key} lists members {keys:?}, but another member \
                     declared {:?}",
                    groups.members(group)
                );
                continue;
            }
            if keys.len() < 2 {
                assert!(
                    keys.is_empty() || keys[0] == key,
                    "element {key} declares a one-member group of element {}",
                    keys[0]
                );
                continue;
            }
            assert!(
                keys.contains(&key),
                "element group of element {key} does not contain it: {keys:?}"
            );
            let group = groups.member_offsets.len().saturating_sub(1);
            if groups.member_offsets.is_empty() {
                groups.member_offsets.push(0);
            }
            for (position, &member) in keys.iter().enumerate() {
                assert!(
                    !keys[..position].contains(&member),
                    "element group of element {key} lists member {member} twice"
                );
                let previous = groups.group_by_key.insert(member, group);
                assert!(
                    previous.is_none(),
                    "element {member} belongs to two element groups"
                );
            }
            groups.member_keys.extend_from_slice(&keys);
            groups.member_offsets.push(groups.member_keys.len());
        }
        groups
    }

    /// Whether no group has two or more members.
    pub(crate) fn is_empty(&self) -> bool {
        self.member_keys.is_empty()
    }

    /// Number of groups.
    pub(crate) fn group_count(&self) -> usize {
        self.member_offsets.len().saturating_sub(1)
    }

    /// The group holding the element with this source key.
    pub(crate) fn group_of_key(&self, key: usize) -> Option<usize> {
        self.group_by_key.get(&key).copied()
    }

    /// The source keys of a group's members, in declared order.
    pub(crate) fn members(&self, group: usize) -> &[usize] {
        &self.member_keys[self.member_offsets[group]..self.member_offsets[group + 1]]
    }

    /// Offset of a group's first member in the flat member order.
    pub(crate) fn member_offset(&self, group: usize) -> usize {
        self.member_offsets[group]
    }

    /// Total number of grouped elements.
    pub(crate) fn member_count(&self) -> usize {
        self.member_keys.len()
    }
}

/// Where every grouped element sits in the current solution.
///
/// Built once per cursor or phase from a [`ListElementGroups`]; slot `i`
/// holds the `(entity, position)` of the `i`-th member in the groups' flat
/// member order, or `None` while that member is unassigned.
#[derive(Debug, Default)]
pub(crate) struct ListGroupPlacement {
    member_positions: Vec<Option<(usize, usize)>>,
}

impl ListGroupPlacement {
    /// Scans every list once, through `key_at`, to locate each member.
    pub(crate) fn locate(
        groups: &ListElementGroups,
        entity_count: usize,
        mut list_len: impl FnMut(usize) -> usize,
        mut key_at: impl FnMut(usize, usize) -> Option<usize>,
    ) -> Self {
        let mut member_positions = vec![None; groups.member_count()];
        for entity in 0..entity_count {
            for position in 0..list_len(entity) {
                let Some(key) = key_at(entity, position) else {
                    continue;
                };
                let Some(group) = groups.group_of_key(key) else {
                    continue;
                };
                let offset = groups.member_offset(group);
                let member = groups
                    .members(group)
                    .iter()
                    .position(|&member| member == key)
                    .expect("a grouped key is one of its group's members");
                member_positions[offset + member] = Some((entity, position));
            }
        }
        Self { member_positions }
    }

    /// The `(entity, position)` of a group's `member`-th member.
    pub(crate) fn member_position(
        &self,
        groups: &ListElementGroups,
        group: usize,
        member: usize,
    ) -> Option<(usize, usize)> {
        self.member_positions[groups.member_offset(group) + member]
    }

    /// Whether every member of a group is assigned.
    pub(crate) fn group_is_assigned(&self, groups: &ListElementGroups, group: usize) -> bool {
        let offset = groups.member_offset(group);
        self.member_positions[offset..offset + groups.members(group).len()]
            .iter()
            .all(Option::is_some)
    }
}
