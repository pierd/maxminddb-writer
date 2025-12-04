use paths::IntoBitPath;
use serde::Serialize;

pub(crate) mod data;
pub mod metadata;
pub(crate) mod node;
pub mod paths;
pub(crate) mod serializer;

#[derive(Debug, Default)]
pub struct Database {
    nodes: node::NodeTree,
    data: data::Datastore,
    pub metadata: metadata::Metadata,
}

impl Database {
    fn update_size(&mut self) {
        // make sure we have correct node count
        let node_count = self.nodes.len();
        self.metadata.node_count = node_count.try_into().unwrap();

        // update record size if needed
        let data_size = self.data.len();
        let max_ptr_value = node_count + data_size + 16;
        self.metadata.record_size = metadata::RecordSize::choose(max_ptr_value);
    }

    pub fn insert_value<T: serde::Serialize>(
        &mut self,
        value: T,
    ) -> Result<data::DataRef, serializer::Error> {
        let result = self.data.insert(value);
        self.update_size();
        result
    }

    pub fn insert_node(&mut self, path: impl IntoBitPath, data: data::DataRef) {
        self.nodes.insert(path, data);
        self.update_size();
    }

    pub fn write_to<W: std::io::Write>(&self, writer: W) -> Result<W, serializer::Error> {
        // write node tree
        let mut writer = self.nodes.write_to(writer, self.metadata.record_size)?;
        // write data section separator
        writer.write_all(&[0u8; 16])?;
        // write data section
        writer.write_all(self.data.serialized_data())?;
        // write metadata marker
        writer.write_all(metadata::METADATA_START_MARKER)?;
        // serialize metadata
        let mut serializer = serializer::Serializer::new(writer);
        self.metadata.serialize(&mut serializer)?;
        // all done
        Ok(serializer.into_inner())
    }

    #[cfg(test)]
    pub(crate) fn to_vec(&self) -> Result<Vec<u8>, serializer::Error> {
        let mut result = Vec::new();
        self.write_to(&mut result)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::metadata::RecordSize;
    use crate::paths::IpAddrWithMask;

    use super::*;

    fn seed_simple_db() -> Database {
        let mut db = Database::default();
        let data_42 = db.insert_value(42u32).unwrap();
        let data_foo = db.insert_value("foo".to_string()).unwrap();
        db.insert_node("0.0.0.0/16".parse::<IpAddrWithMask>().unwrap(), data_42);
        db.insert_node("1.0.0.0/16".parse::<IpAddrWithMask>().unwrap(), data_foo);

        db
    }

    #[test]
    fn test_simple() {
        let db = seed_simple_db();
        let raw_db = db.to_vec().unwrap();

        let reader = maxminddb::Reader::from_source(&raw_db).unwrap();
        let expected_data_42: u32 = reader.lookup([0, 0, 0, 0].into()).unwrap();
        let expected_data_foo: &str = reader.lookup([1, 0, 0, 0].into()).unwrap();

        assert_eq!(expected_data_42, 42);
        assert_eq!(expected_data_foo, "foo");
    }

    #[test]
    fn test_small_record_write() {
        let mut db = seed_simple_db();

        // Force small record size to test the small node writing
        db.metadata.record_size = RecordSize::Small;
        let raw_db = db.to_vec().unwrap();

        let reader = maxminddb::Reader::from_source(&raw_db).unwrap();
        let expected_data: u32 = reader.lookup([0, 0, 0, 0].into()).unwrap();

        assert_eq!(expected_data, 42);
        assert!(matches!(db.metadata.record_size, RecordSize::Small));
    }

    #[test]
    fn test_medium_record_write() {
        let mut db = seed_simple_db();

        // Force medium record size to test the medium node writing
        db.metadata.record_size = RecordSize::Medium;
        let raw_db = db.to_vec().unwrap();

        let reader = maxminddb::Reader::from_source(&raw_db).unwrap();
        let expected_data: u32 = reader.lookup([0, 0, 0, 0].into()).unwrap();

        assert_eq!(expected_data, 42);
        assert!(matches!(db.metadata.record_size, RecordSize::Medium));
    }

    #[test]
    fn test_large_record_write() {
        let mut db = seed_simple_db();

        // Force large record size to test the large node writing
        db.metadata.record_size = RecordSize::Large;
        let raw_db = db.to_vec().unwrap();

        let reader = maxminddb::Reader::from_source(&raw_db).unwrap();
        let expected_data: u32 = reader.lookup([0, 0, 0, 0].into()).unwrap();

        assert_eq!(expected_data, 42);
        assert!(matches!(db.metadata.record_size, RecordSize::Large));
    }
}
