use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::serializer::{Error, Serializer};

// TODO: make sure it's possible to check if dataref points to selected datastore
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DataRef {
    pub(crate) index: usize,
}

impl DataRef {
    pub fn data_section_offset(&self, node_count: usize) -> usize {
        node_count + 16 + self.index
    }
}

#[derive(Debug, Default)]
pub(crate) struct Datastore {
    store: Vec<u8>,
    // Hash of each stored value's bytes to its ref in `store`.
    // Values are serialized without pointers, so equal bytes mean equal values.
    refs: HashMap<u64, Vec<DataRef>>,
}

impl Datastore {
    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn insert<T: serde::Serialize>(&mut self, value: T) -> Result<DataRef, Error> {
        let mut bytes = Vec::new();
        value.serialize(&mut Serializer::new(&mut bytes))?;

        // check if this value has already been inserted
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let stored = self.refs.entry(hasher.finish()).or_default();
        if let Some(data_ref) = stored.iter().find(|data_ref| {
            self.store.get(data_ref.index..data_ref.index + bytes.len()) == Some(&bytes)
        }) {
            // value already in the store -> return existing ref
            return Ok(*data_ref);
        }

        // value not found -> insert
        let data_ref = DataRef {
            index: self.store.len(),
        };
        self.store.extend_from_slice(&bytes);
        stored.push(data_ref);
        Ok(data_ref)
    }

    pub fn serialized_data(&self) -> &[u8] {
        &self.store
    }
}
