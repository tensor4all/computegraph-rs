use std::hash::Hash;

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
