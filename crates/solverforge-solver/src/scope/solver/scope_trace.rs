impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    /// Enables the one bounded, core-owned candidate-pull recorder for this
    /// solve. Normal progress snapshots intentionally omit this recorder.
    pub(crate) fn enable_candidate_trace(
        &mut self,
        header: CandidateTraceHeader,
        max_entries: usize,
    ) {
        self.stats
            .enable_candidate_trace(CandidateTraceTelemetry::new(header, max_entries));
    }

    pub(crate) fn begin_candidate_trace_phase(&mut self) -> Option<usize> {
        self.stats.begin_candidate_trace_phase()
    }

    /// Publishes the executor-supplied terminal phase plan into the enabled
    /// candidate trace. This is intentionally a direct forwarding boundary:
    /// the scope never infers a plan from progress, snapshots, or callbacks.
    pub(crate) fn finalize_candidate_trace_resolved_phase_plan(
        &mut self,
        resolved_phase_plan: crate::stats::CandidateTracePhasePlan,
    ) {
        self.stats
            .finalize_candidate_trace_resolved_phase_plan(resolved_phase_plan);
    }

    /// Records one engine-consumed cursor candidate when tracing is enabled.
    ///
    /// The recorder decides capacity before asking the move for its owned
    /// canonical identity. Once the bounded prefix is full, it still counts
    /// pulls but allocates neither identities nor trace entries.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_candidate_pull<M>(
        &mut self,
        source: CandidateTraceSource,
        phase_index: usize,
        phase_type: &'static str,
        step_index: u64,
        selector_index: Option<usize>,
        candidate_index: usize,
        construction_target: Option<CandidateTraceConstructionTarget>,
        candidate: &M,
    ) -> Option<CandidateTracePullToken>
    where
        M: Move<S>,
    {
        let CandidateTraceRecordDecision::Capture { ordinal } =
            self.stats.prepare_candidate_trace_pull()
        else {
            return None;
        };
        let identity = candidate.candidate_trace_identity();
        Some(self.stats.record_prepared_candidate_trace_pull(CandidatePullTelemetry {
                ordinal,
                source,
                phase_index,
                phase_type: phase_type.to_string(),
                step_index,
                selector_index,
                candidate_index,
                construction_target,
                identity,
                dispositions: Vec::new(),
            }))
    }

    /// Records one explicit specialized-engine trial that is not represented
    /// by a `MoveCursor`.  The operation identity is still created only when
    /// the bounded recorder has room for it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_candidate_operation<I, T>(
        &mut self,
        source: CandidateTraceSource,
        phase_index: usize,
        phase_type: &'static str,
        step_index: u64,
        selector_index: Option<usize>,
        candidate_index: usize,
        construction_target: Option<CandidateTraceConstructionTarget>,
        descriptor_index: usize,
        operation: &'static str,
        components: I,
    ) -> Option<CandidateTracePullToken>
    where
        I: IntoIterator<Item = T>,
        T: Into<crate::stats::CandidateTraceCoordinate>,
    {
        let CandidateTraceRecordDecision::Capture { ordinal } =
            self.stats.prepare_candidate_trace_pull()
        else {
            return None;
        };
        Some(self.stats.record_prepared_candidate_trace_pull(CandidatePullTelemetry {
                ordinal,
                source,
                phase_index,
                phase_type: phase_type.to_string(),
                step_index,
                selector_index,
                candidate_index,
                construction_target,
                identity: Some(crate::stats::CandidateTraceIdentity::operation(
                    descriptor_index,
                    operation,
                    components,
                )),
                dispositions: Vec::new(),
            }))
    }

    pub(crate) fn record_candidate_trace_disposition(
        &mut self,
        token: CandidateTracePullToken,
        disposition: CandidateTraceDisposition,
    ) {
        self.stats
            .record_candidate_trace_disposition(token, disposition);
    }

    pub(crate) fn record_evaluated_move(&mut self, duration: Duration) {
        self.stats.record_evaluated_move(duration);
        if let Some(phase_budget) = self.phase_budget {
            phase_budget.record_evaluated_move();
        }
    }

    pub(crate) fn record_selector_evaluated(&mut self, selector_index: usize, duration: Duration) {
        self.stats.record_selector_evaluated(selector_index, duration);
        if let Some(phase_budget) = self.phase_budget {
            phase_budget.record_evaluated_move();
        }
    }

    pub(crate) fn record_move_speculative(&mut self) {
        self.stats.record_move_speculative();
    }

    pub(crate) fn record_score_calculation(&mut self) {
        self.stats.record_score_calculation();
        if let Some(phase_budget) = self.phase_budget {
            phase_budget.record_score_calculation();
        }
    }
}
