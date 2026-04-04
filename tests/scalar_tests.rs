mod common;

use std::sync::Arc;

use common::ScalarOp;
use computegraph::compile::compile;
use computegraph::fragment::FragmentBuilder;
use computegraph::interner::KeyInterner;
use computegraph::materialize::materialize_merge;
use computegraph::resolve::{resolve, ValDef};
use computegraph::{EvalGraphOp, GlobalOpKey, GlobalValKey, GraphOp, OpMode, ValRef};

// === ScalarOp smoke tests ===

#[test]
fn scalar_op_eval_add() {
    let op = ScalarOp::Add;
    assert_eq!(op.n_inputs(), 2);
    assert_eq!(op.n_outputs(), 1);
    let result = op.eval(&mut (), &[&3.0, &4.0]);
    assert_eq!(result, vec![7.0]);
}

#[test]
fn scalar_op_eval_exp() {
    let op = ScalarOp::Exp;
    let result = op.eval(&mut (), &[&0.0]);
    assert_eq!(result, vec![1.0]);
}

#[test]
fn scalar_op_eval_dup() {
    let op = ScalarOp::Dup;
    assert_eq!(op.n_outputs(), 2);
    let result = op.eval(&mut (), &[&5.0]);
    assert_eq!(result, vec![5.0, 5.0]);
}

// === KeyInterner tests ===

#[test]
fn interner_intern_and_resolve() {
    let mut interner = KeyInterner::<ScalarOp>::new();
    let key = GlobalValKey::Input("x".to_string());
    let id = interner.intern(key.clone());
    assert_eq!(interner.resolve(id), &key);
}

#[test]
fn interner_deduplicates() {
    let mut interner = KeyInterner::<ScalarOp>::new();
    let key = GlobalValKey::Input("x".to_string());
    let id1 = interner.intern(key.clone());
    let id2 = interner.intern(key);
    assert_eq!(id1, id2);
}

#[test]
fn interner_distinct_keys_get_distinct_ids() {
    let mut interner = KeyInterner::<ScalarOp>::new();
    let id_x = interner.intern(GlobalValKey::Input("x".to_string()));
    let id_y = interner.intern(GlobalValKey::Input("y".to_string()));
    assert_ne!(id_x, id_y);
}

#[test]
fn interner_get_returns_none_for_unknown() {
    let interner = KeyInterner::<ScalarOp>::new();
    let key = GlobalValKey::Input("x".to_string());
    assert_eq!(interner.get(&key), None);
}

#[test]
fn interner_derived_key() {
    let mut interner = KeyInterner::<ScalarOp>::new();
    let key = GlobalValKey::<ScalarOp>::Derived {
        op: GlobalOpKey {
            primitive: ScalarOp::Add,
            inputs: vec![
                GlobalValKey::Input("x".to_string()),
                GlobalValKey::Input("y".to_string()),
            ],
            mode: OpMode::Primal,
        },
        output_slot: 0,
    };
    let id = interner.intern(key.clone());
    assert_eq!(interner.resolve(id), &key);
    assert_eq!(interner.get(&key), Some(id));
}

// === Fragment tests ===

#[test]
fn fragment_builder_single_input() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    assert_eq!(x, 0);
    builder.set_outputs(vec![x]);
    let frag = builder.build();
    assert_eq!(frag.inputs().len(), 1);
    assert_eq!(frag.outputs().len(), 1);
    assert_eq!(frag.vals()[x].key, GlobalValKey::Input("x".to_string()));
    assert!(frag.vals()[x].producer.is_none());
}

#[test]
fn fragment_builder_add_op() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let outputs = builder.add_op(
        ScalarOp::Add,
        vec![ValRef::Local(x), ValRef::Local(y)],
        OpMode::Primal,
    );
    assert_eq!(outputs.len(), 1);
    let sum_id = outputs[0];
    builder.set_outputs(vec![sum_id]);
    let frag = builder.build();

    assert_eq!(frag.ops().len(), 1);
    assert_eq!(frag.ops()[0].op, ScalarOp::Add);
    assert!(frag.vals()[sum_id].producer.is_some());

    // Verify GlobalValKey structure
    let expected_key = GlobalValKey::Derived {
        op: GlobalOpKey {
            primitive: ScalarOp::Add,
            inputs: vec![
                GlobalValKey::Input("x".to_string()),
                GlobalValKey::Input("y".to_string()),
            ],
            mode: OpMode::Primal,
        },
        output_slot: 0,
    };
    assert_eq!(frag.vals()[sum_id].key, expected_key);
}

