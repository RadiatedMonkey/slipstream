use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, InvalidInputError, SlipstreamError, SlipstreamResult},
};

use crate::{
    encoding::ReadArrayExt,
    gx::{
        GxBytecode, GxOpCode,
        load_cp::{CpVatA, CpVatB, CpVatC, CpVcdHi, CpVcdLo, LoadCpOpCode},
        load_xf::{LoadXfOpCode, LoadXfPayload},
    },
    node::node::IrNodeType,
    section::DeserializeSection,
    visitor::{Visitable, Visitor},
};

/// Maps shape local matrix IDs to global ones.
///
/// Primitives rigged by multiple bones will have their [`pn_index_enabled`] flag set.
/// Every primitive then stores an index into this table. The table entry then maps this index
/// to global matrices.
///
/// [`pn_index_enabled`]: CpVcdLo::pn_index_enabled
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoneTable {
    pub entries: Vec<u16>,
}

impl BoneTable {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let entry_count = reader.read_u32::<BigEndian>()?;

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            entries.push(reader.read_u16::<BigEndian>()?);
        }

        Ok(Self { entries })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PolygonModifier {
    None,
    ChangeCurrentMatrix,
    Invisible,
}

impl TryFrom<u32> for PolygonModifier {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::None,
            1 => Self::ChangeCurrentMatrix,
            2 => Self::Invisible,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid object modifier: {value} (expected 0, 1 or 2)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl PolygonModifier {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

/// Determines how this shape is bound to a bone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoneBind {
    /// The entire polygon is bound to a single bone.
    Rigid(u32),
    /// Sections of the polygon are bound to different bones.
    ///
    /// If the shape uses this bone bind type, the `GX_VA_PNMTXIDX` flag is set to true.
    /// and each primitive will store an index to its transformation matrix in this table.
    Mixed(BoneTable),
}

/// Setup bytecode for a shape.
///
/// This describes the formats of all the buffers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GxVertexDeclaration {
    /// The low vertex control descriptor.
    pub vcd_lo: CpVcdLo,
    /// The high vertex control descriptor.
    pub vcd_hi: CpVcdHi,
    /// The first vertex attribute table.
    pub vat_a: CpVatA,
    /// The second vertex attribute table.
    pub vat_b: CpVatB,
    /// The third vertex attribute table.
    pub vat_c: CpVatC,
    pub xf: Vec<LoadXfPayload>,
}

impl TryFrom<GxBytecode> for GxVertexDeclaration {
    type Error = SlipstreamError;

    fn try_from(value: GxBytecode) -> Result<Self, Self::Error> {
        let mut vcd_lo = None;
        let mut vcd_hi = None;
        let mut vat_a = None;
        let mut vat_b = None;
        let mut vat_c = None;
        let mut xf = None;

        for opcode in value.commands {
            match opcode {
                GxOpCode::LoadXf(LoadXfOpCode { loads }) => xf = Some(loads),
                GxOpCode::LoadCp(LoadCpOpCode::VcdLo(x)) => vcd_lo = Some(x),
                GxOpCode::LoadCp(LoadCpOpCode::VcdHi(x)) => vcd_hi = Some(x),
                GxOpCode::LoadCp(LoadCpOpCode::VatA(x)) => vat_a = Some(x),
                GxOpCode::LoadCp(LoadCpOpCode::VatB(x)) => vat_b = Some(x),
                GxOpCode::LoadCp(LoadCpOpCode::VatC(x)) => vat_c = Some(x),
                _ => {}
            }
        }

        Ok(Self {
            xf: xf.ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: String::from("vertex declaration did not contain LoadXF opcode"),
                    ..Default::default()
                })
            })?,
            vcd_lo: vcd_lo.ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: String::from("vertex declaration did not contain cp1 opcode"),
                    ..Default::default()
                })
            })?,
            vcd_hi: vcd_hi.ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: String::from("vertex declaration did not contain cp2 opcode"),
                    ..Default::default()
                })
            })?,
            vat_a: vat_a.ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: String::from("vertex declaration did not contain cp3 opcode"),
                    ..Default::default()
                })
            })?,
            vat_b: vat_b.ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: String::from("vertex declaration did not contain cp4 opcode"),
                    ..Default::default()
                })
            })?,
            vat_c: vat_c.ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: String::from("vertex declaration did not contain cp5 opcode"),
                    ..Default::default()
                })
            })?,
        })
    }
}

