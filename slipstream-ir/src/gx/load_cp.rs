use bitfield_struct::{bitenum, bitfield};
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, SlipstreamResult},
};

use crate::{
    mdl0::{colors::ColorFormat, normals::NormalFormat},
    util::VertexFormat,
};

#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum VectorStorage {
    #[fallback]
    NotPresent = 0b00,
    Direct = 0b01,
    Index8 = 0b10,
    Index16 = 0b11,
}

/// Vertex control descriptor part 1/2.
#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct CpVcdLo {
    /// Whether the position/normal matrix index is stored in this vertex.
    ///
    /// This matrix index points to the matrix in the parent shape's bone table,
    /// that should be used to transform this vertex.
    ///
    /// Also called: `GX_VA_PNMTXIDX`
    pub pn_index_enabled: bool,
    pub tm0: bool,
    pub tm1: bool,
    pub tm2: bool,
    pub tm3: bool,
    pub tm4: bool,
    pub tm5: bool,
    pub tm6: bool,
    pub tm7: bool,
    #[bits(2)]
    pub pos_storage: VectorStorage,
    #[bits(2)]
    pub norm_storage: VectorStorage,
    #[bits(2)]
    pub col0_storage: VectorStorage,
    #[bits(2)]
    pub col1_storage: VectorStorage,
    #[bits(15)]
    _padding: u16,
}

/// Vertex control descriptor part 2/2.
#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct CpVcdHi {
    #[bits(2)]
    pub uv0_storage: VectorStorage,
    #[bits(2)]
    pub uv1_storage: VectorStorage,
    #[bits(2)]
    pub uv2_storage: VectorStorage,
    #[bits(2)]
    pub uv3_storage: VectorStorage,
    #[bits(2)]
    pub uv4_storage: VectorStorage,
    #[bits(2)]
    pub uv5_storage: VectorStorage,
    #[bits(2)]
    pub uv6_storage: VectorStorage,
    #[bits(2)]
    pub uv7_storage: VectorStorage,
    #[bits(16)]
    _padding: u16,
}

/// Vertex attribute table, part 1/3.
#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct CpVatA {
    pub pos_extended: bool,
    #[bits(3)]
    pub pos_format: VertexFormat,
    #[bits(5)]
    pub pos_divisor: u8,
    pub norm_extended: bool,
    #[bits(3)]
    pub norm_format: NormalFormat,
    pub col0_extended: bool,
    #[bits(3)]
    pub col0_format: ColorFormat,
    pub col1_extended: bool,
    #[bits(3)]
    pub col1_format: ColorFormat,
    pub uv0_extended: bool,
    #[bits(3)]
    pub uv0_format: VertexFormat,
    #[bits(5)]
    pub uv0_divisor: u8,
    pub dequant: bool,
    /// If this flag is set and [`norm_extended`] is also set, then normals are indexed using
    /// 3 separate indices. If not set and [`norm_extended`] is set, then the normals will be read
    /// as 9 consecutive floats.
    pub norm_i3: bool,
}

/// Vertex attribute table, part 2/3.
#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct CpVatB {
    pub uv1_extended: bool,
    #[bits(3)]
    pub uv1_format: VertexFormat,
    #[bits(5)]
    pub uv1_divisor: u8,
    pub uv2_extended: bool,
    #[bits(3)]
    pub uv2_format: VertexFormat,
    #[bits(5)]
    pub uv2_divisor: u8,
    pub uv3_extended: bool,
    #[bits(3)]
    pub uv3_format: VertexFormat,
    #[bits(5)]
    pub uv3_divisor: u8,
    pub uv4_extended: bool,
    #[bits(3)]
    pub uv4_format: VertexFormat,
    #[bits(1)]
    _padding: bool,
}

/// Vertex attribute table, part 3/3.
#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct CpVatC {
    #[bits(5)]
    pub uv4_divisor: u8,
    pub uv5_extended: bool,
    #[bits(3)]
    pub uv5_format: VertexFormat,
    #[bits(5)]
    pub uv5_divisor: u8,
    pub uv6_extended: bool,
    #[bits(3)]
    pub uv6_format: VertexFormat,
    #[bits(5)]
    pub uv6_divisor: u8,
    pub uv7_extended: bool,
    #[bits(3)]
    pub uv7_format: VertexFormat,
    #[bits(5)]
    pub uv7_divisor: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadCpOpCode {
    VcdLo(CpVcdLo),
    VcdHi(CpVcdHi),
    VatA(CpVatA),
    VatB(CpVatB),
    VatC(CpVatC),
}

impl LoadCpOpCode {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let byte = reader.read_u8()?;
        let word = reader.read_u32::<BigEndian>()?;

        Ok(match byte {
            0x50 => Self::VcdLo(CpVcdLo::from_bits(word)),
            0x60 => Self::VcdHi(CpVcdHi::from_bits(word)),
            0x70 => Self::VatA(CpVatA::from_bits(word)),
            0x80 => Self::VatB(CpVatB::from_bits(word)),
            0x90 => Self::VatC(CpVatC::from_bits(word)),
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid LoadCP subcommand: {byte:#04x}"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}
