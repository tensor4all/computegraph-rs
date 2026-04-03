use crate::traits::GraphOp;

/// Fragment-local value identifier.
pub type LocalValId = usize;

/// Fragment-local operation identifier.
pub type LocalOpId = usize;

/// Distinguishes primal nodes from linear (AD-generated) nodes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OpMode {
    Primal,
    Linear { active_mask: Vec<bool> },
}

/// Reference to a value: either local to the current fragment or external.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ValRef<Op: GraphOp> {
    Local(LocalValId),
    External(GlobalValKey<Op>),
}

/// Cross-fragment structural identity for a value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GlobalValKey<Op: GraphOp> {
    Input(Op::InputKey),
    Derived {
        op: GlobalOpKey<Op>,
        output_slot: u8,
    },
}

/// Cross-fragment structural identity for an operation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlobalOpKey<Op: GraphOp> {
    pub primitive: Op,
    pub inputs: Vec<GlobalValKey<Op>>,
    pub mode: OpMode,
}
