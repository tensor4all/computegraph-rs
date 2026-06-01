//! AD-agnostic tensor computation graph engine.

pub mod compile;
mod eval;
pub mod graph;
pub mod interner;
pub mod materialize;
pub mod resolve;
pub mod traits;
pub mod types;

pub use traits::{EvaluableGraphOperation, GraphOperation};
pub use types::{LocalOperationId, LocalValueId, OperationKey, OperationRole, ValueKey, ValueRef};
