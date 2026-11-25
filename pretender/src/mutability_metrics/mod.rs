//! Mutability analysis: void-mutator detection and mutable-state ratio.
//!
//! Shared between the void-mutator and mutable-state check proposals.
//! Mutable-binding counting lives in `bindings`; void-mutator detection
//! in `void_mutators`.

mod bindings;
mod void_mutators;

pub use bindings::{count_mutable_bindings, MutableBindingAnalysis};
pub use void_mutators::{detect_void_mutators, VoidMutator};
