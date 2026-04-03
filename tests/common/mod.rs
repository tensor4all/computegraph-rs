use computegraph::{GraphOp, Operand};

/// Scalar operations for testing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScalarOp {
    Add,
    Mul,
    Exp,
    Neg,
    Dup,
}

impl GraphOp for ScalarOp {
    type Operand = f64;
    type Context = ();
    type InputKey = String;

    fn n_inputs(&self) -> usize {
        match self {
            ScalarOp::Add | ScalarOp::Mul => 2,
            ScalarOp::Exp | ScalarOp::Neg | ScalarOp::Dup => 1,
        }
    }

    fn n_outputs(&self) -> usize {
        match self {
            ScalarOp::Dup => 2,
            _ => 1,
        }
    }

    fn eval(&self, _ctx: &mut (), inputs: &[&f64]) -> Vec<f64> {
        match self {
            ScalarOp::Add => vec![inputs[0] + inputs[1]],
            ScalarOp::Mul => vec![inputs[0] * inputs[1]],
            ScalarOp::Exp => vec![inputs[0].exp()],
            ScalarOp::Neg => vec![-inputs[0]],
            ScalarOp::Dup => vec![*inputs[0], *inputs[0]],
        }
    }
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
