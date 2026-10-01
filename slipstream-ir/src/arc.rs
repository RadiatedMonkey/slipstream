use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::cursor::RefCursor;
use slipstream_shared::error::{
    CorruptionError, IncorrectFormat, SlipstreamError, SlipstreamResult,
};

use crate::brres::{self, BRRES_MAGIC};
use crate::encoding::{ReadArrayExt, ReadStringExt};
use crate::node::arena::{IrArena, IrNodeDescriptor, IrNodeKey};
use crate::node::node::{ContentSlot, IrNodeType};
use crate::visitor::{Visitable, Visitor};

/// Magic of an ARC file.
pub const ARC_MAGIC: [u8; 4] = [0x55, 0xAA, 0x38, 0x2D];

/// See [`Custom Mario Kart Wiiki`](https://mkwiiki.org/wiki/ARC_(File_Format)) for more info.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Header {
    /// Offset to the first node in the archive.
    pub node_offset: i32,
    /// Size of all nodes including the string table.
    pub size: i32,
    /// File offset of data.
    pub file_offset: i32,
    /// Reserved.
    pub reserved: [i32; 4],
}

impl Header {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let magic = reader.read_u8_array::<4>()?;
        if magic != ARC_MAGIC {
            return Err(IncorrectFormat {
                expected_magic: ARC_MAGIC.to_vec(),
                found_magic: magic.to_vec(),
                location: Some(reader.position()),
            }
            .into());
        }

        let node_offset = reader.read_i32::<BigEndian>()?;
        let size = reader.read_i32::<BigEndian>()?;
        let file_offset = reader.read_i32::<BigEndian>()?;
        let reserved = reader.read_i32_array::<4, BigEndian>()?;

