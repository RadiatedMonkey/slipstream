use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

use crate::{
    encoding::ReadArrayExt,
    gx::load_cp::VectorStorage,
    mdl0::{colors::deserialize_color, polygon::GxVertexDeclaration},
    util::{VectorDivisor, VertexFormat, deserialize_scalar, deserialize_vector},
};

/// Position data that is stored directly inside of a draw call.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DirectPosition {
    Xy([f32; 2]),
    Xyz([f32; 3]),
}

impl DirectPosition {
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        Ok(if decl.vat_a.pos_extended() {
            Self::Xyz(deserialize_vector::<3>(
                reader,
                decl.vat_a.pos_format(),
                VectorDivisor::Custom(decl.vat_a.pos_divisor()),
            )?)
        } else {
            Self::Xy(deserialize_vector::<2>(
                reader,
                decl.vat_a.pos_format(),
                VectorDivisor::Custom(decl.vat_a.pos_divisor()),
            )?)
        })
    }

    pub fn to_xyz(&self) -> [f32; 3] {
        match self {
            Self::Xy([x, y]) => [*x, *y, 0.0],
            Self::Xyz(x) => *x,
        }
    }
}

/// How the position data is stored by the draw call.
#[derive(Debug, Clone, PartialEq)]
pub enum PositionData {
    /// This vertex has no data. Skip it.
    NotPresent,
    /// This vertex is located at the given 8-bit index into a vertex buffer.
    ///
    /// The source vertex buffer is listed in the [`Shape`] header.
    ///
    /// [`Shape`]: crate::format::mdl0::shape::Shape
    Index8(u8),
    /// This vertex is located at the given 16-bit index into a vertex buffer.
    ///
    /// The source vertex buffer is listed in the [`Shape`] header.
    ///
    /// [`Shape`]: crate::format::mdl0::shape::Shape
    Index16(u16),
    /// The data for this vertex is stored directly inside of the draw call.
    Direct(DirectPosition),
}

