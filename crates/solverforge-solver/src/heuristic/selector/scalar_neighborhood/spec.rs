use std::fmt;

use solverforge_config::{MoveSelectorConfig, RecreateHeuristicType};
use solverforge_core::domain::PlanningSolution;

use crate::builder::{RuntimeScalarSlot, RuntimeScalarSlotId, ScalarAccessCapability};

/// One non-grouped scalar neighborhood family.
///
/// This discriminator belongs to the shared leaf kernel rather than a
/// compiler/executor module: static facades, dynamic facades, and compiled
/// declarations all describe the same seven families.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarNeighborhoodKind {
    Change,
    Swap,
    NearbyChange,
    NearbySwap,
    PillarChange,
    PillarSwap,
    RuinRecreate,
}

impl ScalarNeighborhoodKind {
    pub(crate) fn required_capability(self) -> Option<ScalarAccessCapability> {
        match self {
            Self::Change | Self::PillarChange | Self::PillarSwap | Self::RuinRecreate => {
                Some(ScalarAccessCapability::Candidates)
            }
            Self::NearbyChange => Some(ScalarAccessCapability::NearbyValue),
            Self::NearbySwap => Some(ScalarAccessCapability::NearbyEntity),
            Self::Swap => None,
        }
    }

    pub(crate) fn selector_name(self) -> &'static str {
        match self {
            Self::Change => "change_move_selector",
            Self::Swap => "swap_move_selector",
            Self::NearbyChange => "nearby_change_move_selector",
            Self::NearbySwap => "nearby_swap_move_selector",
            Self::PillarChange => "pillar_change_move_selector",
            Self::PillarSwap => "pillar_swap_move_selector",
            Self::RuinRecreate => "ruin_recreate_move_selector",
        }
    }
}

/// Frozen settings for one scalar leaf. It excludes target matching and
/// composition: those belong to the model/compiler and generic composer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarNeighborhoodSpec {
    Change {
        value_candidate_limit: Option<usize>,
    },
    Swap,
    NearbyChange {
        max_nearby: usize,
        value_candidate_limit: Option<usize>,
    },
    NearbySwap {
        max_nearby: usize,
    },
    PillarChange {
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
        value_candidate_limit: Option<usize>,
    },
    PillarSwap {
        minimum_sub_pillar_size: usize,
        maximum_sub_pillar_size: usize,
    },
    RuinRecreate {
        min_ruin_count: usize,
        max_ruin_count: usize,
        moves_per_step: usize,
        value_candidate_limit: Option<usize>,
        recreate_heuristic_type: RecreateHeuristicType,
    },
}

impl ScalarNeighborhoodSpec {
    pub(crate) fn from_config(
        kind: ScalarNeighborhoodKind,
        config: &MoveSelectorConfig,
    ) -> Result<Self, ScalarNeighborhoodBindingError> {
        let spec = match (kind, config) {
            (ScalarNeighborhoodKind::Change, MoveSelectorConfig::ChangeMoveSelector(config)) => {
                Self::Change {
                    value_candidate_limit: config.value_candidate_limit,
                }
            }
            (ScalarNeighborhoodKind::Swap, MoveSelectorConfig::SwapMoveSelector(_)) => Self::Swap,
            (
                ScalarNeighborhoodKind::NearbyChange,
                MoveSelectorConfig::NearbyChangeMoveSelector(config),
            ) => Self::NearbyChange {
                max_nearby: config.max_nearby,
                value_candidate_limit: config.value_candidate_limit,
            },
            (
                ScalarNeighborhoodKind::NearbySwap,
                MoveSelectorConfig::NearbySwapMoveSelector(config),
            ) => Self::NearbySwap {
                max_nearby: config.max_nearby,
            },
            (
                ScalarNeighborhoodKind::PillarChange,
                MoveSelectorConfig::PillarChangeMoveSelector(config),
            ) => Self::PillarChange {
                minimum_sub_pillar_size: config.minimum_sub_pillar_size,
                maximum_sub_pillar_size: config.maximum_sub_pillar_size,
                value_candidate_limit: config.value_candidate_limit,
            },
            (
                ScalarNeighborhoodKind::PillarSwap,
                MoveSelectorConfig::PillarSwapMoveSelector(config),
            ) => Self::PillarSwap {
                minimum_sub_pillar_size: config.minimum_sub_pillar_size,
                maximum_sub_pillar_size: config.maximum_sub_pillar_size,
            },
            (
                ScalarNeighborhoodKind::RuinRecreate,
                MoveSelectorConfig::RuinRecreateMoveSelector(config),
            ) => {
                if config.min_ruin_count == 0 || config.max_ruin_count < config.min_ruin_count {
                    return Err(ScalarNeighborhoodBindingError::InvalidRuinBounds {
                        min_ruin_count: config.min_ruin_count,
                        max_ruin_count: config.max_ruin_count,
                    });
                }
                Self::RuinRecreate {
                    min_ruin_count: config.min_ruin_count,
                    max_ruin_count: config.max_ruin_count,
                    moves_per_step: config.moves_per_step.unwrap_or(10).max(1),
                    value_candidate_limit: config.value_candidate_limit,
                    recreate_heuristic_type: config.recreate_heuristic_type,
                }
            }
            _ => return Err(ScalarNeighborhoodBindingError::ConfigFamilyMismatch { kind }),
        };
        Ok(spec)
    }

