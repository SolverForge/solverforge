use super::*;

/// The engine path that consumed a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateTraceSource {
    /// The construction heuristic phase.
    Construction,
    /// The local search phase.
    LocalSearch,
    /// The variable neighborhood descent phase.
    VariableNeighborhoodDescent,
    /// A k-opt move evaluation.
    KOpt,
    /// Round-robin list construction.
    ListRoundRobinConstruction,
    /// A cheapest-insertion trial in list construction.
    ListCheapestInsertionTrial,
    /// A regret-insertion trial in list construction.
    ListRegretInsertionTrial,
    /// A Clarke-Wright savings evaluation.
    ListClarkeWrightSavings,
    /// A Clarke-Wright route merge.
    ListClarkeWrightMerge,
    /// A Clarke-Wright completion insertion of elements left unrouted.
    ListClarkeWrightCompletionInsertion,
    /// A list k-opt reconnection trial.
    ListKOptReconnection,
    /// A regret-insertion fallback that appends to an owner.
    ListRegretOwnerAppend,
}

impl CandidateTraceSource {
    const fn code(self) -> u8 {
        match self {
            Self::Construction => 1,
            Self::LocalSearch => 2,
            Self::VariableNeighborhoodDescent => 3,
            Self::KOpt => 4,
            Self::ListRoundRobinConstruction => 5,
            Self::ListCheapestInsertionTrial => 6,
            Self::ListRegretInsertionTrial => 7,
            Self::ListClarkeWrightSavings => 8,
            Self::ListClarkeWrightMerge => 9,
            Self::ListClarkeWrightCompletionInsertion => 10,
            Self::ListKOptReconnection => 11,
            Self::ListRegretOwnerAppend => 12,
        }
    }
}

/// One state transition for a retained candidate-pull entry.
///
/// The events are recorded in engine order. This lets a bounded trace prove
/// not only which candidates were pulled, but which one was evaluated,
/// rejected, selected, and actually applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateTraceDisposition {
    /// The engine stopped after the pull and before it evaluated this
    /// candidate.
    InterruptedBeforeEvaluation,
    /// The candidate reached evaluation.
    Evaluated,
    /// Evaluation determined that the candidate was not doable.
    NotDoable,
    /// A hard-improvement requirement rejected the evaluated candidate.
    RejectedByHardImprovement,
    /// A score-improvement requirement rejected the evaluated candidate.
    RejectedByScoreImprovement,
    /// The configured acceptor rejected the evaluated candidate.
    AcceptorRejected,
    /// The candidate was accepted/evaluated but lost to a forager choice.
    ForagerIgnored,
    /// The candidate was selected for commit.
    Selected,
    /// The selected candidate was applied to the working solution.
    Applied,
    /// Parallel evaluation pulled this candidate ahead of the sequential
    /// decision point; the step ended before the candidate was consumed.
    DiscardedSpeculative,
}

impl CandidateTraceDisposition {
    const fn code(self) -> u8 {
        match self {
            Self::InterruptedBeforeEvaluation => 1,
            Self::Evaluated => 2,
            Self::NotDoable => 3,
            Self::RejectedByHardImprovement => 4,
            Self::RejectedByScoreImprovement => 5,
            Self::AcceptorRejected => 6,
            Self::ForagerIgnored => 7,
            Self::Selected => 8,
            Self::Applied => 9,
            Self::DiscardedSpeculative => 10,
        }
    }

    const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::InterruptedBeforeEvaluation
                | Self::NotDoable
                | Self::RejectedByHardImprovement
                | Self::RejectedByScoreImprovement
                | Self::AcceptorRejected
                | Self::ForagerIgnored
                | Self::Applied
                | Self::DiscardedSpeculative
        )
    }
}

/// One ordered candidate pull retained by an enabled trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePullTelemetry {
    /// Monotonic zero-based ordinal across all instrumented engine pulls.
    pub ordinal: u64,
    /// Engine path that consumed the candidate.
    pub source: CandidateTraceSource,
    /// Index of the phase that pulled the candidate.
    pub phase_index: usize,
    /// Type name of that phase.
    pub phase_type: String,
    /// Step index within the phase.
    pub step_index: u64,
    /// Selector that yielded the candidate, when the engine pulls from
    /// indexed selectors.
    pub selector_index: Option<usize>,
    /// Source-local cursor/trial coordinate reported by the engine.
    ///
    /// This is useful when inspecting one implementation, but it is not a
    /// cross-representation comparison key: specialized construction loops
    /// can reuse local coordinates across owners or passes. Consumers compare
    /// the globally monotonic [`Self::ordinal`] together with `identity`.
    pub candidate_index: usize,
    /// Construction slot the candidate was generated for; `None` for
    /// local-search candidates.
    pub construction_target: Option<CandidateTraceConstructionTarget>,
    /// `None` explicitly means this move family did not provide a canonical
    /// logical-coordinate identity. Such a trace is not qualifying evidence.
    pub identity: Option<CandidateTraceIdentity>,
    /// Ordered engine decisions for this captured pull. A terminal detail
    /// trace must finish every retained pull with a terminal disposition.
    pub dispositions: Vec<CandidateTraceDisposition>,
}

impl CandidatePullTelemetry {
    pub(super) fn append_canonical_bytes(&self, out: &mut Vec<u8>) {
        out.push(0x45);
        append_u64(out, self.ordinal);
        out.push(self.source.code());
        append_usize(out, self.phase_index);
        append_string(out, &self.phase_type);
        append_u64(out, self.step_index);
        append_option_usize(out, self.selector_index);
        append_usize(out, self.candidate_index);
        match self.construction_target {
            Some(target) => {
                append_bool(out, true);
                append_usize(out, target.descriptor_index);
                append_usize(out, target.entity_index);
            }
            None => append_bool(out, false),
        }
        match &self.identity {
            Some(identity) => {
                append_bool(out, true);
                identity.append_canonical_bytes(out);
            }
            None => append_bool(out, false),
        }
        append_len(out, self.dispositions.len());
        for disposition in &self.dispositions {
            out.push(disposition.code());
        }
    }

    pub(super) fn has_terminal_disposition(&self) -> bool {
        self.dispositions
            .last()
            .is_some_and(|disposition| disposition.is_terminal())
    }
}
