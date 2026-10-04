use byteorder::ReadBytesExt;
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, SlipstreamError, SlipstreamResult},
    verify,
};

use crate::{
    gx::{
        call::CallDisplayList, draw::DrawOpCode, load_bp::LoadBpOpCode, load_cp::LoadCpOpCode,
        load_indexed::IndexedLoad, load_xf::LoadXfOpCode,
    },
    mdl0::polygon::GxVertexDeclaration,
};

pub mod call;
pub mod draw;
pub mod load_bp;
pub mod load_cp;
pub mod load_indexed;
pub mod load_xf;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GxOpCodeId {
    /// No-op
    Nop = 0x00,
    /// Load CP (command processor) register.
    LoadCp = 0x08,
    /// Load XF (transform unit) register.
    LoadXf = 0x10,
    /// Load indexed 3x3 position matrix.
    LoadIndexedPositionMatrix = 0x20,
    /// Load indexed 3x3 normal matrix.
    LoadIndexedNormalMatrix = 0x28,
    /// Load indexed texture matrix.
    LoadIndexedTextureMatrix = 0x30,
    /// Load indexed light object.
    LoadIndexedLightObject = 0x38,
    /// Call display list
    Call = 0x40,
    /// Unknown opcode.
    Unknown = 0x44,
    /// Invalidate vertex cache.
    InvalidateVertexCache = 0x48,
    /// Load BP (blitting processor) register.
    LoadBp = 0x61,
    /// Draw quads.
    DrawQuads = 0x80,
    /// Draw triangles.
    DrawTriangles = 0x90,
    /// Draw triangle strip.
    DrawTriangleStrip = 0x98,
    /// Draw triangle fan.
    DrawTriangleFan = 0xa0,
    /// Draw lines.
    DrawLines = 0xa8,
    /// Draw line strip.
    DrawLineStrip = 0xb0,
    /// Draw points.
    DrawPoints = 0xb8,
}

impl TryFrom<u8> for GxOpCodeId {
    type Error = SlipstreamError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use GxOpCodeId::*;

