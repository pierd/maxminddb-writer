use std::ops::{Index, IndexMut};

use crate::{data::DataRef, metadata::RecordSize, paths::IntoBitPath};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Target {
    Node(NodeRef),
    Data(DataRef),
}

impl Target {
    fn to_ptr(self, node_count: usize) -> usize {
        match self {
            Target::Node(node) => node.index,
            Target::Data(data) => data.data_section_offset(node_count),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Node([Option<Target>; 2]);

impl Node {
    fn write_to(
        &self,
        writer: &mut impl std::io::Write,
        record_size: RecordSize,
        node_count: usize,
    ) -> Result<(), std::io::Error> {
        let ptrs = [
            self.0[0]
                .map(|t| t.to_ptr(node_count))
                .unwrap_or(node_count),
            self.0[1]
                .map(|t| t.to_ptr(node_count))
                .unwrap_or(node_count),
        ];
        match record_size {
            // 24 bits/ptr -> 6 bytes
            RecordSize::Small => writer.write_all(&[
                (ptrs[0] >> 16) as u8,
                (ptrs[0] >> 8) as u8,
                ptrs[0] as u8,
                (ptrs[1] >> 16) as u8,
                (ptrs[1] >> 8) as u8,
                ptrs[1] as u8,
            ]),
            // 28 bits/ptr -> 7 bytes
            RecordSize::Medium => writer.write_all(&[
                (ptrs[0] >> 20) as u8,
                (ptrs[0] >> 12) as u8,
                (ptrs[0] >> 4) as u8,
                (ptrs[0] << 4) as u8 | (ptrs[1] >> 24) as u8,
                (ptrs[1] >> 16) as u8,
                (ptrs[1] >> 8) as u8,
                ptrs[1] as u8,
            ]),
            // 32 bits/ptr -> 8 bytes
            RecordSize::Large => writer.write_all(&[
                (ptrs[0] >> 24) as u8,
                (ptrs[0] >> 16) as u8,
                (ptrs[0] >> 8) as u8,
                ptrs[0] as u8,
                (ptrs[1] >> 24) as u8,
                (ptrs[1] >> 16) as u8,
                (ptrs[1] >> 8) as u8,
                ptrs[1] as u8,
            ]),
        }
    }
}

impl Index<bool> for Node {
    type Output = Option<Target>;

    fn index(&self, index: bool) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<bool> for Node {
    fn index_mut(&mut self, index: bool) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NodeRef {
    index: usize,
}

#[derive(Debug)]
pub struct NodeTree {
    nodes: Vec<Node>,
}

impl NodeTree {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn insert(&mut self, path: impl IntoBitPath, data: DataRef) {
        let mut path = path.into_bit_path();
        let mut index = 0;
        let Some(mut last_bit) = path.next() else {
            // empty path doesn't insert anything
            return;
        };

        for bit in path {
            let target = self.nodes[index][last_bit];
            match target {
                // node points to another -> follow the path
                Some(Target::Node(NodeRef { index: new_index })) => {
                    index = new_index;
                }
                // node points to data (or is empty) -> split the node
                Some(Target::Data(_)) | None => {
                    let old_index = index;
                    index = self.nodes.len();
                    self.nodes.push(Node([target, target]));
                    self.nodes[old_index][last_bit] = Some(Target::Node(NodeRef { index }));
                }
            }
            last_bit = bit;
        }

        self.nodes[index][last_bit] = Some(Target::Data(data));
    }

    pub fn write_to<W: std::io::Write>(
        &self,
        mut writer: W,
        record_size: RecordSize,
    ) -> Result<W, std::io::Error> {
        for node in &self.nodes {
            node.write_to(&mut writer, record_size, self.len())?;
        }
        Ok(writer)
    }
}

impl Default for NodeTree {
    fn default() -> Self {
        Self {
            nodes: vec![Node::default()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_to_empty() {
        let mut tree = NodeTree::default();
        assert_eq!(tree.nodes.len(), 1);
        tree.insert([false].into_iter(), DataRef { index: 0 });
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(
            tree.nodes[0][false],
            Some(Target::Data(DataRef { index: 0 }))
        );
        assert_eq!(tree.nodes[0][true], None);

        tree.insert([true].into_iter(), DataRef { index: 1 });
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(
            tree.nodes[0][false],
            Some(Target::Data(DataRef { index: 0 }))
        );
        assert_eq!(
            tree.nodes[0][true],
            Some(Target::Data(DataRef { index: 1 }))
        );
    }
}
