/* Supply infrastructure for variable relationship tracking.

Supplies provide efficient access to derived information about planning variables,
such as inverse relationships and list-element positions.

# Zero-Erasure Design

- **Index-based**: All supplies store indices, not cloned domain objects
- **Owned**: Supplies are owned directly, no `Arc`, `Box`, or `dyn`
- **Mutation via `&mut`**: Ordinary Rust ownership, no `RwLock` or interior mutability
- **Generic**: Full type information preserved through the entire pipeline

# Available Supplies

- [`InverseSupply`]: Maps values to entity indices for O(1) inverse lookups
- [`ListStateSupply`]: Tracks element positions in list variables

# Usage

Supplies are owned by the score director or solver scope. They're created
when needed and accessed via regular Rust references:

```
use solverforge_core::domain::supply::{InverseSupply, ListStateSupply};

// Create supplies owned by your containing struct
let mut inverse: InverseSupply<i64> = InverseSupply::new();
let mut list_state: ListStateSupply<usize> = ListStateSupply::new();

// Use with standard Rust ownership
inverse.insert(42, 0);  // value 42 -> entity index 0
list_state.assign(10, 0, 0);  // element 10 -> entity 0, position 0

// Read with shared reference
assert_eq!(inverse.get(&42), Some(0));
assert_eq!(list_state.get_entity(&10), Some(0));
```
*/

mod inverse;
mod list_state;

#[cfg(test)]
mod tests;

pub use inverse::InverseSupply;
pub use list_state::{ElementPosition, ListStateSupply};
