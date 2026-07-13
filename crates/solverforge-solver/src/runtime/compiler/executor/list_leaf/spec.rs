use std::fmt;

use smallvec::SmallVec;
use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::list_kernel::{
    ChangeCoordinates, MultiSwapCoordinates, PermuteCoordinates, ReverseCoordinates, RuinSources,
    SwapCoordinates,
};
use crate::heuristic::r#move::{
    k_opt_reconnection::{enumerate_reconnections, KOptReconnection, THREE_OPT_RECONNECTIONS},
    CutPoint, MAX_LIST_PERMUTE_WINDOW_SIZE,
};
use crate::heuristic::r#move::{SegmentRelocationCoords, SegmentSwapCoords};
use crate::runtime::compiler::graph::ListLeafKind;
use crate::runtime::compiler::types::CompiledListSlot;

use super::move_access::RuntimeListMoveAccess;

/// Immutable, capability-validated input for one compiled list leaf.
#[derive(Clone)]
pub(crate) struct RuntimeListNeighborhoodPlan<S, V, DM, IDM> {
    pub(super) kind: ListLeafKind,
    pub(super) spec: RuntimeListNeighborhoodSpec,
    pub(super) slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
    pub(super) random_seed: Option<u64>,
    /// Reconnection alternatives are structural compiled data, not cursor or
    /// solve state. Keeping them with the plan makes the selector facade a
    /// pure immutable opener.
    pub(super) kopt_patterns: Vec<KOptReconnection>,
}

impl<S, V, DM, IDM> fmt::Debug for RuntimeListNeighborhoodPlan<S, V, DM, IDM> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeListNeighborhoodPlan")
            .field("kind", &self.kind)
            .field("spec", &self.spec)
            .field("slot_count", &self.slots.len())
            .field("random_seed", &self.random_seed)
            .field("kopt_pattern_count", &self.kopt_patterns.len())
            .finish()
    }
}

/// Family-specific settings copied directly from the compiled selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeListNeighborhoodSpec {
    Change,
    NearbyChange {
        max_nearby: usize,
    },
    Swap,
    Permute {
        min_window_size: usize,
        max_window_size: usize,
    },
    Precedence,
    NearbySwap {
        max_nearby: usize,
    },
    SublistChange {
        min_sublist_size: usize,
        max_sublist_size: usize,
    },
    SublistSwap {
        min_sublist_size: usize,
        max_sublist_size: usize,
    },
    Reverse,
    KOpt {
        k: usize,
        min_segment_len: usize,
        max_nearby: usize,
    },
    Ruin {
        min_ruin_count: usize,
        max_ruin_count: usize,
        moves_per_step: usize,
        max_source_list_len: Option<usize>,
        skip_empty_destinations: bool,
    },
}

/// One owned move recipe emitted by the common list cursor.
#[derive(Clone, Debug)]
pub(crate) enum RuntimeListRecipe<S, V> {
    Change {
        access: RuntimeListMoveAccess<S, V>,
        coordinates: ChangeCoordinates,
    },
    Swap {
        access: RuntimeListMoveAccess<S, V>,
        coordinates: SwapCoordinates,
    },
    Permute {
        access: RuntimeListMoveAccess<S, V>,
        coordinates: PermuteCoordinates,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
        inverse_permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    },
    Reverse {
        access: RuntimeListMoveAccess<S, V>,
        coordinates: ReverseCoordinates,
    },
    SublistChange {
        access: RuntimeListMoveAccess<S, V>,
        coordinates: SegmentRelocationCoords,
    },
    SublistSwap {
        access: RuntimeListMoveAccess<S, V>,
        coordinates: SegmentSwapCoords,
    },
    KOpt {
        access: RuntimeListMoveAccess<S, V>,
        entity: usize,
        cuts: SmallVec<[CutPoint; 5]>,
        reconnection: KOptReconnection,
        variable_id: u64,
    },
    Ruin {
        access: RuntimeListMoveAccess<S, V>,
        sources: RuinSources,
        skip_empty_destinations: bool,
    },
    MultiSwap {
        access: RuntimeListMoveAccess<S, V>,
        coordinates: MultiSwapCoordinates,
        require_score_improvement: bool,
    },
}

