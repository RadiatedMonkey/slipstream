use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

use crate::{
    index::IndexGroup,
    node::{
        defer::Deferred,
        node::{IrNodeType, VirtualNode, VirtualNodeBody},
        refs::{IrArena, IrNodeKey},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct TextureLink {
    pub offset1: u32,
    pub offset2: u32,
}

impl TextureLink {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let offset1 = reader.read_u32::<BigEndian>()?;
        let offset2 = reader.read_u32::<BigEndian>()?;

        Ok(Self { offset1, offset2 })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureLinks {
    pub links: Vec<TextureLink>,
}

impl TextureLinks {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let link_count = reader.read_u32::<BigEndian>()?;

        let mut links = Vec::with_capacity(link_count as usize);
        for _ in 0..link_count {
            links.push(TextureLink::deserialize(reader)?);
        }

        Ok(Self { links })
    }
}

#[tracing::instrument(skip_all, fields(parent_id))]
pub fn deserialize_virtual(
    reader: &mut RefCursor<[u8]>,
    parent_id: IrNodeKey,
    arena: &IrArena,
) -> SlipstreamResult<VirtualNodeBody> {
    let section_index = IndexGroup::deserialize(reader)?;

    let mut children = Vec::with_capacity(section_index.entries.len() - 1);
    for entry in &section_index.entries[1..] {
        let name = section_index.get_entry_name(reader, entry)?;
        let data_start = section_index.get_entry_data_start(entry);

        reader.set_position(data_start as u64);

        let links = TextureLinks::deserialize(reader)?;

        let id = arena.next_id();
        let node = VirtualNode {
            label: name,
            id,
            parent: Some(parent_id),
            kind: IrNodeType::TextureLinks,
            body: Deferred::evaluated(VirtualNodeBody {
                children: Vec::new(),
                inspectable: Some(Box::new(links)),
            }),
        };

        arena.insert(id, node);
        children.push(id);
    }

    Ok(VirtualNodeBody {
        children,
        inspectable: None,
    })
}