impl PositionData {
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        let pos_storage = decl.vcd_lo.pos_storage();
        Ok(match pos_storage {
            VectorStorage::NotPresent => PositionData::NotPresent,
            VectorStorage::Index8 => PositionData::Index8(reader.read_u8()?),
            VectorStorage::Index16 => PositionData::Index16(reader.read_u16::<BigEndian>()?),
            VectorStorage::Direct => {
                PositionData::Direct(DirectPosition::deserialize(reader, decl)?)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DirectNormal {
    /// Stores only the normal
    Single([f32; 3]),
    /// Stores the normal, binormal and tangent in a single 9 float block.
    Packed([f32; 9]),
}

impl DirectNormal {
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        Ok(if decl.vat_a.norm_extended() {
            Self::Packed(deserialize_vector::<9>(
                reader,
                VertexFormat::from(decl.vat_a.norm_format()),
                VectorDivisor::Normalize,
            )?)
        } else {
            Self::Single(deserialize_vector::<3>(
                reader,
                VertexFormat::from(decl.vat_a.norm_format()),
                VectorDivisor::Normalize,
            )?)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalIndex<T> {
    /// If NBT mode is set, all three normal vectors are read consecutively.
    /// Otherwise only a single normal vector is read.
    Single(T),
    /// Should only appear when NBT mode is set. The three normal vectors each have their
    /// own indices.
    Triple([T; 3]),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NormalData {
    NotPresent,
    Index8(NormalIndex<u8>),
    Index16(NormalIndex<u16>),
    Direct(DirectNormal),
}

impl NormalData {
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        let norm_storage = decl.vcd_lo.norm_storage();
        Ok(match norm_storage {
            VectorStorage::NotPresent => NormalData::NotPresent,
            VectorStorage::Index8 => {
                let index = match (decl.vat_a.norm_extended(), decl.vat_a.norm_i3()) {
                    (true, true) => NormalIndex::Triple(reader.read_u8_array::<3>()?),
                    (_, false) => NormalIndex::Single(reader.read_u8()?),
                    (false, true) => {
                        unreachable!("norm_i3 cannot be set while norm_extended is unset")
                    }
                };

                NormalData::Index8(index)
            }
            VectorStorage::Index16 => {
                let index = match (decl.vat_a.norm_extended(), decl.vat_a.norm_i3()) {
                    (true, true) => NormalIndex::Triple(reader.read_u16_array::<3, BigEndian>()?),
                    (_, false) => NormalIndex::Single(reader.read_u16::<BigEndian>()?),
                    (false, true) => {
                        unreachable!("norm_i3 cannot be set while norm_extended is unset")
                    }
                };

                NormalData::Index16(index)
            }
            VectorStorage::Direct => NormalData::Direct(DirectNormal::deserialize(reader, decl)?),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DirectColor {
    AlphaDisabled(glam::U8Vec4),
    AlphaEnabled(glam::U8Vec4),
}

impl DirectColor {
    pub fn deserialize_col0(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        Ok(if decl.vat_a.col0_extended() {
            Self::AlphaEnabled(deserialize_color(reader, decl.vat_a.col0_format())?)
        } else {
            Self::AlphaDisabled(deserialize_color(reader, decl.vat_a.col0_format())?)
        })
    }

    pub fn deserialize_col1(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        Ok(if decl.vat_a.col1_extended() {
            Self::AlphaEnabled(deserialize_color(reader, decl.vat_a.col1_format())?)
        } else {
            Self::AlphaDisabled(deserialize_color(reader, decl.vat_a.col1_format())?)
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorData {
    NotPresent,
    Index8(u8),
    Index16(u16),
    Direct(DirectColor),
}

impl ColorData {
    pub fn deserialize_col0(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        let color_storage = decl.vcd_lo.col0_storage();
        Ok(match color_storage {
            VectorStorage::NotPresent => Self::NotPresent,
            VectorStorage::Index8 => Self::Index8(reader.read_u8()?),
            VectorStorage::Index16 => Self::Index16(reader.read_u16::<BigEndian>()?),
            VectorStorage::Direct => Self::Direct(DirectColor::deserialize_col0(reader, decl)?),
        })
    }

    pub fn deserialize_col1(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        let color_storage = decl.vcd_lo.col1_storage();
        Ok(match color_storage {
            VectorStorage::NotPresent => Self::NotPresent,
            VectorStorage::Index8 => Self::Index8(reader.read_u8()?),
            VectorStorage::Index16 => Self::Index16(reader.read_u16::<BigEndian>()?),
            VectorStorage::Direct => Self::Direct(DirectColor::deserialize_col1(reader, decl)?),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DirectUv {
    S(f32),
    St([f32; 2]),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UvData {
    NotPresent,
    Index8(u8),
    Index16(u16),
    Direct(DirectUv),
}

/// Implements the deserialisation methods for all 7 UV fields.
// This is incredibly overengineered, but oh well.
macro_rules! impl_uv_de {
    // $id is the id of the UV (ranging from 0-7)
    //
    // $cp1 is the subcommand that contains the format and extended flag.
    // $cp2 contains the divisor.
    // For all, except uv4, these are equal.
    ($($id:literal => $cp1:ident + $cp2:ident),*) => {
        paste::paste! {
            impl DirectUv {
                $(
                    pub fn [< deserialize_uv $id >](reader: &mut RefCursor<[u8]>, decl: &GxVertexDeclaration) -> SlipstreamResult<Self> {
                        let format = decl.[< $cp1 >].[< uv $id _format >]();
                        let divisor = decl.[< $cp2 >].[< uv $id _divisor >]();

                        Ok(if decl.[< $cp1 >].[< uv $id _extended >]() {
                            Self::St(deserialize_vector::<2>(reader, format, VectorDivisor::Custom(divisor))?)
                        } else {
                            Self::S(deserialize_scalar(reader, format, VectorDivisor::Custom(divisor))?)
                        })
                    }
                )*
            }

            impl UvData {
                $(
                    pub fn [< deserialize_uv $id >](reader: &mut RefCursor<[u8]>, decl: &GxVertexDeclaration) -> SlipstreamResult<Self> {
                        let uv_storage = decl.vcd_hi.[< uv $id _storage >]();
                        Ok(match uv_storage {
                            VectorStorage::NotPresent => Self::NotPresent,
                            VectorStorage::Index8 => Self::Index8(reader.read_u8()?),
                            VectorStorage::Index16 => Self::Index16(reader.read_u16::<BigEndian>()?),
                            VectorStorage::Direct => Self::Direct(DirectUv::[< deserialize_uv $id >](reader, decl)?)
                        })
                    }
                )*
            }
        }
    };
}

impl_uv_de!(
    0 => vat_a + vat_a,
    1 => vat_b + vat_b,
    2 => vat_b + vat_b,
    3 => vat_b + vat_b,
    4 => vat_b + vat_c,
    5 => vat_c + vat_c,
    6 => vat_c + vat_c,
    7 => vat_c + vat_c
);

#[derive(Debug, Clone, PartialEq)]
pub struct OpVertex {
    /// The position/normal matrix that should be used to transform this vertex.
    ///
    /// This is an index into the parent shape's bone table.
    pub pn_matrix_index: Option<u8>,
    pub tms: [Option<u8>; 8],
    pub position: PositionData,
    pub normals: NormalData,
    pub color0: ColorData,
    pub color1: ColorData,
    pub uvs: [UvData; 8],
}

impl OpVertex {
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        decl: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        let mut byte_read = || reader.read_u8();

        // The index must be divided by 3 to get the true index. This converts the byte offset
        // in XF matrix memory to an array index into the table.
        let pn_matrix_index = decl
            .vcd_lo
            .pn_index_enabled()
            .then(&mut byte_read)
            .transpose()?
            .map(|x| x / 3);

        let tms = [
            decl.vcd_lo.tm0().then(&mut byte_read).transpose()?,
            decl.vcd_lo.tm1().then(&mut byte_read).transpose()?,
            decl.vcd_lo.tm2().then(&mut byte_read).transpose()?,
            decl.vcd_lo.tm3().then(&mut byte_read).transpose()?,
            decl.vcd_lo.tm4().then(&mut byte_read).transpose()?,
            decl.vcd_lo.tm5().then(&mut byte_read).transpose()?,
            decl.vcd_lo.tm6().then(&mut byte_read).transpose()?,
            decl.vcd_lo.tm7().then(byte_read).transpose()?,
        ];

        let position = PositionData::deserialize(reader, decl)?;
        let normals = NormalData::deserialize(reader, decl)?;
        let color0 = ColorData::deserialize_col0(reader, decl)?;
        let color1 = ColorData::deserialize_col1(reader, decl)?;

        let uvs = [
            UvData::deserialize_uv0(reader, decl)?,
            UvData::deserialize_uv1(reader, decl)?,
            UvData::deserialize_uv2(reader, decl)?,
            UvData::deserialize_uv3(reader, decl)?,
            UvData::deserialize_uv4(reader, decl)?,
            UvData::deserialize_uv5(reader, decl)?,
            UvData::deserialize_uv6(reader, decl)?,
            UvData::deserialize_uv7(reader, decl)?,
        ];

        Ok(OpVertex {
            pn_matrix_index,
            tms,
            position,
            normals,
            color0,
            color1,
            uvs,
        })
    }
}

/// Raw drawing commands.
///
/// These are directly for the Wii, not suitable for PC rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawOpCode {
    pub vertices: Vec<OpVertex>,
}

impl DrawOpCode {
    /// Decodes the draw command based on the vertex info given in the `LoadCP` opcode.
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        cp_opcodes: &GxVertexDeclaration,
    ) -> SlipstreamResult<Self> {
        let vertex_count = reader.read_u16::<BigEndian>()?;
        tracing::trace!("deserializing {vertex_count} vertices");

        let mut vertices = Vec::with_capacity(vertex_count as usize);
        for _ in 0..vertex_count {
            vertices.push(OpVertex::deserialize(reader, cp_opcodes)?);
        }

        Ok(Self { vertices })
    }
}
