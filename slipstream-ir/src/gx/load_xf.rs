use bitfield_struct::{bitenum, bitfield};
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, SlipstreamResult},
};

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct XfSizePayload {
    #[bits(2)]
    pub color_count: u8,
    #[bits(2)]
    pub normal_count: u8,
    #[bits(4)]
    pub uv_count: u8,
    #[bits(24)]
    _padding: u32,
}

impl XfSizePayload {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}

#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum XfProjectionType {
    St,
    Stq,
    #[fallback]
    Invalid,
}

#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum XfInputForm {
    /// Generally used with ST projection.
    Ab11,
    /// Generally used with STQ projection.
    Abc1,
    #[fallback]
    Invalid,
}

#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum XfTexGenType {
    Regular,
    EmbossMapping,
    ColorMapping1,
    ColorMapping2,
    #[fallback]
    Invalid,
}

#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum XfSourceRow {
    VertexGeometry,
    Normals,
    Colors,
    BinormalT,
    BinormalB,
    Uv1,
    Uv2,
    Uv3,
    Uv4,
    Uv5,
    Uv6,
    Uv7,
    #[fallback]
    Invalid,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct XfSetPayload {
    #[bits(1)]
    pub _unknown1: bool,
    #[bits(1)]
    pub projection: XfProjectionType,
    #[bits(1)]
    pub input_form: XfInputForm,
    #[bits(1, default = false)]
    pub _unknown2: bool,
    #[bits(3)]
    pub tex_gen_type: XfTexGenType,
    #[bits(5)]
    pub source_row: XfSourceRow,
    #[bits(3)]
    pub used_gen_tex_coord: u8,
    #[bits(3)]
    pub used_light_index: u8,
    #[bits(14)]
    _padding: u16,
}

impl XfSetPayload {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadXfPayload {
    Size(XfSizePayload),
    Set { tex_id: u8, payload: XfSetPayload },
    Unknown { tex_id: u8, payload: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadXfOpCode {
    pub loads: Vec<LoadXfPayload>,
}

impl LoadXfOpCode {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let transfer_size = reader.read_u16::<BigEndian>()?;
        let address = reader.read_u16::<BigEndian>()?;

        let mut loads = Vec::with_capacity(transfer_size as usize + 1);
        for _ in 0..transfer_size + 1 {
            let load = match address {
                0x1008 => LoadXfPayload::Size(XfSizePayload::deserialize(reader)?),
                0x1040..=0x1047 => {
                    let tex_id = (address - 0x1040) as u8;
                    LoadXfPayload::Set {
                        tex_id,
                        payload: XfSetPayload::deserialize(reader)?,
                    }
                }
                0x1050..=0x1057 => {
                    let tex_id = (address - 0x1050) as u8;
                    LoadXfPayload::Unknown {
                        tex_id,
                        payload: reader.read_u32::<BigEndian>()?,
                    }
                },
                _ => return Err(CorruptionError {
                    reason: format!("invalid XF register: {address:#06x} (expected 0x1008, 0x1040..=0x1047 or 0x1050..=0x1057)"),
                    location: Some(reader.position())
                }.into())
            };

            loads.push(load);
        }

        Ok(Self { loads })
    }
}
