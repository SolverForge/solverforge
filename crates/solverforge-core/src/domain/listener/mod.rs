//! Variable listener infrastructure for shadow variable updates.
//!
//! Variable listeners are notified when genuine planning variables change,
//! allowing them to update shadow variables accordingly.
//!
//! # Architecture
//!
//! - [`VariableListener`]: Listens to basic/chained variable changes
//! - [`ListVariableListener`]: Listens to list variable changes with range info
//!
//! # Listener Types
//!
//! - **Automatic listeners**: Built-in listeners for Index, Inverse, Next/Previous
//! - **Custom listeners**: User-defined listeners for complex shadow variables

mod traits;

#[cfg(test)]
mod tests;

pub use traits::{
    ListVariableListener, ListVariableNotification, VariableListener, VariableNotification,
};

#[cfg(test)]
mod tests;