impl<S, V, DM, IDM> RuntimeListNeighborhoodPlan<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    pub(crate) fn from_compiled(
        kind: ListLeafKind,
        config: &MoveSelectorConfig,
        slots: Vec<CompiledListSlot<S, V, DM, IDM>>,
        random_seed: Option<u64>,
    ) -> Result<Self, RuntimeListNeighborhoodPlanError> {
        if slots.is_empty() {
            return Err(RuntimeListNeighborhoodPlanError::EmptySlots { kind });
        }
        let spec = RuntimeListNeighborhoodSpec::from_config(kind, config)?;
        let kopt_patterns = match spec {
            RuntimeListNeighborhoodSpec::KOpt { k: 3, .. } => THREE_OPT_RECONNECTIONS.to_vec(),
            RuntimeListNeighborhoodSpec::KOpt { k, .. } => enumerate_reconnections(k),
            _ => Vec::new(),
        };
        Ok(Self {
            kind,
            spec,
            slots,
            random_seed,
            kopt_patterns,
        })
    }

    pub(crate) fn kind(&self) -> ListLeafKind {
        self.kind
    }

    pub(crate) fn slots(&self) -> &[CompiledListSlot<S, V, DM, IDM>] {
        &self.slots
    }

    #[cfg(test)]
    pub(crate) fn random_seed(&self) -> Option<u64> {
        self.random_seed
    }
}

impl RuntimeListNeighborhoodSpec {
    fn from_config(
        kind: ListLeafKind,
        config: &MoveSelectorConfig,
    ) -> Result<Self, RuntimeListNeighborhoodPlanError> {
        let spec = match (kind, config) {
            (ListLeafKind::Change, MoveSelectorConfig::ListChangeMoveSelector(_)) => Self::Change,
            (
                ListLeafKind::NearbyChange,
                MoveSelectorConfig::NearbyListChangeMoveSelector(config),
            ) => Self::NearbyChange {
                max_nearby: config.max_nearby,
            },
            (ListLeafKind::Swap, MoveSelectorConfig::ListSwapMoveSelector(_)) => Self::Swap,
            (ListLeafKind::Permute, MoveSelectorConfig::ListPermuteMoveSelector(config)) => {
                validate_permute_bounds(config.min_window_size, config.max_window_size)?;
                Self::Permute {
                    min_window_size: config.min_window_size,
                    max_window_size: config.max_window_size,
                }
            }
            (ListLeafKind::Precedence, MoveSelectorConfig::ListPrecedenceMoveSelector(_)) => {
                Self::Precedence
            }
            (ListLeafKind::NearbySwap, MoveSelectorConfig::NearbyListSwapMoveSelector(config)) => {
                Self::NearbySwap {
                    max_nearby: config.max_nearby,
                }
            }
            (
                ListLeafKind::SublistChange,
                MoveSelectorConfig::SublistChangeMoveSelector(config),
            ) => {
                validate_sublist_bounds(config.min_sublist_size, config.max_sublist_size)?;
                Self::SublistChange {
                    min_sublist_size: config.min_sublist_size,
                    max_sublist_size: config.max_sublist_size,
                }
            }
            (ListLeafKind::SublistSwap, MoveSelectorConfig::SublistSwapMoveSelector(config)) => {
                validate_sublist_bounds(config.min_sublist_size, config.max_sublist_size)?;
                Self::SublistSwap {
                    min_sublist_size: config.min_sublist_size,
                    max_sublist_size: config.max_sublist_size,
                }
            }
            (ListLeafKind::Reverse, MoveSelectorConfig::ListReverseMoveSelector(_)) => {
                Self::Reverse
            }
            (ListLeafKind::KOpt, MoveSelectorConfig::KOptMoveSelector(config)) => {
                if !(2..=5).contains(&config.k) {
                    return Err(RuntimeListNeighborhoodPlanError::InvalidKOptK { k: config.k });
                }
                Self::KOpt {
                    k: config.k,
                    min_segment_len: config.min_segment_len,
                    max_nearby: config.max_nearby,
                }
            }
            (ListLeafKind::Ruin, MoveSelectorConfig::ListRuinMoveSelector(config)) => {
                if config.min_ruin_count == 0 || config.max_ruin_count < config.min_ruin_count {
                    return Err(RuntimeListNeighborhoodPlanError::InvalidRuinBounds {
                        min_ruin_count: config.min_ruin_count,
                        max_ruin_count: config.max_ruin_count,
                    });
                }
                Self::Ruin {
                    min_ruin_count: config.min_ruin_count,
                    max_ruin_count: config.max_ruin_count,
                    moves_per_step: config.moves_per_step.unwrap_or(10),
                    max_source_list_len: config.max_source_list_len,
                    skip_empty_destinations: config.skip_empty_destinations,
                }
            }
            _ => return Err(RuntimeListNeighborhoodPlanError::ConfigFamilyMismatch { kind }),
        };
        Ok(spec)
    }
}