        Ok(match value {
            0x00 => Nop,
            0x08 => LoadCp,
            0x10 => LoadXf,
            0x20 => LoadIndexedPositionMatrix,
            0x28 => LoadIndexedNormalMatrix,
            0x30 => LoadIndexedTextureMatrix,
            0x38 => LoadIndexedLightObject,
            0x40 => Call,
            0x44 => Unknown,
            0x48 => InvalidateVertexCache,
            0x61 => LoadBp,
            0x80 => DrawQuads,
            0x90 => DrawTriangles,
            0x98 => DrawTriangleStrip,
            0xa0 => DrawTriangleFan,
            0xa8 => DrawLines,
            0xb0 => DrawLineStrip,
            0xb8 => DrawPoints,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid GX opcode: {value:#04x}"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GxOpCode {
    Nop,
    LoadCp(LoadCpOpCode),
    LoadXf(LoadXfOpCode),
    LoadBp(LoadBpOpCode),
    LoadIndexedPosition(IndexedLoad),
    LoadIndexedNormal(IndexedLoad),
    LoadIndexedTextureMatrix(IndexedLoad),
    LoadIndexedLightObject(IndexedLoad),
    Call(CallDisplayList),
    InvalidateVertexCache,
    DrawQuads(DrawOpCode),
    DrawTriangles(DrawOpCode),
    DrawTriangleStrip(DrawOpCode),
    DrawTriangleFan(DrawOpCode),
    DrawLines(DrawOpCode),
    DrawLineStrip(DrawOpCode),
    DrawPoints(DrawOpCode),
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GxBytecode {
    pub commands: Vec<GxOpCode>,
}

impl GxBytecode {
    pub fn deserialize_vertex_declaration(
        reader: &mut RefCursor<[u8]>,
        section_end: u64,
    ) -> SlipstreamResult<Self> {
        let mut commands = Vec::new();
        while reader.position() < section_end {
            let opcode = GxOpCodeId::try_from(reader.read_u8()?)?;
            commands.push(match opcode {
                GxOpCodeId::Nop => continue,
                GxOpCodeId::LoadCp => GxOpCode::LoadCp(LoadCpOpCode::deserialize(reader)?),
                GxOpCodeId::LoadXf => GxOpCode::LoadXf(LoadXfOpCode::deserialize(reader)?),
                GxOpCodeId::LoadBp => GxOpCode::LoadBp(LoadBpOpCode::deserialize(reader)?),
                _ => {
                    return Err(CorruptionError {
                        reason: format!("invalid vertex declaration GX opcode: {opcode:?}"),
                        location: Some(reader.position()),
                    }
                    .into());
                }
            });
        }

        if reader.position() != section_end {
            return Err(CorruptionError {
                reason: format!("did not read correct amount of opcodes in vertex declaration GX bytecode ({} vs. {})", reader.position(), section_end),
                location: Some(reader.position())
            }.into());
        }

        Ok(Self { commands })
    }

    pub fn deserialize_vertex_data(
        reader: &mut RefCursor<[u8]>,
        vertex_decl: &GxVertexDeclaration,
        section_end: u64,
    ) -> SlipstreamResult<Self> {
        let mut commands = Vec::new();
        while reader.position() < section_end {
            let opcode = GxOpCodeId::try_from(reader.read_u8()?)?;
            commands.push(match opcode {
                GxOpCodeId::Nop => continue,
                GxOpCodeId::LoadIndexedPositionMatrix => {
                    GxOpCode::LoadIndexedPosition(IndexedLoad::deserialize(reader)?)
                }
                GxOpCodeId::LoadIndexedNormalMatrix => {
                    GxOpCode::LoadIndexedNormal(IndexedLoad::deserialize(reader)?)
                }
                GxOpCodeId::LoadIndexedTextureMatrix => {
                    GxOpCode::LoadIndexedTextureMatrix(IndexedLoad::deserialize(reader)?)
                }
                GxOpCodeId::LoadIndexedLightObject => {
                    GxOpCode::LoadIndexedLightObject(IndexedLoad::deserialize(reader)?)
                }
                GxOpCodeId::Call => GxOpCode::Call(CallDisplayList::deserialize(reader)?),
                GxOpCodeId::Unknown => GxOpCode::Unknown,
                GxOpCodeId::InvalidateVertexCache => GxOpCode::InvalidateVertexCache,
                GxOpCodeId::DrawQuads => {
                    GxOpCode::DrawQuads(DrawOpCode::deserialize(reader, vertex_decl)?)
                }
                GxOpCodeId::DrawTriangles => {
                    GxOpCode::DrawTriangles(DrawOpCode::deserialize(reader, vertex_decl)?)
                }
                GxOpCodeId::DrawTriangleStrip => {
                    GxOpCode::DrawTriangleStrip(DrawOpCode::deserialize(reader, vertex_decl)?)
                }
                GxOpCodeId::DrawTriangleFan => {
                    GxOpCode::DrawTriangleFan(DrawOpCode::deserialize(reader, vertex_decl)?)
                }
                GxOpCodeId::DrawLines => {
                    GxOpCode::DrawLines(DrawOpCode::deserialize(reader, vertex_decl)?)
                }
                GxOpCodeId::DrawLineStrip => {
                    GxOpCode::DrawLineStrip(DrawOpCode::deserialize(reader, vertex_decl)?)
                }
                GxOpCodeId::DrawPoints => {
                    GxOpCode::DrawPoints(DrawOpCode::deserialize(reader, vertex_decl)?)
                }
                _ => {
                    return Err(CorruptionError {
                        reason: format!("invalid vertex data GX opcode: {opcode:?}"),
                        location: Some(reader.position()),
                    }
                    .into());
                }
            });
        }

        verify!(
            reader.position() == section_end,
            "did not read correct amount of opcodes in vertex declaration Gx bytecode ({} vs {})",
            reader.position(),
            section_end
        );

        Ok(Self { commands })
    }

    pub fn deserialize_tev_data(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        const TEV_BYTECODE_SIZE: usize = 0x20; // Size is always 0x20

        let section_end = reader.position() + TEV_BYTECODE_SIZE as u64;
        let mut commands = Vec::new();

        while reader.position() < section_end {
            let opcode = GxOpCodeId::try_from(reader.read_u8()?)?;
            commands.push(match opcode {
                GxOpCodeId::LoadBp => GxOpCode::LoadBp(LoadBpOpCode::deserialize(reader)?),
                _ => {
                    return Err(CorruptionError {
                        reason: format!("invalid TEV bytecode opcode ID: {opcode:?}"),
                        location: Some(reader.position()),
                    }
                    .into());
                }
            });

            tracing::trace!("TEV opcode: {opcode:?}");

            break;
        }

        Ok(Self { commands })
    }
}
