/* Zero-erasure cross-tri-constraint stream for three-source join patterns.

A `Tri` extends a cross-joined (A, B) pair with a third source C, such as
(Shift, Employee, DayOff) joins reached through
`.join((day_offs, |day_off| ...))` on a cross Bi stream. All type
information is preserved at compile time - no Arc, no dyn, fully
monomorphized.
*/

mod base;

pub use base::Tri;
