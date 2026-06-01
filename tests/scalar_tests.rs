mod common;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use common::ScalarOp;
use computegraph::compile::compile;
use computegraph::graph::GraphBuilder;
use computegraph::interner::ValueKeyInterner;
use computegraph::materialize::materialize_merge;
use computegraph::resolve::{resolve, ValueDef};
use computegraph::{
    EvaluableGraphOperation, GraphOperation, OperationKey, OperationRole, ValueKey, ValueRef,
};

// === ScalarOp smoke tests ===

#[test]
fn scalar_op_eval_add() {
    let op = ScalarOp::Add;
    assert_eq!(op.input_count(), 2);
    assert_eq!(op.output_count(), 1);
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
    assert_eq!(op.output_count(), 2);
    let result = op.eval(&mut (), &[&5.0]);
    assert_eq!(result, vec![5.0, 5.0]);
}

// === ValueKeyInterner tests ===

#[test]
fn interner_intern_and_resolve() {
    let mut interner = ValueKeyInterner::<ScalarOp>::new();
    let key = ValueKey::Input("x".to_string());
    let id = interner.intern(key.clone());
    assert_eq!(interner.resolve(id), &key);
}

#[test]
fn interner_deduplicates() {
    let mut interner = ValueKeyInterner::<ScalarOp>::new();
    let key = ValueKey::Input("x".to_string());
    let id1 = interner.intern(key.clone());
    let id2 = interner.intern(key);
    assert_eq!(id1, id2);
}

#[test]
fn interner_distinct_keys_get_distinct_ids() {
    let mut interner = ValueKeyInterner::<ScalarOp>::new();
    let id_x = interner.intern(ValueKey::Input("x".to_string()));
    let id_y = interner.intern(ValueKey::Input("y".to_string()));
    assert_ne!(id_x, id_y);
}

#[test]
fn interner_get_returns_none_for_unknown() {
    let interner = ValueKeyInterner::<ScalarOp>::new();
    let key = ValueKey::Input("x".to_string());
    assert_eq!(interner.get(&key), None);
}

#[test]
fn interner_derived_key() {
    let mut interner = ValueKeyInterner::<ScalarOp>::new();
    let key = ValueKey::<ScalarOp>::Derived {
        operation: Arc::new(OperationKey::new(
            ScalarOp::Add,
            vec![
                ValueKey::Input("x".to_string()),
                ValueKey::Input("y".to_string()),
            ],
            OperationRole::Primary,
        )),
        output_slot: 0,
    };
    let id = interner.intern(key.clone());
    assert_eq!(interner.resolve(id), &key);
    assert_eq!(interner.get(&key), Some(id));
}

#[test]
fn derived_keys_with_distinct_op_arcs_are_structurally_equal() {
    let inputs = vec![
        ValueKey::Input("x".to_string()),
        ValueKey::Input("y".to_string()),
    ];
    let lhs = ValueKey::<ScalarOp>::Derived {
        operation: Arc::new(OperationKey::new(
            ScalarOp::Add,
            inputs.clone(),
            OperationRole::Primary,
        )),
        output_slot: 0,
    };
    let rhs = ValueKey::<ScalarOp>::Derived {
        operation: Arc::new(OperationKey::new(
            ScalarOp::Add,
            inputs,
            OperationRole::Primary,
        )),
        output_slot: 0,
    };

    assert_eq!(lhs, rhs);
    assert_eq!(hash_key(&lhs), hash_key(&rhs));
}

fn hash_key(key: &ValueKey<ScalarOp>) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

// === Graph tests ===

#[test]
fn graph_builder_single_input() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    assert_eq!(x, 0);
    builder.set_outputs(vec![x]);
    let graph = builder.build();
    assert_eq!(graph.inputs().len(), 1);
    assert_eq!(graph.outputs().len(), 1);
    assert_eq!(graph.values()[x].key, ValueKey::Input("x".to_string()));
    assert!(graph.values()[x].producer.is_none());
}

