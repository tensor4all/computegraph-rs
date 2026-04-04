use computegraph::{EvalGraphOp, GraphOp};

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
}

impl EvalGraphOp for ScalarOp {
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
