//! Immutable records of state-dependent default construction execution.
//!
//! The graph deliberately cannot predict this sequence from an initial
//! solution.  The executor records the exact order it resolved and whether
//! each reached child ran, had no remaining work, or was stopped, so the
//! runner can publish provenance without recreating default logic.

use crate::builder::RuntimeScalarSlotId;
use crate::runtime::compiler::{
    DefaultConstructionStage, DefaultConstructionStepKind, DefaultListPolicyProvenance,
};

/// What happened to one resolved construction child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedConstructionExecutionOutcome {
    Executed,
    SkippedNoWork,
    SkippedTerminated,
}

impl ResolvedConstructionExecutionOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Executed => "executed",
            Self::SkippedNoWork => "skipped_no_work",
            Self::SkippedTerminated => "skipped_terminated",
        }
    }
}

/// Result of one explicit or default-resolved construction node. This is the
/// shared dispatcher result: callers never infer termination from a restored
/// phase overlay or duplicate construction execution to learn what happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConstructionExecution {
    pub(crate) outcome: ResolvedConstructionExecutionOutcome,
}

impl ConstructionExecution {
    pub(crate) const fn executed() -> Self {
        Self {
            outcome: ResolvedConstructionExecutionOutcome::Executed,
        }
    }

    pub(crate) const fn skipped_no_work() -> Self {
        Self {
            outcome: ResolvedConstructionExecutionOutcome::SkippedNoWork,
        }
    }

    pub(crate) const fn skipped_terminated() -> Self {
        Self {
            outcome: ResolvedConstructionExecutionOutcome::SkippedTerminated,
        }
    }

    pub(crate) const fn ran(self) -> bool {
        matches!(self.outcome, ResolvedConstructionExecutionOutcome::Executed)
    }
}

/// One actual child of a default stage, retaining only immutable provenance
/// rather than its executable payload.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedConstructionExecutionStep {
    pub(crate) kind: DefaultConstructionStepKind,
    pub(crate) required_only: bool,
    pub(crate) target: Option<RuntimeScalarSlotId>,
    pub(crate) list_policies: Option<DefaultListPolicyProvenance>,
    pub(crate) outcome: ResolvedConstructionExecutionOutcome,
}

/// One default construction boundary after its solution-state-dependent
/// resolution and execution.
#[derive(Clone, Debug)]
pub(crate) struct DefaultConstructionStageExecutionRecord {
    pub(crate) stage: DefaultConstructionStage,
    pub(crate) outcome: ResolvedConstructionExecutionOutcome,
    pub(crate) steps: Vec<ResolvedConstructionExecutionStep>,
}

/// The complete ordered default-construction record for one reached runtime
/// phase.  The `ran_child_phase` flag preserves the existing construction
/// no-op/finalization decision without forcing the runner to infer it.
#[derive(Clone, Debug, Default)]
pub(crate) struct DefaultRuntimeConstructionExecution {
    pub(crate) ran_child_phase: bool,
    pub(crate) stages: Vec<DefaultConstructionStageExecutionRecord>,
}

impl DefaultRuntimeConstructionExecution {
    pub(super) fn push(&mut self, stage: DefaultConstructionStageExecutionRecord) {
        self.ran_child_phase |= matches!(
            stage.outcome,
            ResolvedConstructionExecutionOutcome::Executed
        );
        self.stages.push(stage);
    }

    pub(crate) fn outcome(&self) -> ResolvedConstructionExecutionOutcome {
        if self.ran_child_phase {
            ResolvedConstructionExecutionOutcome::Executed
        } else if self
            .stages
            .iter()
            .any(|stage| stage.outcome == ResolvedConstructionExecutionOutcome::SkippedTerminated)
        {
            ResolvedConstructionExecutionOutcome::SkippedTerminated
        } else {
            ResolvedConstructionExecutionOutcome::SkippedNoWork
        }
    }
}
