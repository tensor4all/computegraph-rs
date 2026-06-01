use std::sync::Arc;

use crate::traits::GraphOperation;
use crate::types::{
    LocalOperationId, LocalValueId, OperationKey, OperationRole, ValueKey, ValueRef,
};

/// A value node in a graph.
pub struct ValueNode<Op: GraphOperation> {
    /// Cross-graph structural identity.
    pub key: ValueKey<Op>,
    /// `None` for graph inputs; `Some((op_id, output_slot))` for produced values.
    pub producer: Option<(LocalOperationId, usize)>,
}

/// An operation node in a graph.
pub struct OperationNode<Op: GraphOperation> {
    pub operation: Op,
    pub inputs: Vec<ValueRef<Op>>,
    pub outputs: Vec<LocalValueId>,
    pub role: OperationRole,
}

/// The unit of graph construction.
///
/// # Examples
///
/// ```
/// use computegraph::graph::GraphBuilder;
/// use computegraph::{GraphOperation, OperationRole, ValueRef};
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// enum IdentityOp {
///     Identity,
/// }
///
/// impl GraphOperation for IdentityOp {
///     type Operand = f64;
///     type Context = ();
///     type InputKey = &'static str;
///
///     fn input_count(&self) -> usize { 1 }
///     fn output_count(&self) -> usize { 1 }
/// }
///
/// let mut builder = GraphBuilder::<IdentityOp>::new();
/// let x = builder.add_input("x");
/// let y = builder.add_operation(
///     IdentityOp::Identity,
///     vec![ValueRef::Local(x)],
///     OperationRole::Primary,
/// );
/// builder.set_outputs(y.clone());
/// let graph = builder.build();
///
/// assert_eq!(graph.inputs(), &[x]);
/// assert_eq!(graph.outputs(), y.as_slice());
/// ```
pub struct Graph<Op: GraphOperation> {
    pub(crate) values: Vec<ValueNode<Op>>,
    pub(crate) operations: Vec<OperationNode<Op>>,
    pub(crate) inputs: Vec<LocalValueId>,
    pub(crate) outputs: Vec<LocalValueId>,
    pub(crate) parents: Vec<Arc<Graph<Op>>>,
}

impl<Op: GraphOperation> Graph<Op> {
    /// Returns all value nodes in this graph.
    pub fn values(&self) -> &[ValueNode<Op>] {
        &self.values
    }

    /// Returns all operation nodes in this graph.
    pub fn operations(&self) -> &[OperationNode<Op>] {
        &self.operations
    }

    /// Returns the graph-local input value ids.
    pub fn inputs(&self) -> &[LocalValueId] {
        &self.inputs
    }

    /// Returns the graph-local output value ids.
    pub fn outputs(&self) -> &[LocalValueId] {
        &self.outputs
    }

    /// Returns the parent graphs referenced by this graph.
    pub fn parents(&self) -> &[Arc<Graph<Op>>] {
        &self.parents
    }
}

/// Builder for constructing graphs incrementally.
pub struct GraphBuilder<Op: GraphOperation> {
    values: Vec<ValueNode<Op>>,
    operations: Vec<OperationNode<Op>>,
    inputs: Vec<LocalValueId>,
    outputs: Vec<LocalValueId>,
    parents: Vec<Arc<Graph<Op>>>,
    local_keys: Vec<ValueKey<Op>>,
}

impl<Op: GraphOperation> GraphBuilder<Op> {
    /// Creates an empty graph builder.
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            operations: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            parents: Vec::new(),
            local_keys: Vec::new(),
        }
    }

    /// Adds a graph input and returns its local id.
    pub fn add_input(&mut self, key: Op::InputKey) -> LocalValueId {
        let val_id = self.values.len();
        let global_key = ValueKey::Input(key);
        self.values.push(ValueNode {
            key: global_key.clone(),
            producer: None,
        });
        self.local_keys.push(global_key);
        self.inputs.push(val_id);
        val_id
    }

    /// Adds an operation node and returns the local ids for each output.
    pub fn add_operation(
        &mut self,
        operation: Op,
        inputs: Vec<ValueRef<Op>>,
        role: OperationRole,
    ) -> Vec<LocalValueId> {
        assert_eq!(
            inputs.len(),
            operation.input_count(),
            "operation {:?} expected {} inputs, got {}",
            operation,
            operation.input_count(),
            inputs.len()
        );

        let output_count = operation.output_count();
        assert!(
            output_count <= u8::MAX as usize + 1,
            "operation {:?} has too many outputs for ValueKey: {}",
            operation,
            output_count
        );

        let op_id = self.operations.len();
        let global_inputs: Vec<ValueKey<Op>> = inputs
            .iter()
            .map(|input| self.resolve_input_key(input))
            .collect();

        let global_op_key = Arc::new(OperationKey::new(
            operation.clone(),
            global_inputs,
            role.clone(),
        ));

        let mut output_ids = Vec::with_capacity(output_count);
        for slot in 0..output_count {
            let val_id = self.values.len();
            let key = ValueKey::Derived {
                operation: Arc::clone(&global_op_key),
                output_slot: slot as u8,
            };
            self.values.push(ValueNode {
                key: key.clone(),
                producer: Some((op_id, slot)),
            });
            self.local_keys.push(key);
            output_ids.push(val_id);
        }

        self.operations.push(OperationNode {
            operation,
            inputs,
            outputs: output_ids.clone(),
            role,
        });

        output_ids
    }

    /// Declares the graph outputs.
    pub fn set_outputs(&mut self, outputs: Vec<LocalValueId>) {
        for &output in &outputs {
            assert!(
                output < self.values.len(),
                "unknown local output value id {}",
                output
            );
        }
        self.outputs = outputs;
    }

    /// Registers a parent graph for external reference resolution.
    pub fn add_parent(&mut self, parent: Arc<Graph<Op>>) {
        self.parents.push(parent);
    }

    /// Returns the global key for a local value id.
    pub fn global_key(&self, local_id: LocalValueId) -> &ValueKey<Op> {
        assert!(
            local_id < self.local_keys.len(),
            "unknown local value id {}",
            local_id
        );
        &self.local_keys[local_id]
    }

    /// Consumes the builder and produces a graph.
    pub fn build(self) -> Graph<Op> {
        Graph {
            values: self.values,
            operations: self.operations,
            inputs: self.inputs,
            outputs: self.outputs,
            parents: self.parents,
        }
    }

    fn resolve_input_key(&self, input: &ValueRef<Op>) -> ValueKey<Op> {
        match input {
            ValueRef::Local(local_id) => self.global_key(*local_id).clone(),
            ValueRef::External(key) => key.clone(),
        }
    }
}

impl<Op: GraphOperation> Default for GraphBuilder<Op> {
    fn default() -> Self {
        Self::new()
    }
}