#[test]
fn graph_builder_add_operation() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let outputs = builder.add_operation(
        ScalarOp::Add,
        vec![ValueRef::Local(x), ValueRef::Local(y)],
        OperationRole::Primary,
    );
    assert_eq!(outputs.len(), 1);
    let sum_id = outputs[0];
    builder.set_outputs(vec![sum_id]);
    let graph = builder.build();

    assert_eq!(graph.operations().len(), 1);
    assert_eq!(graph.operations()[0].operation, ScalarOp::Add);
    assert!(graph.values()[sum_id].producer.is_some());

    // Verify ValueKey structure
    let expected_key = ValueKey::Derived {
        operation: Arc::new(OperationKey::new(
            ScalarOp::Add,
            vec![
                ValueKey::Input("x".to_string()),
                ValueKey::Input("y".to_string()),
            ],
            OperationRole::Primary,
        )),
        output_slot: 0,
    };
    assert_eq!(graph.values()[sum_id].key, expected_key);
}

#[test]
fn graph_builder_chain() {
    // Build: Exp(Mul(x, a))
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul_out = builder.add_operation(
        ScalarOp::Mul,
        vec![ValueRef::Local(x), ValueRef::Local(a)],
        OperationRole::Primary,
    );
    let exp_out = builder.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::Local(mul_out[0])],
        OperationRole::Primary,
    );
    builder.set_outputs(vec![exp_out[0]]);
    let graph = builder.build();

    assert_eq!(graph.operations().len(), 2);
    assert_eq!(graph.values().len(), 4); // x, a, mul_out, exp_out
}

#[test]
fn graph_builder_dup_two_outputs() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let dup_outs = builder.add_operation(
        ScalarOp::Dup,
        vec![ValueRef::Local(x)],
        OperationRole::Primary,
    );
    assert_eq!(dup_outs.len(), 2);
    builder.set_outputs(dup_outs.clone());
    let graph = builder.build();

    assert_eq!(graph.outputs().len(), 2);
    // Both outputs should be Derived with different output_slot
    let key0 = &graph.values()[dup_outs[0]].key;
    let key1 = &graph.values()[dup_outs[1]].key;
    match (key0, key1) {
        (
            ValueKey::Derived {
                output_slot: s0, ..
            },
            ValueKey::Derived {
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
fn resolve_single_graph() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let sum = builder.add_operation(
        ScalarOp::Add,
        vec![ValueRef::Local(x), ValueRef::Local(y)],
        OperationRole::Primary,
    );
    builder.set_outputs(vec![sum[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph.clone()]);

    // Input keys should resolve
    let x_key = ValueKey::Input("x".to_string());
    match view.resolve_value(&x_key).unwrap() {
        ValueDef::Input { key } => assert_eq!(key, "x"),
        _ => panic!("expected Input"),
    }

    // Derived key should resolve
    let sum_key = &graph.values()[sum[0]].key;
    match view.resolve_value(sum_key).unwrap() {
        ValueDef::Produced {
            operation,
            input_keys,
            role,
            output_slot,
        } => {
            assert_eq!(operation, ScalarOp::Add);
            assert_eq!(input_keys.len(), 2);
            assert_eq!(role, OperationRole::Primary);
            assert_eq!(output_slot, 0);
        }
        _ => panic!("expected Produced"),
    }
}

#[test]
fn resolve_external_ref_across_graphs() {
    // Graph F0: x, a, mul = Mul(x, a)
    let mut b0 = GraphBuilder::<ScalarOp>::new();
    let x = b0.add_input("x".to_string());
    let a = b0.add_input("a".to_string());
    let mul = b0.add_operation(
        ScalarOp::Mul,
        vec![ValueRef::Local(x), ValueRef::Local(a)],
        OperationRole::Primary,
    );
    let mul_key = b0.global_key(mul[0]).clone();
    b0.set_outputs(vec![mul[0]]);
    let f0 = Arc::new(b0.build());

    // Graph F1: references F0's mul output via External, applies Exp
    let mut b1 = GraphBuilder::<ScalarOp>::new();
    b1.add_parent(f0.clone());
    let exp = b1.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::External(mul_key.clone())],
        OperationRole::Primary,
    );
    b1.set_outputs(vec![exp[0]]);
    let f1 = Arc::new(b1.build());

    let view = resolve(vec![f0, f1.clone()]);

    // mul_key should be resolvable
    assert!(view.resolve_value(&mul_key).is_some());

    // exp output should be resolvable
    let exp_key = &f1.values()[exp[0]].key;
    match view.resolve_value(exp_key).unwrap() {
        ValueDef::Produced {
            operation,
            input_keys,
            ..
        } => {
            assert_eq!(operation, ScalarOp::Exp);
            assert_eq!(input_keys.len(), 1);
            assert_eq!(input_keys[0], mul_key);
        }
        _ => panic!("expected Produced"),
    }
}

#[test]
fn resolve_unknown_key_returns_none() {
    let builder = GraphBuilder::<ScalarOp>::new();
    let graph = Arc::new(builder.build());
    let view = resolve(vec![graph]);
    let unknown = ValueKey::Input("unknown".to_string());
    assert!(view.resolve_value(&unknown).is_none());
}

// === Materialize tests ===

#[test]
fn materialize_single_op() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let sum = builder.add_operation(
        ScalarOp::Add,
        vec![ValueRef::Local(x), ValueRef::Local(y)],
        OperationRole::Primary,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[sum_key]);

    assert_eq!(graph.operations.len(), 1);
    assert_eq!(graph.operations[0].operation, ScalarOp::Add);
    assert_eq!(graph.values.len(), 3);
    assert_eq!(graph.inputs.len(), 2);
    assert_eq!(graph.outputs.len(), 1);
}

