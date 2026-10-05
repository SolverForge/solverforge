/* Generic relational scoring terminal over composable join operators.

Owns one operator's state plus the retained signed score per terminal
row. The operator decides WHICH pairs join; the terminal decides WHAT
each pair scores and publishes signed deltas through
`IncrementalConstraint`. One operator family serves every arity: binary
joins land first, chained/deeper rows follow without new terminal logic.
*/

mod debug;
mod incremental;
mod state;
mod triple;
mod weight;

pub use state::Terminal;
pub use triple::TripleTerminal;
pub use weight::{RelationalWeight, RelationalWeight3};
