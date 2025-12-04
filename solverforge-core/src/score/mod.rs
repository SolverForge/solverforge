mod bendable;
mod hard_soft;
mod hard_soft_decimal;
mod simple;
mod simple_decimal;

pub use bendable::{BendableDecimalScore, BendableScore};
pub use hard_soft::{HardMediumSoftScore, HardSoftScore};
pub use hard_soft_decimal::{HardMediumSoftDecimalScore, HardSoftDecimalScore};
pub use simple::SimpleScore;
pub use simple_decimal::SimpleDecimalScore;

use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub trait Score: Clone + PartialOrd + Display + Serialize + for<'de> Deserialize<'de> {
    fn is_feasible(&self) -> bool;
    fn is_solution_initialized(&self) -> bool;
    fn zero() -> Self
    where
        Self: Sized;
    fn negate(&self) -> Self;
    fn add(&self, other: &Self) -> Self;
    fn subtract(&self, other: &Self) -> Self;
}
