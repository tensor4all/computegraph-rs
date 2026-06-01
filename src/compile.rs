use std::collections::HashMap;

use crate::materialize::MaterializedGraph;
use crate::traits::GraphOperation;
use crate::types::ValueKey;

/// A single instruction in the compiled program.
pub struct Instruction<Op: GraphOperation> {
    pub operation: Op,
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
}

/// SSA-form compiled program. Each slot is written exactly once.
pub struct CompiledProgram<Op: GraphOperation> {
    pub instructions: Vec<Instruction<Op>>,
    pub input_slots: Vec<usize>,
    pub output_slots: Vec<usize>,
    pub n_slots: usize,
}

/// Compiles a materialized graph into an SSA instruction sequence.
pub fn compile<Op: GraphOperation>(graph: &MaterializedGraph<Op>) -> CompiledProgram<Op> {
    let instructions = graph
        .operations
        .iter()
        .map(|op_node| Instruction {
            operation: op_node.operation.clone(),
            inputs: op_node.inputs.clone(),
            outputs: op_node.outputs.clone(),
        })
        .collect();

    let input_slots = graph
        .values
        .iter()
        .enumerate()
        .filter(|(_, val)| val.producer.is_none())
        .map(|(index, _)| index)
        .collect();

    let key_to_index: HashMap<&ValueKey<Op>, usize> = graph
        .values
        .iter()
        .enumerate()
        .map(|(index, val)| (&val.key, index))
        .collect();

    let output_slots = graph
        .outputs
        .iter()
        .map(|key| {
            let slot = key_to_index.get(key).copied();
            assert!(
                slot.is_some(),
                "materialized graph is missing output {:?}",
                key
            );
            match slot {
                Some(index) => index,
                None => unreachable!("asserted above"),
            }
        })
        .collect();

    CompiledProgram {
        instructions,
        input_slots,
        output_slots,
        n_slots: graph.values.len(),
    }
}
