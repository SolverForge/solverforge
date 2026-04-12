pub(super) fn zero_telemetry() -> crate::SolverTelemetry {
    crate::SolverTelemetry::default()
}

pub(super) fn telemetry_with_steps(step_count: u64) -> crate::SolverTelemetry {
    crate::SolverTelemetry {
        step_count,
        ..crate::SolverTelemetry::default()
    }
}
