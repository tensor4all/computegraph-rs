use std::hash::Hash;

use crate::fragment::FragmentBuilder;
use crate::types::{LocalValId, OpMode, ValRef};

/// Operation node trait. `computegraph` is fully generic over this abstraction.
///
/// `GraphOp` captures the metadata of an operation (input/output counts,
/// associated types) but does **not** include evaluation. See [`EvalGraphOp`]
/// for the evaluation extension.
///
/// # Examples
///
/// ```ignore
/// use computegraph::GraphOp;
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// enum AddOp {
///     Add,
/// }
///
/// impl GraphOp for AddOp {
///     type Operand = f64;
///     type Context = ();
///     type InputKey = &'static str;
///
///     fn n_inputs(&self) -> usize { 2 }
///     fn n_outputs(&self) -> usize { 1 }
/// }
/// ```
pub trait GraphOp: Clone + std::fmt::Debug + Hash + Eq + Send + Sync + 'static {
    type Operand: Clone + Send + Sync + 'static;
    type Context;
    type InputKey: Clone + std::fmt::Debug + Hash + Eq + Send + Sync + 'static;

    /// Returns the number of inputs consumed by this operation.
    fn n_inputs(&self) -> usize;

    /// Returns the number of outputs produced by this operation.
    fn n_outputs(&self) -> usize;
}

/// Minimal trait for emitting operations into a computation context.
///
/// AD transpose rules use only this interface, enabling both graph-building
/// (`FragmentBuilder`) and eager execution through the same code.
///
/// # Examples
///
/// ```ignore
/// use computegraph::{FragmentBuilder, GraphOp, OpEmitter, OpMode, ValRef};
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// enum UnaryOp {
///     Identity,
/// }
///
/// impl GraphOp for UnaryOp {
///     type Operand = f64;
///     type Context = ();
///     type InputKey = &'static str;
///
///     fn n_inputs(&self) -> usize { 1 }
///     fn n_outputs(&self) -> usize { 1 }
/// }
///
/// let mut builder = FragmentBuilder::<UnaryOp>::new();
/// let x = builder.add_input("x");
/// let ys = builder.add_op(UnaryOp::Identity, vec![ValRef::Local(x)], OpMode::Primal);
/// assert_eq!(ys.len(), 1);
/// ```
pub trait OpEmitter<Op: GraphOp> {
    /// Emits an operation with the given inputs and mode, returning output ids.
    fn add_op(&mut self, op: Op, inputs: Vec<ValRef<Op>>, mode: OpMode) -> Vec<LocalValId>;
}

impl<Op: GraphOp> OpEmitter<Op> for FragmentBuilder<Op> {
    fn add_op(&mut self, op: Op, inputs: Vec<ValRef<Op>>, mode: OpMode) -> Vec<LocalValId> {
        FragmentBuilder::add_op(self, op, inputs, mode)
    }
}

/// Extension trait that adds evaluation capability to a [`GraphOp`].
///
/// # Examples
///
/// ```ignore
/// use computegraph::{GraphOp, EvalGraphOp};
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// enum AddOp {
///     Add,
/// }
///
/// impl GraphOp for AddOp {
///     type Operand = f64;
///     type Context = ();
///     type InputKey = &'static str;
///
///     fn n_inputs(&self) -> usize { 2 }
///     fn n_outputs(&self) -> usize { 1 }
/// }
///
/// impl EvalGraphOp for AddOp {
///     fn eval(&self, _ctx: &mut Self::Context, inputs: &[&Self::Operand]) -> Vec<Self::Operand> {
///         vec![inputs[0] + inputs[1]]
///     }
/// }
/// ```
pub trait EvalGraphOp: GraphOp {
    /// Evaluates the operation given concrete input operands.
    fn eval(&self, ctx: &mut Self::Context, inputs: &[&Self::Operand]) -> Vec<Self::Operand>;
}
