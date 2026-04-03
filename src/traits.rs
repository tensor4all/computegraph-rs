use std::hash::Hash;

/// Runtime value type. Scalars are rank-0 tensors.
///
/// # Examples
///
/// ```ignore
/// use computegraph::Operand;
///
/// #[derive(Clone)]
/// struct Scalar(f64);
///
/// impl Operand for Scalar {
///     fn zero(_shape: &[usize]) -> Self { Self(0.0) }
///     fn one(_shape: &[usize]) -> Self { Self(1.0) }
///     fn reshape(&self, _shape: &[usize]) -> Self { self.clone() }
///     fn broadcast_in_dim(&self, _shape: &[usize], _dims: &[usize]) -> Self { self.clone() }
///     fn add(&self, other: &Self) -> Self { Self(self.0 + other.0) }
///     fn multiply(&self, other: &Self) -> Self { Self(self.0 * other.0) }
///     fn reduce_sum(&self, _axes: &[usize]) -> Self { self.clone() }
///     fn dot_general(
///         &self,
///         other: &Self,
///         _lhs_contracting: &[usize],
///         _rhs_contracting: &[usize],
///         _lhs_batch: &[usize],
///         _rhs_batch: &[usize],
///     ) -> Self { Self(self.0 * other.0) }
///     fn conj(&self) -> Self { self.clone() }
/// }
/// ```
pub trait Operand: Clone + Send + Sync + 'static {
    /// Additive identity for the given shape.
    fn zero(shape: &[usize]) -> Self;

    /// Multiplicative identity for the given shape.
    fn one(shape: &[usize]) -> Self;

    /// Returns a reshaped copy of the operand.
    fn reshape(&self, shape: &[usize]) -> Self;

    /// Broadcasts this operand into a larger shape.
    fn broadcast_in_dim(&self, shape: &[usize], dims: &[usize]) -> Self;

    /// Returns the elementwise sum of two operands.
    fn add(&self, other: &Self) -> Self;

    /// Returns the elementwise product of two operands.
    fn multiply(&self, other: &Self) -> Self;

    /// Reduces the operand by summing over the provided axes.
    fn reduce_sum(&self, axes: &[usize]) -> Self;

    /// Performs a generalized dot product between two operands.
    fn dot_general(
        &self,
        other: &Self,
        lhs_contracting: &[usize],
        rhs_contracting: &[usize],
        lhs_batch: &[usize],
        rhs_batch: &[usize],
    ) -> Self;

    /// Returns the complex conjugate of the operand.
    fn conj(&self) -> Self;
}

/// Operation node trait. `computegraph` is fully generic over this abstraction.
///
/// # Examples
///
/// ```ignore
/// use computegraph::{GraphOp, Operand};
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// enum AddOp {
///     Add,
/// }
///
/// impl Operand for f64 {
///     fn zero(_shape: &[usize]) -> Self { 0.0 }
///     fn one(_shape: &[usize]) -> Self { 1.0 }
///     fn reshape(&self, _shape: &[usize]) -> Self { *self }
///     fn broadcast_in_dim(&self, _shape: &[usize], _dims: &[usize]) -> Self { *self }
///     fn add(&self, other: &Self) -> Self { self + other }
///     fn multiply(&self, other: &Self) -> Self { self * other }
///     fn reduce_sum(&self, _axes: &[usize]) -> Self { *self }
///     fn dot_general(
///         &self,
///         other: &Self,
///         _lhs_contracting: &[usize],
///         _rhs_contracting: &[usize],
///         _lhs_batch: &[usize],
///         _rhs_batch: &[usize],
///     ) -> Self { self * other }
///     fn conj(&self) -> Self { *self }
/// }
///
/// impl GraphOp for AddOp {
///     type Operand = f64;
///     type Context = ();
///     type InputKey = &'static str;
///
///     fn n_inputs(&self) -> usize { 2 }
///     fn n_outputs(&self) -> usize { 1 }
///
///     fn eval(&self, _ctx: &mut Self::Context, inputs: &[&Self::Operand]) -> Vec<Self::Operand> {
///         vec![inputs[0] + inputs[1]]
///     }
/// }
/// ```
pub trait GraphOp: Clone + std::fmt::Debug + Hash + Eq + Send + Sync + 'static {
    type Operand: Operand;
    type Context;
    type InputKey: Clone + std::fmt::Debug + Hash + Eq + Send + Sync + 'static;

    /// Returns the number of inputs consumed by this operation.
    fn n_inputs(&self) -> usize;

    /// Returns the number of outputs produced by this operation.
    fn n_outputs(&self) -> usize;

    /// Evaluates the operation given concrete input operands.
    fn eval(&self, ctx: &mut Self::Context, inputs: &[&Self::Operand]) -> Vec<Self::Operand>;
}

impl Operand for f64 {
    fn zero(_shape: &[usize]) -> Self {
        0.0
    }

    fn one(_shape: &[usize]) -> Self {
        1.0
    }

    fn reshape(&self, _shape: &[usize]) -> Self {
        *self
    }

    fn broadcast_in_dim(&self, _shape: &[usize], _dims: &[usize]) -> Self {
        *self
    }

    fn add(&self, other: &Self) -> Self {
        self + other
    }

    fn multiply(&self, other: &Self) -> Self {
        self * other
    }

    fn reduce_sum(&self, _axes: &[usize]) -> Self {
        *self
    }

    fn dot_general(
        &self,
        other: &Self,
        _lhs_contracting: &[usize],
        _rhs_contracting: &[usize],
        _lhs_batch: &[usize],
        _rhs_batch: &[usize],
    ) -> Self {
        self * other
    }

    fn conj(&self) -> Self {
        *self
    }
}