/// A shape/object/polygon describes how the model should be rendered.
///
/// It contains the actual draw commands for the Broadway GPU to execute.
#[derive(Debug, Clone)]
pub struct Polygon {
    pub array_flags: u32,
    pub modifier: PolygonModifier,
    /// This shape's index in the `Shapes` section.
    pub index: u32,
    /// The amount of vertices in this polygon.
    pub vertex_count: u32,
    pub face_count: u32,
    /// The vertex buffer to use for indexed draws.
    ///
    /// This is an index into the `Vertices` section of the model.
    pub vertex_array_id: u16,
    /// The normal buffer to use for indexed draws.
    ///
    /// This is an index into the `Normals` section of the model.
    pub normal_array_id: u16,
    /// The color buffer to use for indexed draws.
    ///
    /// This is an index into the `Colors` section of the model.
    pub color_array_ids: [u16; 2],
    /// The UV buffer to use for indexed draws.
    ///
    /// This is an index into the `UVs` section of the model.
    pub uv_array_ids: [u16; 8],
    /// Determines how this shape is bound to a bone for rigging.
    pub bone_bind: BoneBind,
    /// Setup bytecode for the buffer formats.
    ///
    /// See [`GxVertexDeclaration`] for more info.
    pub vertex_decl: GxVertexDeclaration,
    /// Bytecode containing this shape's draw calls.
    pub vertex_data_gx: GxBytecode,
}

impl Visitable for Polygon {
    fn accept(&self, visitor: &mut dyn Visitor) {
        visitor.visit_polygon(self)
    }
}

impl DeserializeSection for Polygon {
    const KIND: IrNodeType = IrNodeType::Polygon;

    #[tracing::instrument(skip_all, fields(header_start = _header_start))]
    fn deserialize_section(
        reader: &mut RefCursor<[u8]>,
        _header_start: u64,
    ) -> SlipstreamResult<Self> {
        let object_start = reader.position();
        let length = reader.read_u32::<BigEndian>()?;
        let mdl0_offset = reader.read_i32::<BigEndian>()?;
        let bone_index = match reader.read_i32::<BigEndian>()? {
            -1 => None,
            v => Some(v as u32),
        };

        let cp_vtx = reader.read_u32::<BigEndian>()?;
        let cp_tex = reader.read_u32::<BigEndian>()?;
        let xf_nor_spec = reader.read_u32::<BigEndian>()?;

        let definitions_buffer_size = reader.read_u32::<BigEndian>()?;
        let definitions_size = reader.read_u32::<BigEndian>()?;
        let definitions_offset = reader.read_i32::<BigEndian>()?;
        let vertex_buffer_size = reader.read_u32::<BigEndian>()?;
        let vertex_data_size = reader.read_u32::<BigEndian>()?;
        let vertex_data_offset = reader.read_i32::<BigEndian>()?;
        let array_flags = reader.read_u32::<BigEndian>()?;
        let modifier = PolygonModifier::deserialize(reader)?;
        let _name_offset = reader.read_u32::<BigEndian>()?;
        let index = reader.read_u32::<BigEndian>()?;
        let vertex_count = reader.read_u32::<BigEndian>()?;
        let face_count = reader.read_u32::<BigEndian>()?;
        let vertex_array_id = reader.read_u16::<BigEndian>()?;
        let normal_array_id = reader.read_u16::<BigEndian>()?;
        let color_array_ids = reader.read_u16_array::<2, BigEndian>()?;
        let uv_array_ids = reader.read_u16_array::<8, BigEndian>()?;
        let _unknown = reader.read_u32::<BigEndian>()?;
        let bone_table_offset = reader.read_u32::<BigEndian>()?;

        reader.set_position(object_start + bone_table_offset as u64);

        let bone_bind = match bone_index {
            Some(index) => BoneBind::Rigid(index),
            None => BoneBind::Mixed(BoneTable::deserialize(reader)?),
        };

        dbg!(&bone_bind);

        // The definitions and vertices offsets are relative to their fields, not the the file start.
        const VERTEX_DECL_INTERNAL_OFFSET: u64 = 0x20;
        const VERTEX_DATA_INTERNAL_OFFSET: u64 = 0x24;

        let definitions_start =
            object_start as i64 + VERTEX_DECL_INTERNAL_OFFSET as i64 + definitions_offset as i64;

        let definitions_end = definitions_start + definitions_size as i64;

        reader.set_position(definitions_start as u64);

        tracing::trace!("Reading vertex declaration GX bytecode");
        let vertex_decl = GxVertexDeclaration::try_from(
            GxBytecode::deserialize_vertex_declaration(reader, definitions_end as u64)?,
        )?;

        let vertices_start =
            object_start as i64 + VERTEX_DATA_INTERNAL_OFFSET as i64 + vertex_data_offset as i64;

        let vertices_end = vertices_start + vertex_data_size as i64;

        reader.set_position(vertices_start as u64);

        tracing::trace!("Reading vertex data GX bytecode");
        let vertex_data_gx =
            GxBytecode::deserialize_vertex_data(reader, &vertex_decl, vertices_end as u64)?;

        Ok(Self {
            vertex_count,
            face_count,
            vertex_array_id,
            normal_array_id,
            color_array_ids,
            uv_array_ids,

            array_flags,
            modifier,
            index,

            bone_bind,
            vertex_decl,
            vertex_data_gx,
        })
    }
}
