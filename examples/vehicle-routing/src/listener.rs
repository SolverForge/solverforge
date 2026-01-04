//! Event listener that sends VRP solver events to SERIO Console.

use parking_lot::RwLock;
use solverforge::prelude::*;
use solverforge_console::{ConsoleInstance, ConsoleManager};
use std::time::Instant;

use crate::domain::VehicleRoutePlan;

/// Event listener for Vehicle Routing solver.
///
/// Sends detailed solver events to SERIO Console channels for TUI display.
/// Provides full explainability: problem config, phase metrics, step progress.
#[derive(Debug)]
pub struct VrpConsoleListener {
    console: RwLock<ConsoleInstance>,
    solve_start: Instant,
    phase_start: RwLock<Option<Instant>>,
    phase_metrics: RwLock<PhaseMetrics>,
    problem_info: RwLock<Option<ProblemInfo>>,
}

#[derive(Debug, Default, Clone)]
struct PhaseMetrics {
    steps_accepted: u64,
    moves_evaluated: u64,
    last_score: String,
}

#[derive(Debug, Clone)]
struct ProblemInfo {
    vehicles: usize,
    visits: usize,
    locations: usize,
}

impl VrpConsoleListener {
    /// Creates a new VRP console listener.
    ///
    /// Creates a console instance from the global ConsoleManager using the provided job ID.
    pub fn new(job_id: &str) -> Self {
        let console = ConsoleManager::global().create_console(job_id.to_string());
        Self {
            console: RwLock::new(console),
            solve_start: Instant::now(),
            phase_start: RwLock::new(None),
            phase_metrics: RwLock::new(PhaseMetrics::default()),
            problem_info: RwLock::new(None),
        }
    }

    /// Records move evaluation (called from solver loop).
    pub fn record_move(&self) {
        self.phase_metrics.write().moves_evaluated += 1;
    }

    /// Records move acceptance (called from solver loop).
    pub fn record_accepted(&self, score: &str) {
        let mut metrics = self.phase_metrics.write();
        metrics.steps_accepted += 1;
        metrics.last_score = score.to_string();
    }

    /// Reports step progress (call every N steps for periodic updates).
    pub fn report_step_progress(&self, step: u64) {
        let metrics = self.phase_metrics.read();
        let phase_elapsed = self.phase_start.read()
            .map(|start| start.elapsed())
            .unwrap_or_default();

        let moves_per_sec = if phase_elapsed.as_secs_f64() > 0.0 {
            (metrics.moves_evaluated as f64 / phase_elapsed.as_secs_f64()) as u64
        } else {
            0
        };

        let mut console = self.console.write();
        let solver_channel = console.channel("solver");

        solver_channel.info(&format!(
            "Step {} | {:?} | {}/sec | {}",
            format_number(step),
            format_duration(phase_elapsed),
            format_number(moves_per_sec),
            metrics.last_score
        ));

        solver_channel.metric("moves_per_sec", &moves_per_sec.to_string());
        solver_channel.metric("steps_accepted", &metrics.steps_accepted.to_string());
    }
}

impl SolverEventListener<VehicleRoutePlan> for VrpConsoleListener {
    fn on_best_solution_changed(&self, _solution: &VehicleRoutePlan, score: &HardSoftScore) {
        let mut console = self.console.write();
        let core = console.core_channel();

        core.info(&format!("New best solution: {}", score));
        core.metric("best_score", &score.to_string());
    }

    fn on_solving_started(&self, solution: &VehicleRoutePlan) {
        // Store problem info
        *self.problem_info.write() = Some(ProblemInfo {
            vehicles: solution.vehicles.len(),
            visits: solution.visits.len(),
            locations: solution.locations.len(),
        });

        let mut console = self.console.write();
        let core = console.core_channel();

        core.info(&format!(
            "Problem: vehicles ({}), visits ({}), locations ({})",
            format_number(solution.vehicles.len() as u64),
            format_number(solution.visits.len() as u64),
            format_number(solution.locations.len() as u64)
        ));

        let scale = calculate_problem_scale(solution.visits.len(), solution.vehicles.len());
        core.info(&format!(
            "Entity count ({}), variable count ({}), value count ({}), problem scale ({})",
            format_number(solution.visits.len() as u64),
            format_number(solution.visits.len() as u64),
            format_number(solution.vehicles.len() as u64),
            scale
        ));

        core.info("Solving started");
    }