#[test]
fn materialize_chain() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul = builder.add_operation(
        ScalarOp::Mul,
        vec![ValueRef::Local(x), ValueRef::Local(a)],
        OperationRole::Primary,
    );
    let exp = builder.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::Local(mul[0])],
        OperationRole::Primary,
    );
    let exp_key = builder.global_key(exp[0]).clone();
    builder.set_outputs(vec![exp[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[exp_key]);

    assert_eq!(graph.operations.len(), 2);
    assert_eq!(graph.values.len(), 4);
    assert_eq!(graph.operations[0].operation, ScalarOp::Mul);
    assert_eq!(graph.operations[1].operation, ScalarOp::Exp);
}

#[test]
fn materialize_cse_deduplicates() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let sum = builder.add_operation(
        ScalarOp::Add,
        vec![ValueRef::Local(x), ValueRef::Local(x)],
        OperationRole::Primary,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[sum_key]);

    assert_eq!(graph.values.len(), 2);
    assert_eq!(graph.operations.len(), 1);
    assert_eq!(graph.operations[0].inputs[0], graph.operations[0].inputs[1]);
}

#[test]
fn materialize_across_graphs() {
    let mut b0 = GraphBuilder::<ScalarOp>::new();
    let x = b0.add_input("x".to_string());
    let a = b0.add_input("a".to_string());
    let mul = b0.add_operation(
        ScalarOp::Mul,
        vec![ValueRef::Local(x), ValueRef::Local(a)],
        OperationRole::Primary,
    );
    let mul_key = b0.global_key(mul[0]).clone();
    b0.set_outputs(vec![mul[0]]);
    let f0 = Arc::new(b0.build());

    let mut b1 = GraphBuilder::<ScalarOp>::new();
    b1.add_parent(f0.clone());
    let exp = b1.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::External(mul_key)],
        OperationRole::Primary,
    );
    let exp_key = b1.global_key(exp[0]).clone();
    b1.set_outputs(vec![exp[0]]);
    let f1 = Arc::new(b1.build());

    let view = resolve(vec![f0, f1]);
    let graph = materialize_merge(&view, &[exp_key]);

    assert_eq!(graph.operations.len(), 2);
    assert_eq!(graph.values.len(), 4);
}

// === Compile + Eval tests ===

#[test]
fn compile_and_eval_add() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let sum = builder.add_operation(
        ScalarOp::Add,
        vec![ValueRef::Local(x), ValueRef::Local(y)],
        OperationRole::Primary,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
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
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul = builder.add_operation(
        ScalarOp::Mul,
        vec![ValueRef::Local(x), ValueRef::Local(a)],
        OperationRole::Primary,
    );
    let exp = builder.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::Local(mul[0])],
        OperationRole::Primary,
    );
    let exp_key = builder.global_key(exp[0]).clone();
    builder.set_outputs(vec![exp[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[exp_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&1.0, &2.0]);
    assert!((result[0] - 2.0_f64.exp()).abs() < 1e-12);
}

#[test]
fn compile_and_eval_reuse() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let y = builder.add_input("y".to_string());
    let sum = builder.add_operation(
        ScalarOp::Add,
        vec![ValueRef::Local(x), ValueRef::Local(y)],
        OperationRole::Primary,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[sum_key]);
    let prog = compile(&graph);

    assert_eq!(prog.eval(&mut (), &[&1.0, &2.0]), vec![3.0]);
    assert_eq!(prog.eval(&mut (), &[&10.0, &20.0]), vec![30.0]);
}

