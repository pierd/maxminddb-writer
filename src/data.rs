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
}

impl Datastore {
    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn insert<T: serde::Serialize>(&mut self, value: T) -> Result<DataRef, Error> {
        let data_ref = DataRef {
            index: self.store.len(),
        };
        value
            .serialize(&mut Serializer::new(&mut self.store))
            .map(|_| data_ref)
    }

    pub fn serialized_data(&self) -> &[u8] {
        &self.store
    }
}
