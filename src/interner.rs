use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::traits::GraphOp;
use crate::types::GlobalValKey;

/// Interned identity for O(1) equality comparison of [`GlobalValKey`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ValKeyId(u32);

/// Maps [`GlobalValKey`] to [`ValKeyId`] for fast equality and deduplication.
pub struct KeyInterner<Op: GraphOp> {
    map: HashMap<GlobalValKey<Op>, ValKeyId>,
    keys: Vec<GlobalValKey<Op>>,
}

impl<Op: GraphOp> KeyInterner<Op> {
    /// Creates an empty interner.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            keys: Vec::new(),
        }
    }

    /// Interns a key, returning its unique id.
    pub fn intern(&mut self, key: GlobalValKey<Op>) -> ValKeyId {
        match self.map.entry(key.clone()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                assert!(
                    self.keys.len() <= u32::MAX as usize,
                    "too many interned value keys: {}",
                    self.keys.len()
                );
                let id = ValKeyId(self.keys.len() as u32);
                self.keys.push(key);
                entry.insert(id);
                id
            }
        }
    }

    /// Looks up the id for a key without interning it.
    pub fn get(&self, key: &GlobalValKey<Op>) -> Option<ValKeyId> {
        self.map.get(key).copied()
    }

    /// Retrieves the full key from an id.
    pub fn resolve(&self, id: ValKeyId) -> &GlobalValKey<Op> {
        let index = id.0 as usize;
        assert!(
            index < self.keys.len(),
            "unknown interned value key id {}",
            index
        );
        &self.keys[index]
    }
}

impl<Op: GraphOp> Default for KeyInterner<Op> {
    fn default() -> Self {
        Self::new()
    }
}
