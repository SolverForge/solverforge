/* List construction phase for assigning list elements to entities.

Provides several construction strategies for list variables
(e.g., assigning visits to vehicles in VRP):

- [`ListConstructionPhase`]: Simple round-robin assignment
- [`ListCheapestInsertionPhase`]: Score-guided greedy insertion
- [`ListRegretInsertionPhase`]: Regret-based insertion (reduces greedy myopia)
*/

mod cheapest;
mod regret;
mod round_robin;
mod state;

pub use cheapest::ListCheapestInsertionPhase;
pub use regret::ListRegretInsertionPhase;
pub use round_robin::{ListConstructionPhase, ListConstructionPhaseBuilder};
