use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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
#[derive(Clone, Debug)]
pub enum GlobalValKey<Op: GraphOp> {
    Input(Op::InputKey),
    Derived {
        /// Shared structural identity of the operation that produced this value.
        op: Arc<GlobalOpKey<Op>>,
        output_slot: u8,
    },
}

/// Cross-fragment structural identity for an operation.
///
/// `GlobalOpKey` caches a structural fingerprint so maps keyed by recursively
/// derived values can avoid repeatedly re-hashing the whole input tree. Equality
/// still checks the full structure after the fingerprint prefilter.
#[derive(Clone, Debug)]
pub struct GlobalOpKey<Op: GraphOp> {
    primitive: Op,
    inputs: Vec<GlobalValKey<Op>>,
    mode: OpMode,
    /// Cached hash prefilter for recursively structural keys.
    ///
    /// This is not an identity proof: equality still compares the full
    /// structure after the fingerprint matches, so hash collisions remain
    /// correct.
    fingerprint: u64,
}

impl<Op: GraphOp> GlobalOpKey<Op> {
    /// Builds an operation key and precomputes its structural fingerprint.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use computegraph::{GlobalOpKey, GlobalValKey, GraphOp, OpMode};
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// enum Op {
    ///     Add,
    /// }
    ///
    /// impl GraphOp for Op {
    ///     type Operand = f64;
    ///     type Context = ();
    ///     type InputKey = &'static str;
    ///
    ///     fn n_inputs(&self) -> usize { 2 }
    ///     fn n_outputs(&self) -> usize { 1 }
    /// }
    ///
    /// let key = GlobalOpKey::new(
    ///     Op::Add,
    ///     vec![GlobalValKey::Input("x"), GlobalValKey::Input("y")],
    ///     OpMode::Primal,
    /// );
    /// assert_eq!(key.inputs().len(), 2);
    /// ```
    pub fn new(primitive: Op, inputs: Vec<GlobalValKey<Op>>, mode: OpMode) -> Self {
        let fingerprint = fingerprint_op(&primitive, &inputs, &mode);
        Self {
            primitive,
            inputs,
            mode,
            fingerprint,
        }
    }

    /// Returns the cached structural fingerprint.
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    /// Returns the operation primitive.
    pub fn primitive(&self) -> &Op {
        &self.primitive
    }

    /// Returns the structural input keys.
    pub fn inputs(&self) -> &[GlobalValKey<Op>] {
        &self.inputs
    }

    /// Returns whether this operation belongs to the primal or linear graph.
    pub fn mode(&self) -> &OpMode {
        &self.mode
    }
}

impl<Op: GraphOp> PartialEq for GlobalValKey<Op> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Input(lhs), Self::Input(rhs)) => lhs == rhs,
            (
                Self::Derived {
                    op: lhs_op,
                    output_slot: lhs_slot,
                },
                Self::Derived {
                    op: rhs_op,
                    output_slot: rhs_slot,
                },
            ) => {
                lhs_slot == rhs_slot
                    && (Arc::ptr_eq(lhs_op, rhs_op) || lhs_op.as_ref() == rhs_op.as_ref())
            }
            _ => false,
        }
    }
}

impl<Op: GraphOp> Eq for GlobalValKey<Op> {}

impl<Op: GraphOp> Hash for GlobalValKey<Op> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Input(key) => {
                0u8.hash(state);
                key.hash(state);
            }
            Self::Derived { op, output_slot } => {
                1u8.hash(state);
                op.fingerprint.hash(state);
                output_slot.hash(state);
            }
        }
    }
}

impl<Op: GraphOp> PartialEq for GlobalOpKey<Op> {
    fn eq(&self, other: &Self) -> bool {
        self.fingerprint == other.fingerprint
            && self.primitive == other.primitive
            && self.mode == other.mode
            && self.inputs == other.inputs
    }
}

impl<Op: GraphOp> Eq for GlobalOpKey<Op> {}

impl<Op: GraphOp> Hash for GlobalOpKey<Op> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.fingerprint.hash(state);
    }
}

fn fingerprint_op<Op: GraphOp>(primitive: &Op, inputs: &[GlobalValKey<Op>], mode: &OpMode) -> u64 {
    let mut hasher = DefaultHasher::new();
    primitive.hash(&mut hasher);
    mode.hash(&mut hasher);
    inputs.len().hash(&mut hasher);
    for input in inputs {
        fingerprint_val(input).hash(&mut hasher);
    }
    hasher.finish()
}

fn fingerprint_val<Op: GraphOp>(key: &GlobalValKey<Op>) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
