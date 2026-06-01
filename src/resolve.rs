use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::graph::Graph;
use crate::traits::GraphOperation;
use crate::types::{OperationRole, ValueKey, ValueRef};

/// Definition of a value as seen through the resolver.
#[derive(Clone, Debug, PartialEq)]
pub enum ValueDef<Op: GraphOperation> {
    Input {
        key: Op::InputKey,
    },
    Produced {
        operation: Op,
        /// Inputs resolved to global keys.
        input_keys: Vec<ValueKey<Op>>,
        role: OperationRole,
        output_slot: usize,
    },
}

/// Trait for resolving [`ValueKey`] to its definition.
pub trait Resolver<Op: GraphOperation> {
    fn resolve_value(&self, key: &ValueKey<Op>) -> Option<ValueDef<Op>>;
}

/// Logical traversal view over one or more graphs.
pub struct ResolvedView<Op: GraphOperation> {
    pub roots: Vec<Arc<Graph<Op>>>,
    resolver: Box<dyn Resolver<Op>>,
}

impl<Op: GraphOperation> ResolvedView<Op> {
    /// Resolves a global value key to its logical definition.
    pub fn resolve_value(&self, key: &ValueKey<Op>) -> Option<ValueDef<Op>> {
        self.resolver.resolve_value(key)
    }
}

struct HashMapResolver<Op: GraphOperation> {
    map: HashMap<ValueKey<Op>, ValueDef<Op>>,
}

impl<Op: GraphOperation> Resolver<Op> for HashMapResolver<Op> {
    fn resolve_value(&self, key: &ValueKey<Op>) -> Option<ValueDef<Op>> {
        self.map.get(key).cloned()
    }
}

/// Builds a logical lookup view over graphs and their parent chains.
pub fn resolve<Op: GraphOperation>(roots: Vec<Arc<Graph<Op>>>) -> ResolvedView<Op> {
    let mut map = HashMap::new();
    let mut visited = HashSet::new();

    for root in &roots {
        walk_graph(root, &mut map, &mut visited);
    }

    ResolvedView {
        roots,
        resolver: Box::new(HashMapResolver { map }),
    }
}

fn walk_graph<Op: GraphOperation>(
    graph: &Graph<Op>,
    map: &mut HashMap<ValueKey<Op>, ValueDef<Op>>,
    visited: &mut HashSet<*const Graph<Op>>,
) {
    let graph_ptr: *const Graph<Op> = graph;
    if !visited.insert(graph_ptr) {
        return;
    }

    for parent in graph.parents() {
        walk_graph(parent, map, visited);
    }

    for val in graph.values() {
        if map.contains_key(&val.key) {
            continue;
        }

        match val.producer {
            None => {
                let input_key = match &val.key {
                    ValueKey::Input(key) => key.clone(),
                    _ => panic!(
                        "graph input value must use ValueKey::Input, got {:?}",
                        val.key
                    ),
                };
                map.insert(val.key.clone(), ValueDef::Input { key: input_key });
            }
            Some((op_id, output_slot)) => {
                assert!(
                    op_id < graph.operations().len(),
                    "value references unknown producer op id {}",
                    op_id
                );
                let operation_node = &graph.operations()[op_id];
                let input_keys = operation_node
                    .inputs
                    .iter()
                    .map(|input| match input {
                        ValueRef::Local(local_id) => {
                            assert!(
                                *local_id < graph.values().len(),
                                "operation {:?} references unknown local value id {}",
                                operation_node.operation,
                                local_id
                            );
                            graph.values()[*local_id].key.clone()
                        }
                        ValueRef::External(key) => key.clone(),
                    })
                    .collect();

                map.insert(
                    val.key.clone(),
                    ValueDef::Produced {
                        operation: operation_node.operation.clone(),
                        input_keys,
                        role: operation_node.role.clone(),
                        output_slot,
                    },
                );
            }
        }
    }
}
