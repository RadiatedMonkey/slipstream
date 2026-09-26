use crate::encoding::ReadArrayExt;
use crate::gx::GxBytecode;
use crate::index::IndexGroup;
use crate::node::defer::Deferred;
use crate::node::node::{VirtualNode, VirtualNodeBody, VirtualNodeKind};
use crate::node::refs::{VirtualNodeId, VirtualNodeMap};
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::cursor::RefCursor;
use slipstream_shared::error::SlipstreamResult;

#[derive(Debug, Clone, PartialEq)]
pub struct Tev {
    pub tex_scales: [u8; 8],
    pub bytecode: GxBytecode,
}

impl Tev {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let length = reader.read_u32::<BigEndian>()?;
        let mdl0_offset = reader.read_i32::<BigEndian>()?;
        let index = reader.read_i32::<BigEndian>()?;
        let stage_count = reader.read_u8()?;

        reader.set_position(reader.position() + 3); // padding

        let tex_scales = reader.read_u8_array::<8>()?;

        reader.set_position(reader.position() + 8); // padding

        let bytecode = GxBytecode::deserialize_tev_data(reader)?;

        Ok(Self {
            tex_scales,
            bytecode,
        })
    }
}

#[tracing::instrument(skip_all, fields(parent_id))]
pub fn deserialize_virtual(
    reader: &mut RefCursor<[u8]>,
    parent_id: VirtualNodeId,
    node_map: &VirtualNodeMap,
) -> SlipstreamResult<VirtualNodeBody> {
    let section_index = IndexGroup::deserialize(reader)?;

    let mut tevs = Vec::with_capacity(section_index.entries.len() - 1);
    for entry in &section_index.entries[1..] {
        let name = section_index.get_entry_name(reader, entry)?;
        let data_start = section_index.get_entry_data_start(entry);

        reader.set_position(data_start as u64);

        let tev = Tev::deserialize(reader)?;
        let id = node_map.next_id();
        let node = VirtualNode {
            label: name,
            id,
            parent: Some(parent_id),
            kind: VirtualNodeKind::Tevs,
            body: Deferred::evaluated(VirtualNodeBody {
                children: Vec::new(),
                inspectable: Some(Box::new(tev)),
            }),
        };

        node_map.insert(id, node);
        tevs.push(id);
    }

    tracing::error!("TODO TEVS");

    Ok(VirtualNodeBody {
        children: tevs,
        inspectable: None,
    })
}