        Ok(Self {
            node_offset,
            size,
            file_offset,
            reserved,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum NodeType {
    File,
    Directory,
}

impl NodeType {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let b = reader.read_u8()?;
        Self::try_from(b)
    }
}

impl TryFrom<u8> for NodeType {
    type Error = SlipstreamError;

    fn try_from(value: u8) -> SlipstreamResult<Self> {
        Ok(match value {
            0 => NodeType::File,
            1 => NodeType::Directory,
            v => {
                return Err(CorruptionError {
                    reason: format!(
                        "arc node type is expected to be either 0 (file) or 1 (directory), got {v}"
                    ),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

/// Exact size of a single ARC node.
const ARC_NODE_SIZE: usize = 0x0c;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeContent {
    File { data: RefCursor<[u8]> },
    Directory { parent: u32, skip_node: u32 },
}

impl NodeContent {
    pub fn is_directory(&self) -> bool {
        matches!(self, Self::Directory { .. })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    pub data: NodeContent,
}

impl Node {
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        string_pool: &mut RefCursor<[u8]>,
    ) -> SlipstreamResult<Self> {
        let ty = NodeType::deserialize(reader)?;
        let name_offset = reader.read_u24::<BigEndian>()?;
        let data1 = reader.read_u32::<BigEndian>()?;
        let data2 = reader.read_u32::<BigEndian>()?;

        string_pool.set_position(name_offset as u64);

        let name = string_pool.read_null_string::<BigEndian>()?;

        tracing::trace!("Discovered node `{name}`");

        let data = match ty {
            NodeType::Directory => NodeContent::Directory {
                parent: data1,
                skip_node: data2,
            },
            NodeType::File => {
                let data_start = data1;

                let mut data = reader.clone();
                data.set_position(data_start as u64);
                data.set_tail();

                NodeContent::File { data }
            }
        };

        Ok(Self { name, data })
    }
}

pub struct UnknownFile {
    pub reader: RefCursor<[u8]>,
}

impl Visitable for UnknownFile {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_unknown(self)
    }

    fn accept_mut(&mut self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_unknown_mut(self)
    }
}

fn parse_leaf_node(
    reader: &mut RefCursor<[u8]>,
    parent_id: IrNodeKey,
    arena: &IrArena,
    label: String,
) -> SlipstreamResult<IrNodeKey> {
    let magic: [u8; 4] = reader.read_u8_array()?;
    reader.set_position(reader.position() - 4);

    match magic {
        ARC_MAGIC => deserialize(reader, Some(parent_id), arena, label),
        BRRES_MAGIC => brres::deserialize(reader, Some(parent_id), arena, label),
        _ => {
            let key = arena.insert(IrNodeDescriptor {
                label,
                ty: IrNodeType::Unknown,
                parent: Some(parent_id),
                children: Vec::new(),
                contents: ContentSlot::eager(Box::new(UnknownFile {
                    reader: reader.clone(),
                })),
            });

            Ok(key)
        }
    }
}

pub struct ArcDirectory {
    pub uncompressed_size: i32,
}

impl Visitable for ArcDirectory {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_arc(self)
    }

    fn accept_mut(&mut self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_arc_mut(self)
    }
}

/// Constructs a tree of the directories in an ARC file.
///
/// ARC files store their nodes in a linear list, this function converts it into
/// a tree by resolving references.
#[tracing::instrument(skip_all, fields(label))]
fn construct_directory_tree(
    node_list: &mut [Node],
    parent: Option<IrNodeKey>,
    arena: &IrArena,
    label: String,
    uncompressed_size: i32,
    cursor: &mut usize,
) -> SlipstreamResult<IrNodeKey> {
    let &NodeContent::Directory { skip_node, .. } = &node_list[*cursor].data else {
        return Err(CorruptionError {
            reason: "expected directory at root, found file instead".to_owned(),
            ..Default::default()
        }
        .into());
    };

    *cursor += 1;

    let key = arena.reserve_key();

    let mut children = Vec::new();
    while *cursor < skip_node as usize && *cursor < node_list.len() {
        let curr_node = &mut node_list[*cursor];

        let name = std::mem::take(&mut curr_node.name);
        match &mut curr_node.data {
            NodeContent::Directory { .. } => {
                let child = construct_directory_tree(
                    node_list,
                    Some(key),
                    arena,
                    name,
                    uncompressed_size,
                    cursor,
                )?;
                children.push(child);
            }
            NodeContent::File { data } => {
                let sections = parse_leaf_node(data, key, arena, name)?;
                children.push(sections);

                *cursor += 1;
            }
        }
    }

    arena.insert_at(
        key,
        IrNodeDescriptor {
            label,
            ty: IrNodeType::ArcDirectory {
                empty: children.is_empty(),
            },
            parent,
            children,
            contents: ContentSlot::eager(Box::new(ArcDirectory { uncompressed_size })),
        },
    );

    Ok(key)
}

pub fn deserialize(
    reader: &mut RefCursor<[u8]>,
    parent_id: Option<IrNodeKey>,
    arena: &IrArena,
    name: String,
) -> SlipstreamResult<IrNodeKey> {
    tracing::trace!("Parsing ARC file `{name}`");

    let header = Header::deserialize(reader)?;

    let ty = NodeType::deserialize(reader)?;
    if ty != NodeType::Directory {
        return Err(CorruptionError {
            reason: String::from("expected directory at root, found file"),
            ..Default::default()
        }
        .into());
    }

    let _offset = reader.read_u24::<BigEndian>()?;
    let _data1 = reader.read_u32::<BigEndian>()?;
    let node_count = reader.read_u32::<BigEndian>()?;

    let mut string_pool = {
        let start = header.node_offset as i64 + ARC_NODE_SIZE as i64 * node_count as i64;
        let end = (header.node_offset + header.size) as u64;

        tracing::trace!("ARC string pool is in range {start}..{end}");

        let mut pool = reader.clone();
        pool.set_position(start as u64);
        pool.set_tail();

        pool
    };

    let mut nodes = Vec::with_capacity(node_count as usize);
    nodes.push(Node {
        name: "<null>".to_owned(),
        data: NodeContent::Directory {
            parent: _data1,
            skip_node: node_count,
        },
    });

    tracing::trace!("Reading {node_count} nodes");
    for _ in 1..node_count {
        let node = Node::deserialize(reader, &mut string_pool)?;
        nodes.push(node);
    }

    let mut cursor = 0;

    tracing::trace!("Constructing directory tree and parsing nodes...");
    let ret =
        construct_directory_tree(&mut nodes, parent_id, arena, name, header.size, &mut cursor)?;
    tracing::trace!("Constructed directory tree successfully");
    Ok(ret)
}
