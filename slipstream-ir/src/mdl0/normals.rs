use std::ops::ControlFlow;

use bitfield_struct::bitenum;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use slipstream_shared::cursor::{MutCursor, RefCursor};
use slipstream_shared::error::{CorruptionError, SlipstreamError, SlipstreamResult};
use slipstream_shared::verify;

use crate::mdl0::section::DeserializeContents;
use crate::node::node::IrNodeType;
use crate::util::{VectorDivisor, VertexFormat, deserialize_vector_data};
use crate::visitor::{Visitable, Visitor};

const COMPONENTS_NORMAL: u32 = 0x0;
const COMPONENTS_ALL: u32 = 0x1;
const COMPONENTS_ANY: u32 = 0x2;

/// This enum has the same variant to value mapping as [`VertexFormat`] but leaves out
/// the formats that are invalid for normal data (i.e only signed formats).
///
/// [`VertexFormat`]: crate::format::mdl0::util::VertexFormat
#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum NormalFormat {
    Int8 = 1,
    Int16 = 3,
    Float32 = 4,
    /// Fallback value for `bitenum`, this variant should never be used.
    #[fallback]
    Invalid,
}

impl TryFrom<u32> for NormalFormat {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        // Literally the same as VertexFormat, except that it only supports signed
        // formats.
        Ok(match value {
            // 0 => Self::Uint8,
            1 => Self::Int8,
            // 2 => Self::Uint16,
            3 => Self::Int16,
            4 => Self::Float32,
            v => {
                return Err(CorruptionError {
                    reason: format!("invalid vertex format: {v} (expected 1, 3 or 4)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl NormalFormat {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }

    fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        verify!(
            *self != Self::Invalid,
            "cannot serialize NormalFormat::Invalid"
        );

        writer.write_u32::<BigEndian>(*self as u32)?;
        Ok(())
    }
}

/// The type of normals that are stored in the normal buffer.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NormalBufType {
    /// Only the normal itself is included in the buffer.
    Normal,
    /// All three (normal/binormal/tangent) vectors are included in the buffer.
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NormalBufData {
    /// Only the normal.
    Single(Vec<[f32; 3]>),
    /// Includes all of the normal, bi-normal and tangent
    Triple(Vec<[f32; 9]>),
}

impl NormalBufData {
    pub fn ty(&self) -> NormalBufType {
        match self {
            Self::Single(_) => NormalBufType::Normal,
            Self::Triple(_) => NormalBufType::All,
        }
    }

    /// Returns the amount of entries in the buffer.
    ///
    /// This counts the [`All`] variant as one entry.
    ///
    /// [`Nbt3`]: NormalBufType::All
    pub fn len(&self) -> usize {
        match self {
            Self::Single(x) => x.len(),
            Self::Triple(x) => x.len(),
        }
    }
}

/// A large buffer of normals that the shape draw commands index into to draw their polygons.
///
/// The original file might store this data in a lower quality format, but the parser will always convert everything
/// to floats.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalBuffer {
    /// Index of this buffer into the `Normals` section of the model.
    pub index: u32,
    /// Scalar data type to use for the normal vectors.
    pub format: NormalFormat,
    /// The divisor is used to scale vectors at lower quality formats.
    ///
    /// For example if the vertex format is [`Int16`], then naively converting the
    /// vertices to floats would only give a range of -32,768 to 32,767 with whole integer intervals.
    ///
    /// The divisor is the power of 2 that is divided by the vertices to produce floats.
    /// I.e `float = int16 / 2^divisor`.
    ///
    /// [`Int16`]: NormalFormat::Int16
    pub divisor: u8,
    /// The size in bytes of each entry.
    pub stride: u8,
    /// The normal data.
    pub normals: NormalBufData,
}

impl NormalBuffer {
    pub fn get_normal(&self, index: usize) -> Option<[f32; 3]> {
        match &self.normals {
            NormalBufData::Single(x) => x.get(index).copied(),
            NormalBufData::Triple(x) => x.get(index).map(|[x, y, z, ..]| [*x, *y, *z]),
        }
    }
}

impl Visitable for NormalBuffer {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_normals(self)
    }

    fn accept_mut(&mut self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_normals_mut(self)
    }
}

impl DeserializeContents for NormalBuffer {
    const NAME: &str = "Normals";
    const KIND: IrNodeType = IrNodeType::NormalBuffer;

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
        let format = NormalFormat::deserialize(reader)?;
        let divisor = reader.read_u8()?;
        let stride = reader.read_u8()?;
        let normal_count = reader.read_u16::<BigEndian>()?;

        let normals_start = header_start as i64 + data_offset as i64;
        reader.set_position(normals_start as u64);

        let normals = match component_count {
            COMPONENTS_NORMAL => NormalBufData::Single(deserialize_vector_data::<3>(
                reader,
                normal_count as usize,
                VertexFormat::from(format),
                VectorDivisor::Custom(divisor),
            )?),
            COMPONENTS_ALL => NormalBufData::Triple(deserialize_vector_data::<9>(
                reader,
                normal_count as usize,
                VertexFormat::from(format),
                VectorDivisor::Custom(divisor),
            )?),
            COMPONENTS_ANY => NormalBufData::Single(deserialize_vector_data::<3>(
                reader,
                normal_count as usize,
                VertexFormat::from(format),
                VectorDivisor::Custom(divisor),
            )?),
            v => {
                return Err(CorruptionError {
                    reason: format!("invalid component count: {v} (expected 0-2)"),
                    location: Some(reader.position()),
                    ..Default::default()
                }
                .into());
            }
        };

        Ok(Self {
            index,
            format,
            divisor,
            stride,
            normals,
        })
    }
}
