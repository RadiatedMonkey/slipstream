use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::cursor::RefCursor;
use slipstream_shared::error::{CorruptionError, SlipstreamResult};

use crate::encoding::ReadArrayExt;
use crate::mdl0::section::DeserializeContents;
use crate::mdl0::SectionHeader;
use crate::node::node::{IrNode, IrNodeType};
use crate::util::{VectorDivisor, VertexFormat, deserialize_vector_data};
use crate::visitor::{
    Visitable, Visitor, VisitorContext, VisitorContextMut, VisitorContextNode,
    VisitorContextNodeMut,
};

const COMPONENTS_XY: u32 = 0x0;
const COMPONENTS_XYZ: u32 = 0x1;

#[derive(Debug, Clone, PartialEq)]
pub enum VertexPositionType {
    Xy,
    Xyz,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VertexBufData {
    Xy(Vec<[f32; 2]>),
    Xyz(Vec<[f32; 3]>),
}

impl VertexBufData {
    pub const fn ty(&self) -> VertexPositionType {
        match self {
            Self::Xy(_) => VertexPositionType::Xy,
            Self::Xyz(_) => VertexPositionType::Xyz,
        }
    }

    pub const fn components(&self) -> usize {
        match self {
            Self::Xy(_) => 2,
            Self::Xyz(_) => 3,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Xy(verts) => bytemuck::cast_slice(verts),
            Self::Xyz(verts) => bytemuck::cast_slice(verts),
        }
    }

    /// Returns the buffer size in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::Xy(verts) => 2 * size_of::<f32>() * verts.len(),
            Self::Xyz(verts) => 3 * size_of::<f32>() * verts.len(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Xy(verts) => verts.len(),
            Self::Xyz(verts) => verts.len(),
        }
    }
}

/// A vertex buffer.
///
/// These buffers are not useful on their own. The model's [`Shape`]s contain setup and
/// draw calls that use indices into these buffers.
///
/// [`Shape`]: crate::format::mdl0::shapes::Shape
#[derive(Debug, Clone, PartialEq)]
pub struct VertexBuffer {
    pub header: SectionHeader,
    /// The format of the vertices in this buffer.
    pub format: VertexFormat,
    /// The divisor is used to scale vectors at lower quality formats.
    ///
    /// For example if the vertex format is [`Uint8`], then naively converting the
    /// vertices to floats would only give a range of 0-255 with whole integer intervals.
    ///
    /// The divisor is the power of 2 that is divided by the vertices to produce floats.
    /// I.e `float = uint8 / 2^divisor`.
    ///
    /// [`Uint8`]: VertexFormat::Uint8
    pub divisor: u8,
    /// The size in bytes of each position.
    pub stride: u8,
    pub bounding_volume_min: [f32; 3],
    /// Outer AABB of this vertex buffer.
    pub bounding_volume_max: [f32; 3],
    /// The vertex data.
    pub vertices: VertexBufData,
}

impl VertexBuffer {
    /// Convenience method that loads the vertex at the given index and upcasts it to an XYZ vertex.
    /// For vertices with only two components, the Z component is set 0.
    pub fn get_xyz(&self, index: usize) -> Option<[f32; 3]> {
        match &self.vertices {
            VertexBufData::Xy(xy) => xy.get(index).map(|[x, y]| [*x, *y, 0.0]),
            VertexBufData::Xyz(xyz) => xyz.get(index).copied(),
        }
    }
}

impl Visitable for VertexBuffer {
    fn accept(&self, node: VisitorContextNode<'_>, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_vertices(VisitorContext::new(node, self))
    }

    fn accept_mut(
        &mut self,
        node: VisitorContextNodeMut<'_>,
        visitor: &mut dyn Visitor,
    ) -> ControlFlow<()> {
        visitor.visit_vertices_mut(VisitorContextMut::new(node, self))
    }
}

impl DeserializeContents for VertexBuffer {
    const NAME: &str = "Vertices";
    const KIND: IrNodeType = IrNodeType::VertexBuffer;

    #[tracing::instrument(skip_all, fields(header_start))]
    fn deserialize_contents(
        reader: &mut RefCursor<[u8]>
    ) -> SlipstreamResult<Self> {
        let header = SectionHeader::deserialize(reader)?;
        let component_count = reader.read_u32::<BigEndian>()?;
        let format = VertexFormat::deserialize(reader)?;
        let divisor = reader.read_u8()?;
        let stride = reader.read_u8()?;
        let vertex_count = reader.read_u16::<BigEndian>()?;
        let bounding_volume_min = reader.read_f32_array::<3, BigEndian>()?;
        let bounding_volume_max = reader.read_f32_array::<3, BigEndian>()?;

        tracing::trace!("Reading {vertex_count} vertices");

        reader.set_position(header.get_data_start());

        let vertices = match component_count {
            COMPONENTS_XY => VertexBufData::Xy(deserialize_vector_data::<2>(
                reader,
                vertex_count as usize,
                format,
                VectorDivisor::Custom(divisor),
            )?),
            COMPONENTS_XYZ => VertexBufData::Xyz(deserialize_vector_data::<3>(
                reader,
                vertex_count as usize,
                format,
                VectorDivisor::Custom(divisor),
            )?),
            v => {
                return Err(CorruptionError {
                    reason: format!("invalid vertex component count: {v} (expected 2 or 3)"),
                    location: Some(reader.position()),
                }
                .into());
            }
        };

        Ok(Self {
            header,
            vertices,
            format,
            divisor,
            stride,
            bounding_volume_min,
            bounding_volume_max,
        })
    }
}
