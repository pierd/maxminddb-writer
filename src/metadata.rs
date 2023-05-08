use std::collections::HashMap;

pub(crate) const METADATA_START_MARKER: &[u8] = b"\xab\xcd\xefMaxMind.com";

#[derive(Clone, Copy, Debug)]
pub enum RecordSize {
    Small,
    Medium,
    Large,
}

impl RecordSize {
    pub fn choose(max_ptr_value: usize) -> Self {
        if max_ptr_value < 1 << 24 {
            RecordSize::Small
        } else if max_ptr_value < 1 << 28 {
            RecordSize::Medium
        } else {
            RecordSize::Large
        }
    }
}

impl serde::Serialize for RecordSize {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            RecordSize::Small => 24u16.serialize(serializer),
            RecordSize::Medium => 28u16.serialize(serializer),
            RecordSize::Large => 32u16.serialize(serializer),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum IpVersion {
    V4,
    V6,
}

impl serde::Serialize for IpVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            IpVersion::V4 => 4u16.serialize(serializer),
            IpVersion::V6 => 6u16.serialize(serializer),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Metadata {
    pub(crate) node_count: u32,
    pub(crate) record_size: RecordSize,
    pub ip_version: IpVersion,
    pub database_type: String,
    pub languages: Vec<String>,
    pub binary_format_major_version: u16,
    pub binary_format_minor_version: u16,
    pub build_epoch: u64,
    pub description: HashMap<String, String>,
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            node_count: 0,
            record_size: RecordSize::Small,
            ip_version: IpVersion::V4,
            database_type: String::new(),
            languages: Vec::new(),
            binary_format_major_version: 0,
            binary_format_minor_version: 0,
            build_epoch: 0,
            description: HashMap::new(),
        }
    }
}