#[test]
fn fragment_builder_chain() {
    // Build: Exp(Mul(x, a))
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul_out = builder.add_op(
        ScalarOp::Mul,
        vec![ValRef::Local(x), ValRef::Local(a)],
        OpMode::Primal,
    );
    let exp_out = builder.add_op(
        ScalarOp::Exp,
        vec![ValRef::Local(mul_out[0])],
        OpMode::Primal,
    );
    builder.set_outputs(vec![exp_out[0]]);
    let frag = builder.build();

    assert_eq!(frag.ops().len(), 2);
    assert_eq!(frag.vals().len(), 4); // x, a, mul_out, exp_out
}

#[test]
fn fragment_builder_dup_two_outputs() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let dup_outs = builder.add_op(ScalarOp::Dup, vec![ValRef::Local(x)], OpMode::Primal);
    assert_eq!(dup_outs.len(), 2);
    builder.set_outputs(dup_outs.clone());
    let frag = builder.build();

    assert_eq!(frag.outputs().len(), 2);
    // Both outputs should be Derived with different output_slot
    let key0 = &frag.vals()[dup_outs[0]].key;
    let key1 = &frag.vals()[dup_outs[1]].key;
    match (key0, key1) {
        (
            GlobalValKey::Derived {
                output_slot: s0, ..
            },
            GlobalValKey::Derived {
                output_slot: s1, ..
            },
        ) => {
            assert_eq!(*s0, 0);
            assert_eq!(*s1, 1);
        }
        _ => panic!("expected Derived keys"),
    }
}

// === Resolve tests ===

#[test]
fn resolve_single_fragment() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let sum = builder.add_op(
        ScalarOp::Add,
        vec![ValRef::Local(x), ValRef::Local(y)],
        OpMode::Primal,
    );
    builder.set_outputs(vec![sum[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag.clone()]);

    // Input keys should resolve
    let x_key = GlobalValKey::Input("x".to_string());
    match view.resolve_val(&x_key).unwrap() {
        ValDef::Input { key } => assert_eq!(key, "x"),
        _ => panic!("expected Input"),
    }

    // Derived key should resolve
    let sum_key = &frag.vals()[sum[0]].key;
    match view.resolve_val(sum_key).unwrap() {
        ValDef::Produced {
            op,
            input_keys,
            mode,
            output_slot,
        } => {
            assert_eq!(op, ScalarOp::Add);
            assert_eq!(input_keys.len(), 2);
            assert_eq!(mode, OpMode::Primal);
            assert_eq!(output_slot, 0);
        }
        _ => panic!("expected Produced"),
    }
}

#[test]
fn resolve_external_ref_across_fragments() {
    // Fragment F0: x, a, mul = Mul(x, a)
    let mut b0 = FragmentBuilder::<ScalarOp>::new();
    let x = b0.add_input("x".to_string());
    let a = b0.add_input("a".to_string());
    let mul = b0.add_op(
        ScalarOp::Mul,
        vec![ValRef::Local(x), ValRef::Local(a)],
        OpMode::Primal,
    );
    let mul_key = b0.global_key(mul[0]).clone();
    b0.set_outputs(vec![mul[0]]);
    let f0 = Arc::new(b0.build());

    // Fragment F1: references F0's mul output via External, applies Exp
    let mut b1 = FragmentBuilder::<ScalarOp>::new();
    b1.add_parent(f0.clone());
    let exp = b1.add_op(
        ScalarOp::Exp,
        vec![ValRef::External(mul_key.clone())],
        OpMode::Primal,
    );
    b1.set_outputs(vec![exp[0]]);
    let f1 = Arc::new(b1.build());

    let view = resolve(vec![f0, f1.clone()]);

    // mul_key should be resolvable
    assert!(view.resolve_val(&mul_key).is_some());

    // exp output should be resolvable
    let exp_key = &f1.vals()[exp[0]].key;
    match view.resolve_val(exp_key).unwrap() {
        ValDef::Produced { op, input_keys, .. } => {
            assert_eq!(op, ScalarOp::Exp);
            assert_eq!(input_keys.len(), 1);
            assert_eq!(input_keys[0], mul_key);
        }
        _ => panic!("expected Produced"),
    }
}

