use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::traits::GraphOperation;
use crate::types::ValueKey;

/// Interned identity for O(1) equality comparison of [`ValueKey`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ValueKeyId(u32);

/// Maps [`ValueKey`] to [`ValueKeyId`] for fast equality and deduplication.
pub struct ValueKeyInterner<Op: GraphOperation> {
    map: HashMap<ValueKey<Op>, ValueKeyId>,
    keys: Vec<ValueKey<Op>>,
}

impl<Op: GraphOperation> ValueKeyInterner<Op> {
    /// Creates an empty interner.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            keys: Vec::new(),
        }
    }

    /// Interns a key, returning its unique id.
    pub fn intern(&mut self, key: ValueKey<Op>) -> ValueKeyId {
        match self.map.entry(key.clone()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                assert!(
                    self.keys.len() <= u32::MAX as usize,
                    "too many interned value keys: {}",
                    self.keys.len()
                );
                let id = ValueKeyId(self.keys.len() as u32);
                self.keys.push(key);
                entry.insert(id);
                id
            }
        }
    }

    /// Looks up the id for a key without interning it.
    pub fn get(&self, key: &ValueKey<Op>) -> Option<ValueKeyId> {
        self.map.get(key).copied()
    }

    /// Retrieves the full key from an id.
    pub fn resolve(&self, id: ValueKeyId) -> &ValueKey<Op> {
        let index = id.0 as usize;
        assert!(
            index < self.keys.len(),
            "unknown interned value key id {}",
            index
        );
        &self.keys[index]
    }
}

impl<Op: GraphOperation> Default for ValueKeyInterner<Op> {
    fn default() -> Self {
        Self::new()
    }
}
