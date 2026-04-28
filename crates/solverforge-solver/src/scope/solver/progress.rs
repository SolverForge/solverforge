#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverProgressKind {
    Progress,
    BestSolution,
}

#[derive(Debug, Clone)]
pub struct SolverProgressRef<'a, S: PlanningSolution> {
    pub kind: SolverProgressKind,
    pub status: SolverLifecycleState,
    pub solution: Option<&'a S>,
    pub current_score: Option<&'a S::Score>,
    pub best_score: Option<&'a S::Score>,
    pub telemetry: crate::stats::SolverTelemetry,
}

pub trait ProgressCallback<S: PlanningSolution>: Send + Sync {
    fn invoke(&self, progress: SolverProgressRef<'_, S>);
}

impl<S: PlanningSolution> ProgressCallback<S> for () {
    fn invoke(&self, _progress: SolverProgressRef<'_, S>) {}
}

impl<S, F> ProgressCallback<S> for F
where
    S: PlanningSolution,
    F: for<'a> Fn(SolverProgressRef<'a, S>) + Send + Sync,
{
    fn invoke(&self, progress: SolverProgressRef<'_, S>) {
        self(progress);
    }
}
