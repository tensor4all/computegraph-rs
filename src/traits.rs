use std::hash::Hash;

/// Operation node trait. `computegraph` is fully generic over this abstraction.
///
/// `GraphOperation` captures the metadata of an operation (input/output counts,
/// associated types) but does **not** include evaluation. See [`EvaluableGraphOperation`]
/// for the evaluation extension.
///
/// # Examples
///
/// ```
/// use computegraph::GraphOperation;
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// enum AddOp {
///     Add,
/// }
///
/// impl GraphOperation for AddOp {
///     type Operand = f64;
///     type Context = ();
///     type InputKey = &'static str;
///
///     fn input_count(&self) -> usize { 2 }
///     fn output_count(&self) -> usize { 1 }
/// }
///
/// assert_eq!(AddOp::Add.input_count(), 2);
/// ```
pub trait GraphOperation: Clone + std::fmt::Debug + Hash + Eq + Send + Sync + 'static {
    type Operand: Clone + Send + Sync + 'static;
    type Context;
    type InputKey: Clone + std::fmt::Debug + Hash + Eq + Send + Sync + 'static;

    /// Returns the number of inputs consumed by this operation.
    fn input_count(&self) -> usize;

    /// Returns the number of outputs produced by this operation.
    fn output_count(&self) -> usize;
}

/// Extension trait that adds evaluation capability to a [`GraphOperation`].
///
/// # Examples
///
/// ```
/// use computegraph::{EvaluableGraphOperation, GraphOperation};
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// enum AddOp {
///     Add,
/// }
///
/// impl GraphOperation for AddOp {
///     type Operand = f64;
///     type Context = ();
///     type InputKey = &'static str;
///
///     fn input_count(&self) -> usize { 2 }
///     fn output_count(&self) -> usize { 1 }
/// }
///
/// impl EvaluableGraphOperation for AddOp {
///     fn eval(&self, _ctx: &mut Self::Context, inputs: &[&Self::Operand]) -> Vec<Self::Operand> {
///         vec![inputs[0] + inputs[1]]
///     }
/// }
///
/// let result = AddOp::Add.eval(&mut (), &[&3.0, &4.0]);
/// assert_eq!(result, vec![7.0]);
/// ```
pub trait EvaluableGraphOperation: GraphOperation {
    /// Evaluates the operation given concrete input operands.
    fn eval(&self, ctx: &mut Self::Context, inputs: &[&Self::Operand]) -> Vec<Self::Operand>;
}
