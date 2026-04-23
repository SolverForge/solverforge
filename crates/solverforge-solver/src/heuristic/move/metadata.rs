use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use smallvec::{smallvec, SmallVec};

pub const NONE_ID: u64 = u64::MAX;
pub type MoveIdentity = SmallVec<[u64; 8]>;

/// Canonical tabu metadata for one move candidate.
///
/// The canonical local-search path uses this structure to drive all tabu
/// dimensions from one source of truth:
/// - `entity_tokens` for entity tabu
/// - `destination_value_tokens` for value tabu
/// - `move_id` for exact move tabu
/// - `undo_move_id` for reverse-move tabu
///
/// Exact-move identities intentionally stay as ordered raw components rather
/// than pre-hashed scalars so exact and reverse memories retain their original
/// structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MoveTabuScope {
    pub descriptor_index: usize,
    pub variable_name: &'static str,
}

impl MoveTabuScope {
    pub const fn new(descriptor_index: usize, variable_name: &'static str) -> Self {
        Self {
            descriptor_index,
            variable_name,
        }
    }

    pub const fn entity_token(self, entity_id: u64) -> ScopedEntityTabuToken {
        ScopedEntityTabuToken {
            scope: self,
            entity_id,
        }
    }

    pub const fn value_token(self, value_id: u64) -> ScopedValueTabuToken {
        ScopedValueTabuToken {
            scope: self,
            value_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopedEntityTabuToken {
    pub scope: MoveTabuScope,
    pub entity_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopedValueTabuToken {
    pub scope: MoveTabuScope,
    pub value_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveTabuSignature {
    pub scope: MoveTabuScope,
    pub entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]>,
    pub destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]>,
    pub move_id: MoveIdentity,
    pub undo_move_id: MoveIdentity,
}

impl MoveTabuSignature {
    pub fn new(scope: MoveTabuScope, move_id: MoveIdentity, undo_move_id: MoveIdentity) -> Self {
        Self {
            scope,
            entity_tokens: SmallVec::new(),
            destination_value_tokens: SmallVec::new(),
            move_id,
            undo_move_id,
        }
    }

    pub fn with_entity_tokens<I>(mut self, entity_tokens: I) -> Self
    where
        I: IntoIterator<Item = ScopedEntityTabuToken>,
    {
        self.entity_tokens = entity_tokens.into_iter().collect();
        self
    }

    pub fn with_destination_value_tokens<I>(mut self, destination_value_tokens: I) -> Self
    where
        I: IntoIterator<Item = ScopedValueTabuToken>,
    {
        self.destination_value_tokens = destination_value_tokens.into_iter().collect();
        self
    }
}

pub(crate) fn encode_usize(value: usize) -> u64 {
    value as u64
}

pub(crate) fn encode_option_usize(value: Option<usize>) -> u64 {
    value.map_or(NONE_ID, encode_usize)
}

pub(crate) fn hash_str(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn hash_debug<T: Debug>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{value:?}").hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn encode_option_debug<T: Debug>(value: Option<&T>) -> u64 {
    value.map_or(NONE_ID, hash_debug)
}

pub(crate) const TABU_OP_SWAP: u64 = 0xF000_0000_0000_0001;
pub(crate) const TABU_OP_PILLAR_SWAP: u64 = 0xF000_0000_0000_0002;
pub(crate) const TABU_OP_LIST_SWAP: u64 = 0xF000_0000_0000_0003;
pub(crate) const TABU_OP_LIST_REVERSE: u64 = 0xF000_0000_0000_0004;

pub(crate) fn scoped_move_identity(
    scope: MoveTabuScope,
    operation_id: u64,
    components: impl IntoIterator<Item = u64>,
) -> MoveIdentity {
    let mut identity = smallvec![
        operation_id,
        encode_usize(scope.descriptor_index),
        hash_str(scope.variable_name),
    ];
    identity.extend(components);
    identity
}

pub(crate) fn ordered_coordinate_pair(first: (u64, u64), second: (u64, u64)) -> [(u64, u64); 2] {
    if first <= second {
        [first, second]
    } else {
        [second, first]
    }
}

pub(crate) fn append_canonical_usize_slice_pair(
    identity: &mut MoveIdentity,
    left: &[usize],
    right: &[usize],
) {
    let (first, second) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    identity.push(encode_usize(first.len()));
    identity.push(encode_usize(second.len()));
    identity.extend(first.iter().map(|&idx| encode_usize(idx)));
    identity.push(NONE_ID);
    identity.extend(second.iter().map(|&idx| encode_usize(idx)));
}

fn append_unique_tokens<T>(target: &mut SmallVec<[T; 2]>, tokens: &[T])
where
    T: Copy + PartialEq,
{
    for &token in tokens {
        if !target.contains(&token) {
            target.push(token);
        }
    }
}

pub(crate) fn compose_sequential_tabu_signature(
    prefix: &'static str,
    left: &MoveTabuSignature,
    right: &MoveTabuSignature,
) -> MoveTabuSignature {
    let mut entity_tokens = left.entity_tokens.clone();
    append_unique_tokens(&mut entity_tokens, &right.entity_tokens);

    let mut destination_value_tokens = left.destination_value_tokens.clone();
    append_unique_tokens(
        &mut destination_value_tokens,
        &right.destination_value_tokens,
    );

    let mut move_id = smallvec![hash_str(prefix)];
    move_id.extend(left.move_id.iter().copied());
    move_id.extend(right.move_id.iter().copied());

    let mut undo_move_id = smallvec![hash_str(prefix)];
    undo_move_id.extend(right.undo_move_id.iter().copied());
    undo_move_id.extend(left.undo_move_id.iter().copied());

    let scope = if left.scope == right.scope {
        left.scope
    } else {
        MoveTabuScope::new(left.scope.descriptor_index, prefix)
    };

    MoveTabuSignature::new(scope, move_id, undo_move_id)
        .with_entity_tokens(entity_tokens)
        .with_destination_value_tokens(destination_value_tokens)
}
