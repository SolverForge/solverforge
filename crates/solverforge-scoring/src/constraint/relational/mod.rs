/* Generic relational scoring terminal over the common operator tree.

One terminal scores any operator tree and retains a signed score per
terminal row; the operator decides WHICH rows exist, the terminal decides
WHAT each row scores and publishes signed deltas through
`IncrementalConstraint`. Arity lives entirely in the operator tree, so one
terminal serves every depth.
*/

mod grouped_weights;
mod operator_scorer;
mod operator_terminal;
mod shared_terminal;

pub use grouped_weights::{
    DirectComplementAdapter, DirectComplementWeight, GroupAdapter, GroupWeight, GroupedJoinAdapter,
    GroupedJoinWeight, OptionalComplementAdapter, OptionalComplementWeight,
};
pub use operator_scorer::{AdaptWeight, OperatorScorer, RowWeight, ScorerChain};
pub use operator_terminal::OperatorTerminal;
pub use shared_terminal::{SharedOperatorBuilder, SharedOperatorTerminal};