#[test]
fn resolve_unknown_key_returns_none() {
    let builder = FragmentBuilder::<ScalarOp>::new();
    let frag = Arc::new(builder.build());
    let view = resolve(vec![frag]);
    let unknown = GlobalValKey::Input("unknown".to_string());
    assert!(view.resolve_val(&unknown).is_none());
}

// === Materialize tests ===

#[test]
fn materialize_single_op() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let sum = builder.add_op(
        ScalarOp::Add,
        vec![ValRef::Local(x), ValRef::Local(y)],
        OpMode::Primal,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[sum_key]);

    assert_eq!(graph.ops.len(), 1);
    assert_eq!(graph.ops[0].op, ScalarOp::Add);
    assert_eq!(graph.vals.len(), 3);
    assert_eq!(graph.inputs.len(), 2);
    assert_eq!(graph.outputs.len(), 1);
}

#[test]
fn materialize_chain() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul = builder.add_op(
        ScalarOp::Mul,
        vec![ValRef::Local(x), ValRef::Local(a)],
        OpMode::Primal,
    );
    let exp = builder.add_op(ScalarOp::Exp, vec![ValRef::Local(mul[0])], OpMode::Primal);
    let exp_key = builder.global_key(exp[0]).clone();
    builder.set_outputs(vec![exp[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[exp_key]);

    assert_eq!(graph.ops.len(), 2);
    assert_eq!(graph.vals.len(), 4);
    assert_eq!(graph.ops[0].op, ScalarOp::Mul);
    assert_eq!(graph.ops[1].op, ScalarOp::Exp);
}

#[test]
fn materialize_cse_deduplicates() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let sum = builder.add_op(
        ScalarOp::Add,
        vec![ValRef::Local(x), ValRef::Local(x)],
        OpMode::Primal,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[sum_key]);

    assert_eq!(graph.vals.len(), 2);
    assert_eq!(graph.ops.len(), 1);
    assert_eq!(graph.ops[0].inputs[0], graph.ops[0].inputs[1]);
}

#[test]
fn materialize_across_fragments() {
    let mut b0 = FragmentBuilder::<ScalarOp>::new();
    let x = b0.add_input("x".to_string());
    let a = b0.add_input("a".to_string());
    let mul = b0.add_op(
        ScalarOp::Mul,
        vec![ValRef::Local(x), ValRef::Local(a)],
        OpMode::Primal,
    );
    let mul_key = b0.global_key(mul[0]).clone();
    b0.set_outputs(vec![mul[0]]);
    let f0 = Arc::new(b0.build());

    let mut b1 = FragmentBuilder::<ScalarOp>::new();
    b1.add_parent(f0.clone());
    let exp = b1.add_op(
        ScalarOp::Exp,
        vec![ValRef::External(mul_key)],
        OpMode::Primal,
    );
    let exp_key = b1.global_key(exp[0]).clone();
    b1.set_outputs(vec![exp[0]]);
    let f1 = Arc::new(b1.build());

    let view = resolve(vec![f0, f1]);
    let graph = materialize_merge(&view, &[exp_key]);

    assert_eq!(graph.ops.len(), 2);
    assert_eq!(graph.vals.len(), 4);
}

// === Compile + Eval tests ===

#[test]
fn compile_and_eval_add() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let sum = builder.add_op(
        ScalarOp::Add,
        vec![ValRef::Local(x), ValRef::Local(y)],
        OpMode::Primal,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[sum_key]);
    let prog = compile(&graph);

    assert_eq!(prog.input_slots.len(), 2);
    assert_eq!(prog.output_slots.len(), 1);
    assert_eq!(prog.instructions.len(), 1);

    let result = prog.eval(&mut (), &[&3.0, &4.0]);
    assert_eq!(result, vec![7.0]);
}

#[test]
fn compile_and_eval_chain() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul = builder.add_op(
        ScalarOp::Mul,
        vec![ValRef::Local(x), ValRef::Local(a)],
        OpMode::Primal,
    );
    let exp = builder.add_op(ScalarOp::Exp, vec![ValRef::Local(mul[0])], OpMode::Primal);
    let exp_key = builder.global_key(exp[0]).clone();
    builder.set_outputs(vec![exp[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[exp_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&1.0, &2.0]);
    assert!((result[0] - 2.0_f64.exp()).abs() < 1e-12);
}

#[test]
fn compile_and_eval_reuse() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let sum = builder.add_op(
        ScalarOp::Add,
        vec![ValRef::Local(x), ValRef::Local(y)],
        OpMode::Primal,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[sum_key]);
    let prog = compile(&graph);

    assert_eq!(prog.eval(&mut (), &[&1.0, &2.0]), vec![3.0]);
    assert_eq!(prog.eval(&mut (), &[&10.0, &20.0]), vec![30.0]);
}