fn validate_permute_bounds(
    min_window_size: usize,
    max_window_size: usize,
) -> Result<(), RuntimeListNeighborhoodPlanError> {
    if min_window_size < 2
        || max_window_size < min_window_size
        || max_window_size > MAX_LIST_PERMUTE_WINDOW_SIZE
    {
        return Err(RuntimeListNeighborhoodPlanError::InvalidPermuteBounds {
            min_window_size,
            max_window_size,
        });
    }
    Ok(())
}

fn validate_sublist_bounds(
    min_sublist_size: usize,
    max_sublist_size: usize,
) -> Result<(), RuntimeListNeighborhoodPlanError> {
    if min_sublist_size == 0 || max_sublist_size < min_sublist_size {
        return Err(RuntimeListNeighborhoodPlanError::InvalidSublistBounds {
            min_sublist_size,
            max_sublist_size,
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeListNeighborhoodPlanError {
    EmptySlots {
        kind: ListLeafKind,
    },
    ConfigFamilyMismatch {
        kind: ListLeafKind,
    },
    InvalidPermuteBounds {
        min_window_size: usize,
        max_window_size: usize,
    },
    InvalidSublistBounds {
        min_sublist_size: usize,
        max_sublist_size: usize,
    },
    InvalidKOptK {
        k: usize,
    },
    InvalidRuinBounds {
        min_ruin_count: usize,
        max_ruin_count: usize,
    },
}

impl fmt::Display for RuntimeListNeighborhoodPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySlots { kind } => {
                write!(formatter, "list neighborhood {kind:?} has no slots")
            }
            Self::ConfigFamilyMismatch { kind } => write!(
                formatter,
                "list neighborhood {kind:?} received another selector config family"
            ),
            Self::InvalidPermuteBounds {
                min_window_size,
                max_window_size,
            } => write!(
                formatter,
                "list permute bounds require 2 <= min <= max <= {MAX_LIST_PERMUTE_WINDOW_SIZE}, got {min_window_size}..={max_window_size}"
            ),
            Self::InvalidSublistBounds {
                min_sublist_size,
                max_sublist_size,
            } => write!(
                formatter,
                "sublist bounds require 1 <= min <= max, got {min_sublist_size}..={max_sublist_size}"
            ),
            Self::InvalidKOptK { k } => write!(formatter, "k-opt requires 2 <= k <= 5, got {k}"),
            Self::InvalidRuinBounds {
                min_ruin_count,
                max_ruin_count,
            } => write!(
                formatter,
                "list ruin bounds require 1 <= min <= max, got {min_ruin_count}..={max_ruin_count}"
            ),
        }
    }
}

impl std::error::Error for RuntimeListNeighborhoodPlanError {}
