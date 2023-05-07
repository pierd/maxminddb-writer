use std::collections::HashMap;

pub const METADATA_START_MARKER: &[u8] = b"\xab\xcd\xefMaxMind.com";

#[derive(Clone, Debug, serde::Serialize)]
pub struct DatabaseMetadata {
    node_count: u32,
    record_size: u16,
    ip_version: u16,
    database_type: String,
    languages: Vec<String>,
    binary_format_major_version: u16,
    binary_format_minor_version: u16,
    build_epoch: u64,
    description: HashMap<String, String>,
}

impl DatabaseMetadata {
    pub fn with_node_count(mut self, node_count: u32) -> Self {
        self.node_count = node_count;
        self
    }

    pub fn with_ipv6(mut self) -> Self {
        self.ip_version = 6;
        self
    }
}

impl Default for DatabaseMetadata {
    fn default() -> Self {
        DatabaseMetadata {
            node_count: 0,
            record_size: 24,
            ip_version: 4,
            database_type: String::new(),
            languages: Vec::new(),
            binary_format_major_version: 0,
            binary_format_minor_version: 0,
            build_epoch: 0,
            description: HashMap::new(),
        }
    }
}