#[test]
fn compile_and_eval_multi_output() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul = builder.add_op(
        ScalarOp::Mul,
        vec![ValRef::Local(x), ValRef::Local(a)],
        OpMode::Primal,
    );
    let exp = builder.add_op(ScalarOp::Exp, vec![ValRef::Local(mul[0])], OpMode::Primal);
    let mul_key = builder.global_key(mul[0]).clone();
    let exp_key = builder.global_key(exp[0]).clone();
    builder.set_outputs(vec![mul[0], exp[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[mul_key, exp_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&1.0, &2.0]);
    assert_eq!(result.len(), 2);
    assert!((result[0] - 2.0).abs() < 1e-12);
    assert!((result[1] - 2.0_f64.exp()).abs() < 1e-12);
}

// === End-to-end integration tests ===

#[test]
fn e2e_exp_ax_single_fragment() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul = builder.add_op(
        ScalarOp::Mul,
        vec![ValRef::Local(x), ValRef::Local(a)],
        OpMode::Primal,
    );
    let exp = builder.add_op(ScalarOp::Exp, vec![ValRef::Local(mul[0])], OpMode::Primal);
    let exp_key = builder.global_key(exp[0]).clone();
    builder.set_outputs(vec![exp[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[exp_key]);
    let prog = compile(&graph);
    let result = prog.eval(&mut (), &[&2.0, &3.0]);

    assert!((result[0] - 6.0_f64.exp()).abs() < 1e-12);
}

#[test]
fn e2e_exp_ax_multi_fragment() {
    let mut b0 = FragmentBuilder::<ScalarOp>::new();
    let x = b0.add_input("x".to_string());
    let a = b0.add_input("a".to_string());
    let mul = b0.add_op(
        ScalarOp::Mul,
        vec![ValRef::Local(x), ValRef::Local(a)],
        OpMode::Primal,
    );
    let mul_key = b0.global_key(mul[0]).clone();
    b0.set_outputs(vec![mul[0]]);
    let f0 = Arc::new(b0.build());

    let mut b1 = FragmentBuilder::<ScalarOp>::new();
    b1.add_parent(f0.clone());
    let exp = b1.add_op(
        ScalarOp::Exp,
        vec![ValRef::External(mul_key)],
        OpMode::Primal,
    );
    let exp_key = b1.global_key(exp[0]).clone();
    b1.set_outputs(vec![exp[0]]);
    let f1 = Arc::new(b1.build());

    let view = resolve(vec![f0, f1]);
    let graph = materialize_merge(&view, &[exp_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&2.0, &3.0]);
    assert!((result[0] - 6.0_f64.exp()).abs() < 1e-12);

    let result2 = prog.eval(&mut (), &[&1.0, &1.0]);
    assert!((result2[0] - 1.0_f64.exp()).abs() < 1e-12);
}

#[test]
fn e2e_x_plus_x() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let sum = builder.add_op(
        ScalarOp::Add,
        vec![ValRef::Local(x), ValRef::Local(x)],
        OpMode::Primal,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[sum_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&5.0]);
    assert_eq!(result, vec![10.0]);
}

#[test]
fn e2e_neg_exp() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let exp = builder.add_op(ScalarOp::Exp, vec![ValRef::Local(x)], OpMode::Primal);
    let neg = builder.add_op(ScalarOp::Neg, vec![ValRef::Local(exp[0])], OpMode::Primal);
    let neg_key = builder.global_key(neg[0]).clone();
    builder.set_outputs(vec![neg[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[neg_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&0.0]);
    assert!((result[0] - (-1.0)).abs() < 1e-12);
}

#[test]
fn e2e_dup_and_add() {
    let mut builder = FragmentBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let dup = builder.add_op(ScalarOp::Dup, vec![ValRef::Local(x)], OpMode::Primal);
    let sum = builder.add_op(
        ScalarOp::Add,
        vec![ValRef::Local(dup[0]), ValRef::Local(dup[1])],
        OpMode::Primal,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let frag = Arc::new(builder.build());

    let view = resolve(vec![frag]);
    let graph = materialize_merge(&view, &[sum_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&7.0]);
    assert_eq!(result, vec![14.0]);
}
