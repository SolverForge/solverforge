use std::fmt;
use std::sync::Arc;

use crate::stats::CandidateTraceIdentity;

/// Host-owned numeric metadata for one already-generated move.
///
/// The compiled selector invokes this only for `sorted` or `probabilistic`
/// order. The callback sees the same canonical logical identity used by the
/// candidate trace, so native and host-language models rank identical move
/// coordinates without rebuilding or executing a second selector.
pub trait RuntimeCandidateMetric<S>: Send + Sync {
    fn measure(&self, solution: &S, candidate: &CandidateTraceIdentity) -> f64;
}

pub struct RuntimeCandidateMetricBinding<S> {
    name: Arc<str>,
    metric: Arc<dyn RuntimeCandidateMetric<S>>,
}

impl<S> RuntimeCandidateMetricBinding<S> {
    pub fn new(
        name: impl Into<Arc<str>>,
        metric: Arc<dyn RuntimeCandidateMetric<S>>,
    ) -> Result<Self, String> {
        let name = name.into();
        if name.is_empty() {
            return Err("candidate metric name must not be empty".to_string());
        }
        Ok(Self { name, metric })
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn measure(&self, solution: &S, candidate: &CandidateTraceIdentity) -> f64 {
        self.metric.measure(solution, candidate)
    }
}

impl<S> Clone for RuntimeCandidateMetricBinding<S> {
    fn clone(&self) -> Self {
        Self {
            name: Arc::clone(&self.name),
            metric: Arc::clone(&self.metric),
        }
    }
}

impl<S> fmt::Debug for RuntimeCandidateMetricBinding<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeCandidateMetricBinding")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

pub struct RuntimeCandidateMetricRegistry<S> {
    metrics: Vec<RuntimeCandidateMetricBinding<S>>,
}

impl<S> Clone for RuntimeCandidateMetricRegistry<S> {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics.clone(),
        }
    }
}

impl<S> fmt::Debug for RuntimeCandidateMetricRegistry<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_list()
            .entries(self.metrics.iter().map(RuntimeCandidateMetricBinding::name))
            .finish()
    }
}

impl<S> RuntimeCandidateMetricRegistry<S> {
    pub fn new(metrics: Vec<RuntimeCandidateMetricBinding<S>>) -> Result<Self, String> {
        for (index, metric) in metrics.iter().enumerate() {
            if metrics[..index]
                .iter()
                .any(|existing| existing.name() == metric.name())
            {
                return Err(format!(
                    "candidate metric `{}` is declared more than once",
                    metric.name()
                ));
            }
        }
        Ok(Self { metrics })
    }

    pub fn get(&self, name: &str) -> Option<&RuntimeCandidateMetricBinding<S>> {
        self.metrics.iter().find(|metric| metric.name() == name)
    }
}

impl<S> Default for RuntimeCandidateMetricRegistry<S> {
    fn default() -> Self {
        Self {
            metrics: Vec::new(),
        }
    }
}
