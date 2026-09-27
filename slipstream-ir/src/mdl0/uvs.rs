use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, SlipstreamResult},
};

use crate::{
    encoding::ReadArrayExt,
    index::IndexGroup,
    node::{
        defer::Deferred,
        node::{IrNodeType, VirtualNode, VirtualNodeBody},
        refs::{IrArena, IrNodeKey},
    },
    util::{VectorDivisor, VertexFormat, deserialize_scalar_data, deserialize_vector_data},
    visitor::{Visitable, Visitor},
};

const COMPONENTS_S: u32 = 0x00;
const COMPONENTS_ST: u32 = 0x01;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UvDataType {
    S,
    St,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UvBufData {
    S(Vec<f32>),
    St(Vec<[f32; 2]>),
}

impl UvBufData {
    pub fn ty(&self) -> UvDataType {
        match self {
            Self::S(_) => UvDataType::S,
            Self::St(_) => UvDataType::St,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UvBuf {
    pub index: u32,
    pub format: VertexFormat,
    pub stride: u8,
    pub uvs: UvBufData,
    pub bounding_volume_min: [f32; 2],
    pub bounding_volume_max: [f32; 2],
}

impl UvBuf {
    pub fn deserialize(reader: &mut RefCursor<[u8]>, header_start: u32) -> SlipstreamResult<Self> {
        let _length = reader.read_u32::<BigEndian>()?;
        let _mdl0_offset = reader.read_i32::<BigEndian>()?;
        let data_offset = reader.read_i32::<BigEndian>()?;
        let _name_offset = reader.read_i32::<BigEndian>()?;
        let index = reader.read_u32::<BigEndian>()?;
        let component_count = reader.read_u32::<BigEndian>()?;
        let format = VertexFormat::deserialize(reader)?;
        let divisor = reader.read_u8()?;
        let stride = reader.read_u8()?;

        let uv_count = reader.read_u16::<BigEndian>()?;
        let bounding_volume_min = reader.read_f32_array::<2, BigEndian>()?;
        let bounding_volume_max = reader.read_f32_array::<2, BigEndian>()?;

        let uv_start = header_start as i64 + data_offset as i64;
        reader.set_position(uv_start as u64);

        let uvs = match component_count {
            COMPONENTS_S => UvBufData::S(deserialize_scalar_data(
                reader,
                uv_count as usize,
                format,
                VectorDivisor::Custom(divisor),
            )?),
            COMPONENTS_ST => UvBufData::St(deserialize_vector_data::<2>(
                reader,
                uv_count as usize,
                format,
                VectorDivisor::Custom(divisor),
            )?),
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid UV format: {component_count} (expected 0, 1)"),
                    ..Default::default()
                }
                .into());
            }
        };

        Ok(Self {
            index,
            format,
            stride,
            uvs,
            bounding_volume_min,
            bounding_volume_max,
        })
    }
}

impl Visitable for UvBuf {
    fn accept(&self, visitor: &mut dyn Visitor) {
        visitor.visit_uvs(self)
    }
}

#[tracing::instrument(skip_all, fields(parent_id))]
pub fn deserialize_virtual(
    reader: &mut RefCursor<[u8]>,
    header_start: u32,
    parent_id: IrNodeKey,
    arena: &IrArena,
) -> SlipstreamResult<VirtualNodeBody> {
    let section_index = IndexGroup::deserialize(reader)?;

    let mut children = Vec::with_capacity(section_index.entries.len() - 1);
    for entry in &section_index.entries[1..] {
        let name = section_index.get_entry_name(reader, entry)?;
        let data_start = section_index.get_entry_data_start(entry);

        reader.set_position(data_start as u64);

        let uvs = UvBuf::deserialize(reader, header_start)?;

        let id = arena.next_id();
        let node = VirtualNode {
            label: name,
            id,
            kind: IrNodeType::Uvs,
            parent: Some(parent_id),
            body: Deferred::evaluated(VirtualNodeBody {
                children: Vec::new(),
                inspectable: Some(Box::new(uvs)),
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
