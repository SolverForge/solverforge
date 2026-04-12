/* Colorful console output for solver metrics.

Provides a custom `tracing` layer that formats solver events with colors.

## Log Levels

- **INFO**: Lifecycle events (solving/phase start/end)
- **DEBUG**: Progress updates (1/sec with speed and score)
- **TRACE**: Individual step evaluations
*/

mod banner;
mod format;
mod init;
mod layer;
mod time;
mod visitor;

pub use init::init;
pub use layer::SolverConsoleLayer;
