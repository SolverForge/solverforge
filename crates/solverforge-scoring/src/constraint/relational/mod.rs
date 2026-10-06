/* Generic relational scoring terminal over the common operator tree.

One terminal scores any operator tree and retains a signed score per
terminal row; the operator decides WHICH rows exist, the terminal decides
WHAT each row scores and publishes signed deltas through
`IncrementalConstraint`. Arity lives entirely in the operator tree, so one
terminal serves every depth.
*/

mod operator_terminal;

pub use operator_terminal::OperatorTerminal;
