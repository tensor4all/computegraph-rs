use crate::compile::CompiledProgram;
use crate::traits::EvalGraphOp;

impl<Op: EvalGraphOp> CompiledProgram<Op> {
    /// Executes the compiled program with the given inputs.
    pub fn eval(&self, ctx: &mut Op::Context, inputs: &[&Op::Operand]) -> Vec<Op::Operand> {
        assert_eq!(
            inputs.len(),
            self.input_slots.len(),
            "expected {} inputs, got {}",
            self.input_slots.len(),
            inputs.len()
        );

        let mut slots: Vec<Option<Op::Operand>> = vec![None; self.n_slots];
        for (index, &slot) in self.input_slots.iter().enumerate() {
            slots[slot] = Some(inputs[index].clone());
        }

        for instruction in &self.instructions {
            let input_vals: Vec<&Op::Operand> = instruction
                .inputs
                .iter()
                .map(|&slot| {
                    let value = slots[slot].as_ref();
                    assert!(value.is_some(), "input slot {} was not filled", slot);
                    match value {
                        Some(value) => value,
                        None => unreachable!("asserted above"),
                    }
                })
                .collect();

            let outputs = instruction.op.eval(ctx, &input_vals);
            assert_eq!(
                outputs.len(),
                instruction.outputs.len(),
                "operation {:?} produced {} outputs, expected {}",
                instruction.op,
                outputs.len(),
                instruction.outputs.len()
            );

            for (output, &slot) in outputs.iter().zip(&instruction.outputs) {
                slots[slot] = Some(output.clone());
            }
        }

        self.output_slots
            .iter()
            .map(|&slot| {
                let value = slots[slot].as_ref();
                assert!(value.is_some(), "output slot {} was not filled", slot);
                match value {
                    Some(value) => value.clone(),
                    None => unreachable!("asserted above"),
                }
            })
            .collect()
    }
}
