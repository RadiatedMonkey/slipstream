use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, SlipstreamResult},
};

use crate::mdl0::section::DeserializeContents;
use crate::{
    encoding::ReadArrayExt,
    node::node::IrNodeType,
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
pub enum UvData {
    S(Vec<f32>),
    St(Vec<[f32; 2]>),
}

impl UvData {
    pub fn ty(&self) -> UvDataType {
        match self {
            Self::S(_) => UvDataType::S,
            Self::St(_) => UvDataType::St,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UvBuffer {
    pub index: u32,
    pub format: VertexFormat,
    pub stride: u8,
    pub uvs: UvData,
    pub bounding_volume_min: [f32; 2],
    pub bounding_volume_max: [f32; 2],
}

impl Visitable for UvBuffer {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_uvs(self)
    }

    fn accept_mut(&mut self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_uvs_mut(self)
    }
}

impl DeserializeContents for UvBuffer {
    const NAME: &str = "UVs";
    const KIND: IrNodeType = IrNodeType::UvBuffer;

    #[tracing::instrument(skip_all, fields(header_start))]
    fn deserialize_contents(
        reader: &mut RefCursor<[u8]>,
        header_start: u64,
    ) -> SlipstreamResult<Self> {
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
            COMPONENTS_S => UvData::S(deserialize_scalar_data(
                reader,
                uv_count as usize,
                format,
                VectorDivisor::Custom(divisor),
            )?),
            COMPONENTS_ST => UvData::St(deserialize_vector_data::<2>(
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
