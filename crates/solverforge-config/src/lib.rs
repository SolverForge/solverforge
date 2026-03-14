/* Configuration system for SolverForge.

Load solver configuration from TOML files to control termination,
phases, and acceptors without code changes.

# Examples

Load configuration from TOML string:

```
use solverforge_config::SolverConfig;
use std::time::Duration;

let config = SolverConfig::from_toml_str(r#"
[termination]
seconds_spent_limit = 30
unimproved_seconds_spent_limit = 5

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases]]
type = "local_search"
[phases.acceptor]
type = "late_acceptance"
late_acceptance_size = 400
"#).unwrap();

assert_eq!(config.time_limit(), Some(Duration::from_secs(30)));
assert_eq!(config.phases.len(), 2);
```

Use default config when file is missing:

```
use solverforge_config::SolverConfig;

let config = SolverConfig::load("solver.toml").unwrap_or_default();
// Proceeds with defaults if file doesn't exist
```
*/

mod acceptor;
mod director;
mod error;
mod forager;
mod move_selector;
mod phase;
mod solver_config;
mod termination;

pub use acceptor::{
    AcceptorConfig, GreatDelugeConfig, LateAcceptanceConfig, SimulatedAnnealingConfig,
    TabuSearchConfig,
};
pub use director::DirectorConfig;
pub use error::ConfigError;
pub use forager::{ForagerConfig, PickEarlyType};
pub use move_selector::{
    CartesianProductConfig, ChangeMoveConfig, KOptMoveSelectorConfig, ListChangeMoveConfig,
    ListReverseMoveConfig, ListRuinMoveSelectorConfig, ListSwapMoveConfig, MoveSelectorConfig,
    NearbyListChangeMoveConfig, NearbyListSwapMoveConfig, SubListChangeMoveConfig,
    SubListSwapMoveConfig, SwapMoveConfig, UnionMoveSelectorConfig,
};
pub use phase::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, CustomPhaseConfig,
    ExhaustiveSearchConfig, ExhaustiveSearchType, LocalSearchConfig, PartitionedSearchConfig,
    PhaseConfig,
};
pub use solver_config::{EnvironmentMode, MoveThreadCount, SolverConfig, SolverConfigOverride};
pub use termination::TerminationConfig;

#[cfg(test)]
mod tests;
