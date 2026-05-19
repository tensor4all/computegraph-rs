use std::sync::Arc;

use crate::traits::GraphOp;
use crate::types::{GlobalOpKey, GlobalValKey, LocalOpId, LocalValId, OpMode, ValRef};

/// A value node in a fragment.
pub struct ValNode<Op: GraphOp> {
    /// Cross-fragment structural identity.
    pub key: GlobalValKey<Op>,
    /// `None` for fragment inputs; `Some((op_id, output_slot))` for produced values.
    pub producer: Option<(LocalOpId, usize)>,
}

/// An operation node in a fragment.
pub struct OpNode<Op: GraphOp> {
    pub op: Op,
    pub inputs: Vec<ValRef<Op>>,
    pub outputs: Vec<LocalValId>,
    pub mode: OpMode,
}

/// The unit of graph construction.
pub struct Fragment<Op: GraphOp> {
    pub(crate) vals: Vec<ValNode<Op>>,
    pub(crate) ops: Vec<OpNode<Op>>,
    pub(crate) inputs: Vec<LocalValId>,
    pub(crate) outputs: Vec<LocalValId>,
    pub(crate) parents: Vec<Arc<Fragment<Op>>>,
}

impl<Op: GraphOp> Fragment<Op> {
    /// Returns all value nodes in this fragment.
    pub fn vals(&self) -> &[ValNode<Op>] {
        &self.vals
    }

    /// Returns all operation nodes in this fragment.
    pub fn ops(&self) -> &[OpNode<Op>] {
        &self.ops
    }

    /// Returns the fragment-local input value ids.
    pub fn inputs(&self) -> &[LocalValId] {
        &self.inputs
    }

    /// Returns the fragment-local output value ids.
    pub fn outputs(&self) -> &[LocalValId] {
        &self.outputs
    }

    /// Returns the parent fragments referenced by this fragment.
    pub fn parents(&self) -> &[Arc<Fragment<Op>>] {
        &self.parents
    }
}

/// Builder for constructing fragments incrementally.
pub struct FragmentBuilder<Op: GraphOp> {
    vals: Vec<ValNode<Op>>,
    ops: Vec<OpNode<Op>>,
    inputs: Vec<LocalValId>,
    outputs: Vec<LocalValId>,
    parents: Vec<Arc<Fragment<Op>>>,
    local_keys: Vec<GlobalValKey<Op>>,
}

impl<Op: GraphOp> FragmentBuilder<Op> {
    /// Creates an empty fragment builder.
    pub fn new() -> Self {
        Self {
            vals: Vec::new(),
            ops: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            parents: Vec::new(),
            local_keys: Vec::new(),
        }
    }

    /// Adds a fragment input and returns its local id.
    pub fn add_input(&mut self, key: Op::InputKey) -> LocalValId {
        let val_id = self.vals.len();
        let global_key = GlobalValKey::Input(key);
        self.vals.push(ValNode {
            key: global_key.clone(),
            producer: None,
        });
        self.local_keys.push(global_key);
        self.inputs.push(val_id);
        val_id
    }

    /// Adds an operation node and returns the local ids for each output.
    pub fn add_op(&mut self, op: Op, inputs: Vec<ValRef<Op>>, mode: OpMode) -> Vec<LocalValId> {
        assert_eq!(
            inputs.len(),
            op.n_inputs(),
            "operation {:?} expected {} inputs, got {}",
            op,
            op.n_inputs(),
            inputs.len()
        );

        let n_outputs = op.n_outputs();
        assert!(
            n_outputs <= u8::MAX as usize + 1,
            "operation {:?} has too many outputs for GlobalValKey: {}",
            op,
            n_outputs
        );

        let op_id = self.ops.len();
        let global_inputs: Vec<GlobalValKey<Op>> = inputs
            .iter()
            .map(|input| self.resolve_input_key(input))
            .collect();

        let global_op_key = Arc::new(GlobalOpKey::new(op.clone(), global_inputs, mode.clone()));

        let mut output_ids = Vec::with_capacity(n_outputs);
        for slot in 0..n_outputs {
            let val_id = self.vals.len();
            let key = GlobalValKey::Derived {
                op: Arc::clone(&global_op_key),
                output_slot: slot as u8,
            };
            self.vals.push(ValNode {
                key: key.clone(),
                producer: Some((op_id, slot)),
            });
            self.local_keys.push(key);
            output_ids.push(val_id);
        }

        self.ops.push(OpNode {
            op,
            inputs,
            outputs: output_ids.clone(),
            mode,
        });

        output_ids
    }

    /// Declares the fragment outputs.
    pub fn set_outputs(&mut self, outputs: Vec<LocalValId>) {
        for &output in &outputs {
            assert!(
                output < self.vals.len(),
                "unknown local output value id {}",
                output
            );
        }
        self.outputs = outputs;
    }

    /// Registers a parent fragment for external reference resolution.
    pub fn add_parent(&mut self, parent: Arc<Fragment<Op>>) {
        self.parents.push(parent);
    }

    /// Returns the global key for a local value id.
    pub fn global_key(&self, local_id: LocalValId) -> &GlobalValKey<Op> {
        assert!(
            local_id < self.local_keys.len(),
            "unknown local value id {}",
            local_id
        );
        &self.local_keys[local_id]
    }

    /// Consumes the builder and produces a fragment.
    pub fn build(self) -> Fragment<Op> {
        Fragment {
            vals: self.vals,
            ops: self.ops,
            inputs: self.inputs,
            outputs: self.outputs,
            parents: self.parents,
        }
    }

    fn resolve_input_key(&self, input: &ValRef<Op>) -> GlobalValKey<Op> {
        match input {
            ValRef::Local(local_id) => self.global_key(*local_id).clone(),
            ValRef::External(key) => key.clone(),
        }
    }
}

impl<Op: GraphOp> Default for FragmentBuilder<Op> {
    fn default() -> Self {
        Self::new()
    }
}