#[test]
fn compile_and_eval_multi_output() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul = builder.add_operation(
        ScalarOp::Mul,
        vec![ValueRef::Local(x), ValueRef::Local(a)],
        OperationRole::Primary,
    );
    let exp = builder.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::Local(mul[0])],
        OperationRole::Primary,
    );
    let mul_key = builder.global_key(mul[0]).clone();
    let exp_key = builder.global_key(exp[0]).clone();
    builder.set_outputs(vec![mul[0], exp[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[mul_key, exp_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&1.0, &2.0]);
    assert_eq!(result.len(), 2);
    assert!((result[0] - 2.0).abs() < 1e-12);
    assert!((result[1] - 2.0_f64.exp()).abs() < 1e-12);
}

// === End-to-end integration tests ===

#[test]
fn e2e_exp_ax_single_graph() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let a = builder.add_input("a".to_string());
    let mul = builder.add_operation(
        ScalarOp::Mul,
        vec![ValueRef::Local(x), ValueRef::Local(a)],
        OperationRole::Primary,
    );
    let exp = builder.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::Local(mul[0])],
        OperationRole::Primary,
    );
    let exp_key = builder.global_key(exp[0]).clone();
    builder.set_outputs(vec![exp[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[exp_key]);
    let prog = compile(&graph);
    let result = prog.eval(&mut (), &[&2.0, &3.0]);

    assert!((result[0] - 6.0_f64.exp()).abs() < 1e-12);
}

#[test]
fn e2e_exp_ax_multi_graph() {
    let mut b0 = GraphBuilder::<ScalarOp>::new();
    let x = b0.add_input("x".to_string());
    let a = b0.add_input("a".to_string());
    let mul = b0.add_operation(
        ScalarOp::Mul,
        vec![ValueRef::Local(x), ValueRef::Local(a)],
        OperationRole::Primary,
    );
    let mul_key = b0.global_key(mul[0]).clone();
    b0.set_outputs(vec![mul[0]]);
    let f0 = Arc::new(b0.build());

    let mut b1 = GraphBuilder::<ScalarOp>::new();
    b1.add_parent(f0.clone());
    let exp = b1.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::External(mul_key)],
        OperationRole::Primary,
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
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let sum = builder.add_operation(
        ScalarOp::Add,
        vec![ValueRef::Local(x), ValueRef::Local(x)],
        OperationRole::Primary,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[sum_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&5.0]);
    assert_eq!(result, vec![10.0]);
}

#[test]
fn e2e_neg_exp() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let exp = builder.add_operation(
        ScalarOp::Exp,
        vec![ValueRef::Local(x)],
        OperationRole::Primary,
    );
    let neg = builder.add_operation(
        ScalarOp::Neg,
        vec![ValueRef::Local(exp[0])],
        OperationRole::Primary,
    );
    let neg_key = builder.global_key(neg[0]).clone();
    builder.set_outputs(vec![neg[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[neg_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&0.0]);
    assert!((result[0] - (-1.0)).abs() < 1e-12);
}

#[test]
fn e2e_dup_and_add() {
    let mut builder = GraphBuilder::<ScalarOp>::new();
    let x = builder.add_input("x".to_string());
    let dup = builder.add_operation(
        ScalarOp::Dup,
        vec![ValueRef::Local(x)],
        OperationRole::Primary,
    );
    let sum = builder.add_operation(
        ScalarOp::Add,
        vec![ValueRef::Local(dup[0]), ValueRef::Local(dup[1])],
        OperationRole::Primary,
    );
    let sum_key = builder.global_key(sum[0]).clone();
    builder.set_outputs(vec![sum[0]]);
    let graph = Arc::new(builder.build());

    let view = resolve(vec![graph]);
    let graph = materialize_merge(&view, &[sum_key]);
    let prog = compile(&graph);

    let result = prog.eval(&mut (), &[&7.0]);
    assert_eq!(result, vec![14.0]);
}
