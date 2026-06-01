use std::collections::HashMap;
use std::sync::Arc;

use crate::resolve::{ResolvedView, ValueDef};
use crate::traits::GraphOperation;
use crate::types::{OperationKey, OperationRole, ValueKey};

/// A value in the materialized graph.
pub struct MaterializedValue<Op: GraphOperation> {
    pub key: ValueKey<Op>,
    /// `None` for inputs; `Some((op_index, output_slot))` for produced values.
    pub producer: Option<(usize, usize)>,
}

/// An operation in the materialized graph.
pub struct MaterializedOperation<Op: GraphOperation> {
    pub operation: Op,
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
    pub role: OperationRole,
}

/// Fully flattened, deduplicated graph ready for compilation.
pub struct MaterializedGraph<Op: GraphOperation> {
    pub values: Vec<MaterializedValue<Op>>,
    pub operations: Vec<MaterializedOperation<Op>>,
    pub inputs: Vec<ValueKey<Op>>,
    pub outputs: Vec<ValueKey<Op>>,
}

struct Materializer<'a, Op: GraphOperation> {
    view: &'a ResolvedView<Op>,
    val_map: HashMap<ValueKey<Op>, usize>,
    op_map: HashMap<Arc<OperationKey<Op>>, usize>,
    values: Vec<MaterializedValue<Op>>,
    operations: Vec<MaterializedOperation<Op>>,
    input_keys: Vec<ValueKey<Op>>,
}

impl<'a, Op: GraphOperation> Materializer<'a, Op> {
    fn new(view: &'a ResolvedView<Op>) -> Self {
        Self {
            view,
            val_map: HashMap::new(),
            op_map: HashMap::new(),
            values: Vec::new(),
            operations: Vec::new(),
            input_keys: Vec::new(),
        }
    }

    fn visit(&mut self, key: &ValueKey<Op>) -> usize {
        if let Some(&index) = self.val_map.get(key) {
            return index;
        }

        let resolved = self.view.resolve_value(key);
        assert!(
            resolved.is_some(),
            "key not found in resolved view: {:?}",
            key
        );
        match resolved {
            Some(ValueDef::Input { .. }) => self.materialize_input(key),
            Some(ValueDef::Produced {
                operation,
                input_keys,
                role,
                output_slot,
            }) => self.materialize_produced(operation, input_keys, role, output_slot),
            None => unreachable!("asserted above"),
        }
    }

    fn materialize_input(&mut self, key: &ValueKey<Op>) -> usize {
        let index = self.values.len();
        self.values.push(MaterializedValue {
            key: key.clone(),
            producer: None,
        });
        self.val_map.insert(key.clone(), index);
        self.input_keys.push(key.clone());
        index
    }

    fn materialize_produced(
        &mut self,
        operation: Op,
        input_keys: Vec<ValueKey<Op>>,
        role: OperationRole,
        output_slot: usize,
    ) -> usize {
        let op_key = Arc::new(OperationKey::new(
            operation.clone(),
            input_keys.clone(),
            role.clone(),
        ));

        if self.op_map.contains_key(&op_key) {
            let output_key = ValueKey::Derived {
                operation: op_key,
                output_slot: output_slot as u8,
            };
            let val_index = self.val_map.get(&output_key).copied();
            assert!(
                val_index.is_some(),
                "materialized op {:?} is missing output slot {}",
                operation,
                output_slot
            );
            return match val_index {
                Some(index) => index,
                None => unreachable!("asserted above"),
            };
        }

        let materialized_inputs = input_keys.iter().map(|input| self.visit(input)).collect();
        let op_index = self.operations.len();
        self.op_map.insert(Arc::clone(&op_key), op_index);
        self.operations.push(MaterializedOperation {
            operation: operation.clone(),
            inputs: materialized_inputs,
            outputs: Vec::with_capacity(operation.output_count()),
            role,
        });

        for slot in 0..operation.output_count() {
            let output_key = ValueKey::Derived {
                operation: Arc::clone(&op_key),
                output_slot: slot as u8,
            };
            let val_index = self.values.len();
            self.values.push(MaterializedValue {
                key: output_key.clone(),
                producer: Some((op_index, slot)),
            });
            self.val_map.insert(output_key, val_index);
            self.operations[op_index].outputs.push(val_index);
        }

        self.operations[op_index].outputs[output_slot]
    }
}

/// Flattens resolved graphs into a single materialized graph.
pub fn materialize_merge<Op: GraphOperation>(
    view: &ResolvedView<Op>,
    outputs: &[ValueKey<Op>],
) -> MaterializedGraph<Op> {
    let mut materializer = Materializer::new(view);

    for output in outputs {
        materializer.visit(output);
    }

    MaterializedGraph {
        values: materializer.values,
        operations: materializer.operations,
        inputs: materializer.input_keys,
        outputs: outputs.to_vec(),
    }
}