    pub(crate) fn kind(self) -> ScalarNeighborhoodKind {
        match self {
            Self::Change { .. } => ScalarNeighborhoodKind::Change,
            Self::Swap => ScalarNeighborhoodKind::Swap,
            Self::NearbyChange { .. } => ScalarNeighborhoodKind::NearbyChange,
            Self::NearbySwap { .. } => ScalarNeighborhoodKind::NearbySwap,
            Self::PillarChange { .. } => ScalarNeighborhoodKind::PillarChange,
            Self::PillarSwap { .. } => ScalarNeighborhoodKind::PillarSwap,
            Self::RuinRecreate { .. } => ScalarNeighborhoodKind::RuinRecreate,
        }
    }
}

/// One selected scalar recipe. The recipe owns the frozen physical slot so a
/// selected candidate never borrows a cursor, callback view, or row buffer.
#[derive(Clone, Debug)]
pub(crate) enum RuntimeScalarRecipe<S> {
    Change {
        slot: RuntimeScalarSlot<S>,
        entity_index: usize,
        to_value: Option<usize>,
    },
    Swap {
        slot: RuntimeScalarSlot<S>,
        left_entity_index: usize,
        right_entity_index: usize,
    },
    PillarChange {
        slot: RuntimeScalarSlot<S>,
        entity_indices: Vec<usize>,
        to_value: Option<usize>,
    },
    PillarSwap {
        slot: RuntimeScalarSlot<S>,
        left_indices: Vec<usize>,
        right_indices: Vec<usize>,
    },
    RuinRecreate {
        slot: RuntimeScalarSlot<S>,
        entity_indices: Vec<usize>,
        value_candidate_limit: Option<usize>,
        recreate_heuristic_type: RecreateHeuristicType,
    },
}

/// Structural leaf-creation failures. Direct dynamic APIs and compiler
/// lowering use this same type so a missing nearby source never becomes an
/// alternate candidate universe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarNeighborhoodBindingError {
    ConfigFamilyMismatch {
        kind: ScalarNeighborhoodKind,
    },
    MissingCapability {
        slot: RuntimeScalarSlotId,
        capability: ScalarAccessCapability,
    },
    InvalidRuinBounds {
        min_ruin_count: usize,
        max_ruin_count: usize,
    },
}

impl ScalarNeighborhoodBindingError {
    pub(crate) fn validate_slot<S>(
        kind: ScalarNeighborhoodKind,
        slot: &RuntimeScalarSlot<S>,
    ) -> Result<(), Self>
    where
        S: PlanningSolution,
    {
        let Some(capability) = kind.required_capability() else {
            return Ok(());
        };
        if slot.has_capability(capability) {
            Ok(())
        } else {
            Err(Self::MissingCapability {
                slot: slot.id(),
                capability,
            })
        }
    }
}

impl fmt::Display for ScalarNeighborhoodBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigFamilyMismatch { kind } => {
                write!(
                    f,
                    "{} received another selector configuration family",
                    kind.selector_name()
                )
            }
            Self::MissingCapability { slot, capability } => {
                write!(
                    f,
                    "{slot} does not provide required {}",
                    capability_label(*capability)
                )
            }
            Self::InvalidRuinBounds {
                min_ruin_count,
                max_ruin_count,
            } => write!(
                f,
                "ruin_recreate bounds require 1 <= min <= max, got {min_ruin_count}..={max_ruin_count}"
            ),
        }
    }
}

impl std::error::Error for ScalarNeighborhoodBindingError {}

fn capability_label(capability: ScalarAccessCapability) -> &'static str {
    match capability {
        ScalarAccessCapability::Candidates => "scalar candidate source",
        ScalarAccessCapability::NearbyValue => "nearby scalar value source",
        ScalarAccessCapability::NearbyEntity => "nearby scalar entity source",
        ScalarAccessCapability::ConstructionEntityOrder => "construction entity-order source",
        ScalarAccessCapability::ConstructionValueOrder => "construction value-order source",
    }
}
