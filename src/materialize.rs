use std::collections::HashMap;
use std::sync::Arc;

use crate::resolve::{ResolvedView, ValDef};
use crate::traits::GraphOp;
use crate::types::{GlobalOpKey, GlobalValKey, OpMode};

/// A value in the materialized graph.
pub struct MaterializedVal<Op: GraphOp> {
    pub key: GlobalValKey<Op>,
    /// `None` for inputs; `Some((op_index, output_slot))` for produced values.
    pub producer: Option<(usize, usize)>,
}

/// An operation in the materialized graph.
pub struct MaterializedOp<Op: GraphOp> {
    pub op: Op,
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
    pub mode: OpMode,
}

/// Fully flattened, deduplicated graph ready for compilation.
pub struct MaterializedGraph<Op: GraphOp> {
    pub vals: Vec<MaterializedVal<Op>>,
    pub ops: Vec<MaterializedOp<Op>>,
    pub inputs: Vec<GlobalValKey<Op>>,
    pub outputs: Vec<GlobalValKey<Op>>,
}

struct Materializer<'a, Op: GraphOp> {
    view: &'a ResolvedView<Op>,
    val_map: HashMap<GlobalValKey<Op>, usize>,
    op_map: HashMap<Arc<GlobalOpKey<Op>>, usize>,
    vals: Vec<MaterializedVal<Op>>,
    ops: Vec<MaterializedOp<Op>>,
    input_keys: Vec<GlobalValKey<Op>>,
}

impl<'a, Op: GraphOp> Materializer<'a, Op> {
    fn new(view: &'a ResolvedView<Op>) -> Self {
        Self {
            view,
            val_map: HashMap::new(),
            op_map: HashMap::new(),
            vals: Vec::new(),
            ops: Vec::new(),
            input_keys: Vec::new(),
        }
    }

    fn visit(&mut self, key: &GlobalValKey<Op>) -> usize {
        if let Some(&index) = self.val_map.get(key) {
            return index;
        }

        let resolved = self.view.resolve_val(key);
        assert!(
            resolved.is_some(),
            "key not found in resolved view: {:?}",
            key
        );
        match resolved {
            Some(ValDef::Input { .. }) => self.materialize_input(key),
            Some(ValDef::Produced {
                op,
                input_keys,
                mode,
                output_slot,
            }) => self.materialize_produced(op, input_keys, mode, output_slot),
            None => unreachable!("asserted above"),
        }
    }

    fn materialize_input(&mut self, key: &GlobalValKey<Op>) -> usize {
        let index = self.vals.len();
        self.vals.push(MaterializedVal {
            key: key.clone(),
            producer: None,
        });
        self.val_map.insert(key.clone(), index);
        self.input_keys.push(key.clone());
        index
    }

    fn materialize_produced(
        &mut self,
        op: Op,
        input_keys: Vec<GlobalValKey<Op>>,
        mode: OpMode,
        output_slot: usize,
    ) -> usize {
        let op_key = Arc::new(GlobalOpKey::new(
            op.clone(),
            input_keys.clone(),
            mode.clone(),
        ));

        if self.op_map.contains_key(&op_key) {
            let output_key = GlobalValKey::Derived {
                op: op_key,
                output_slot: output_slot as u8,
            };
            let val_index = self.val_map.get(&output_key).copied();
            assert!(
                val_index.is_some(),
                "materialized op {:?} is missing output slot {}",
                op,
                output_slot
            );
            return match val_index {
                Some(index) => index,
                None => unreachable!("asserted above"),
            };
        }

        let materialized_inputs = input_keys.iter().map(|input| self.visit(input)).collect();
        let op_index = self.ops.len();
        self.op_map.insert(Arc::clone(&op_key), op_index);
        self.ops.push(MaterializedOp {
            op: op.clone(),
            inputs: materialized_inputs,
            outputs: Vec::with_capacity(op.n_outputs()),
            mode,
        });

        for slot in 0..op.n_outputs() {
            let output_key = GlobalValKey::Derived {
                op: Arc::clone(&op_key),
                output_slot: slot as u8,
            };
            let val_index = self.vals.len();
            self.vals.push(MaterializedVal {
                key: output_key.clone(),
                producer: Some((op_index, slot)),
            });
            self.val_map.insert(output_key, val_index);
            self.ops[op_index].outputs.push(val_index);
        }

        self.ops[op_index].outputs[output_slot]
    }
}

/// Flattens resolved fragments into a single materialized graph.
pub fn materialize_merge<Op: GraphOp>(
    view: &ResolvedView<Op>,
    outputs: &[GlobalValKey<Op>],
) -> MaterializedGraph<Op> {
    let mut materializer = Materializer::new(view);

    for output in outputs {
        materializer.visit(output);
    }

    MaterializedGraph {
        vals: materializer.vals,
        ops: materializer.ops,
        inputs: materializer.input_keys,
        outputs: outputs.to_vec(),
    }
}
