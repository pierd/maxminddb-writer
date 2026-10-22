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
    // Hash of each stored value's bytes to its (offset, length) in `store`.
    // Values are serialized without pointers, so equal bytes mean equal values.
    refs: HashMap<u64, Vec<(usize, usize)>>,
}

impl Datastore {
    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn insert<T: serde::Serialize>(&mut self, value: T) -> Result<DataRef, Error> {
        let mut bytes = Vec::new();
        value.serialize(&mut Serializer::new(&mut bytes))?;

        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let stored = self.refs.entry(hasher.finish()).or_default();
        if let Some(&(index, _)) = stored
            .iter()
            .find(|&&(index, len)| self.store[index..index + len] == bytes[..])
        {
            return Ok(DataRef { index });
        }

        let index = self.store.len();
        self.store.extend_from_slice(&bytes);
        stored.push((index, bytes.len()));
        Ok(DataRef { index })
    }

    pub fn serialized_data(&self) -> &[u8] {
        &self.store
    }
}
