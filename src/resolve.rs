use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::fragment::Fragment;
use crate::traits::GraphOp;
use crate::types::{GlobalValKey, OpMode, ValRef};

/// Definition of a value as seen through the resolver.
#[derive(Clone, Debug, PartialEq)]
pub enum ValDef<Op: GraphOp> {
    Input {
        key: Op::InputKey,
    },
    Produced {
        op: Op,
        /// Inputs resolved to global keys.
        input_keys: Vec<GlobalValKey<Op>>,
        mode: OpMode,
        output_slot: usize,
    },
}

/// Trait for resolving [`GlobalValKey`] to its definition.
pub trait Resolver<Op: GraphOp> {
    fn resolve_val(&self, key: &GlobalValKey<Op>) -> Option<ValDef<Op>>;
}

/// Logical traversal view over one or more fragments.
pub struct ResolvedView<Op: GraphOp> {
    pub roots: Vec<Arc<Fragment<Op>>>,
    resolver: Box<dyn Resolver<Op>>,
}

impl<Op: GraphOp> ResolvedView<Op> {
    /// Resolves a global value key to its logical definition.
    pub fn resolve_val(&self, key: &GlobalValKey<Op>) -> Option<ValDef<Op>> {
        self.resolver.resolve_val(key)
    }
}

struct HashMapResolver<Op: GraphOp> {
    map: HashMap<GlobalValKey<Op>, ValDef<Op>>,
}

impl<Op: GraphOp> Resolver<Op> for HashMapResolver<Op> {
    fn resolve_val(&self, key: &GlobalValKey<Op>) -> Option<ValDef<Op>> {
        self.map.get(key).cloned()
    }
}

/// Builds a logical lookup view over fragments and their parent chains.
pub fn resolve<Op: GraphOp>(roots: Vec<Arc<Fragment<Op>>>) -> ResolvedView<Op> {
    let mut map = HashMap::new();
    let mut visited = HashSet::new();

    for root in &roots {
        walk_fragment(root, &mut map, &mut visited);
    }

    ResolvedView {
        roots,
        resolver: Box::new(HashMapResolver { map }),
    }
}

fn walk_fragment<Op: GraphOp>(
    fragment: &Fragment<Op>,
    map: &mut HashMap<GlobalValKey<Op>, ValDef<Op>>,
    visited: &mut HashSet<*const Fragment<Op>>,
) {
    let fragment_ptr: *const Fragment<Op> = fragment;
    if !visited.insert(fragment_ptr) {
        return;
    }

    for parent in fragment.parents() {
        walk_fragment(parent, map, visited);
    }

    for val in fragment.vals() {
        if map.contains_key(&val.key) {
            continue;
        }

        match val.producer {
            None => {
                let input_key = match &val.key {
                    GlobalValKey::Input(key) => key.clone(),
                    _ => panic!(
                        "fragment input value must use GlobalValKey::Input, got {:?}",
                        val.key
                    ),
                };
                map.insert(val.key.clone(), ValDef::Input { key: input_key });
            }
            Some((op_id, output_slot)) => {
                assert!(
                    op_id < fragment.ops().len(),
                    "value references unknown producer op id {}",
                    op_id
                );
                let op_node = &fragment.ops()[op_id];
                let input_keys = op_node
                    .inputs
                    .iter()
                    .map(|input| match input {
                        ValRef::Local(local_id) => {
                            assert!(
                                *local_id < fragment.vals().len(),
                                "operation {:?} references unknown local value id {}",
                                op_node.op,
                                local_id
                            );
                            fragment.vals()[*local_id].key.clone()
                        }
                        ValRef::External(key) => key.clone(),
                    })
                    .collect();

                map.insert(
                    val.key.clone(),
                    ValDef::Produced {
                        op: op_node.op.clone(),
                        input_keys,
                        mode: op_node.mode.clone(),
                        output_slot,
                    },
                );
            }
        }
    }
}
