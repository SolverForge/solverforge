use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use smallvec::SmallVec;

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
