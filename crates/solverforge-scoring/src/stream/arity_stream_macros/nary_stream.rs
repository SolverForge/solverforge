/* Shared N-ary constraint stream macros for bi/tri/quad/penta arities.

All four stream arities share identical structure, but each arity now lives
in its own file so the module root stays limited to wiring.
*/

#[macro_use]
mod bi;
#[macro_use]
mod shared;
#[macro_use]
mod penta;
#[macro_use]
mod quad;
#[macro_use]
mod tri;
