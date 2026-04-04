//! AD-agnostic tensor computation graph engine.

pub mod compile;
mod eval;
pub mod fragment;
pub mod interner;
pub mod materialize;
pub mod resolve;
pub mod traits;
pub mod types;

pub use traits::{EvalGraphOp, GraphOp};
pub use types::{GlobalOpKey, GlobalValKey, LocalOpId, LocalValId, OpMode, ValRef};