    fn on_solving_ended(&self, _solution: &VehicleRoutePlan, is_terminated_early: bool) {
        let total_duration = self.solve_start.elapsed();
        let metrics = self.phase_metrics.read();
        let problem_info = self.problem_info.read();

        let moves_per_sec = if total_duration.as_secs_f64() > 0.0 {
            (metrics.moves_evaluated as f64 / total_duration.as_secs_f64()) as u64
        } else {
            0
        };

        let mut console = self.console.write();
        let core = console.core_channel();

        if is_terminated_early {
            core.warn("Solving terminated early");
        } else {
            core.info("Solving completed");
        }

        core.info(&format!(
            "Total time: {}, best score: {}, move speed: {}/sec",
            format_duration(total_duration),
            metrics.last_score,
            format_number(moves_per_sec)
        ));

        if let Some(info) = problem_info.as_ref() {
            core.metric("total_vehicles", &info.vehicles.to_string());
            core.metric("total_visits", &info.visits.to_string());
        }
        core.metric("total_duration_ms", &total_duration.as_millis().to_string());
        core.metric("total_moves", &metrics.moves_evaluated.to_string());
        core.metric("total_steps", &metrics.steps_accepted.to_string());
        core.metric("final_score", &metrics.last_score);
    }
}

impl PhaseLifecycleListener<VehicleRoutePlan> for VrpConsoleListener {
    fn on_phase_started(&self, phase_index: usize, phase_type: &str) {
        *self.phase_start.write() = Some(Instant::now());
        *self.phase_metrics.write() = PhaseMetrics::default();

        let mut console = self.console.write();
        let solver_channel = console.channel("solver");

        solver_channel.info(&format!(
            "Phase {} ({}) started",
            phase_index,
            phase_type
        ));
    }

    fn on_phase_ended(&self, phase_index: usize, phase_type: &str) {
        let phase_duration = self.phase_start.read()
            .map(|start| start.elapsed())
            .unwrap_or_default();

        let metrics = self.phase_metrics.read();

        let moves_per_sec = if phase_duration.as_secs_f64() > 0.0 {
            (metrics.moves_evaluated as f64 / phase_duration.as_secs_f64()) as u64
        } else {
            0
        };

        let acceptance_rate = if metrics.moves_evaluated > 0 {
            (metrics.steps_accepted as f64 / metrics.moves_evaluated as f64) * 100.0
        } else {
            0.0
        };

        let mut console = self.console.write();
        let solver_channel = console.channel("solver");

        solver_channel.info(&format!(
            "Phase {} ({}) ended: time ({}), best score ({}), speed ({}/sec), steps ({}, {:.1}% accepted)",
            phase_index,
            phase_type,
            format_duration(phase_duration),
            metrics.last_score,
            format_number(moves_per_sec),
            format_number(metrics.steps_accepted),
            acceptance_rate
        ));

        solver_channel.metric(&format!("phase_{}_duration_ms", phase_index), &phase_duration.as_millis().to_string());
        solver_channel.metric(&format!("phase_{}_moves_per_sec", phase_index), &moves_per_sec.to_string());
        solver_channel.metric(&format!("phase_{}_acceptance_rate", phase_index), &format!("{:.1}", acceptance_rate));
    }
}

impl StepLifecycleListener<VehicleRoutePlan> for VrpConsoleListener {
    fn on_step_started(&self, _step_index: u64) {
        // Step-level events are too granular for console output
        // Metrics are tracked via record_move/record_accepted instead
    }

    fn on_step_ended(&self, _step_index: u64, _score: &HardSoftScore) {
        // Step-level events are too granular for console output
        // Use report_step_progress for periodic updates
    }
}

/// Formats a number with commas.
fn format_number(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }

    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, ch) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }

    result.chars().rev().collect()
}

/// Formats a duration for display.
fn format_duration(d: std::time::Duration) -> String {
    let total_ms = d.as_millis();
    if total_ms < 1000 {
        format!("{}ms", total_ms)
    } else if total_ms < 60_000 {
        format!("{:.2}s", d.as_secs_f64())
    } else {
        let mins = total_ms / 60_000;
        let secs = (total_ms % 60_000) / 1000;
        format!("{}m {}s", mins, secs)
    }
}

/// Calculates approximate problem scale.
fn calculate_problem_scale(entity_count: usize, value_count: usize) -> String {
    if entity_count == 0 || value_count == 0 {
        return "0".to_string();
    }

    let log_scale = (entity_count as f64) * (value_count as f64).log10();
    let exponent = log_scale.floor() as i32;
    let mantissa = 10f64.powf(log_scale - exponent as f64);

    format!("{:.3} × 10^{}", mantissa, exponent)
}
