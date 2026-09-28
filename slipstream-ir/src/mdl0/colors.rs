use std::ops::ControlFlow;

use bitfield_struct::bitenum;
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, InvalidInputError, SlipstreamError, SlipstreamResult},
};

use crate::mdl0::section::DeserializeSection;
use crate::{
    encoding::ReadArrayExt,
    node::node::IrNodeType,
    visitor::{Visitable, Visitor},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ColorComponents {
    Rgb = 0x00,
    Rgba = 0x01,
}

impl TryFrom<u32> for ColorComponents {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Rgb,
            1 => Self::Rgba,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid color format: {value} (expected 0, 1)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl ColorComponents {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

/// Describes the format of the color.
#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ColorFormat {
    /// 16 total bits.
    ///
    /// Red: 5 bits;
    /// Green: 6 bits;
    /// Blue: 5 bits;
    /// Alpha: N/A
    Rgb565 = 0x00,
    /// 24 total bits.
    ///
    /// Red: 8 bits;
    /// Green: 8 bits;
    /// Blue: 8 bits;
    /// Alpha: N/A
    Rgb24 = 0x01,
    /// 32 total bits.
    ///
    /// Red: 8 bits;
    /// Green: 8 bits;
    /// Blue: 8 bits;
    /// Alpha: 8 bits (discarded)
    Rgbx32 = 0x02,
    /// 16 total bits.
    ///
    /// Red: 4 bits;
    /// Green: 4 bits;
    /// Blue: 4 bits;
    /// Alpha: 4 bits
    Rgba16 = 0x03,
    /// 24 total bits.
    ///
    /// Red: 6 bits;
    /// Green: 6 bits;
    /// Blue: 6 bits;
    /// Alpha: 6 bits
    Rgba24 = 0x04,
    /// 32 total bits.
    ///
    /// Red: 8 bits;
    /// Green: 8 bits;
    /// Blue: 8 bits;
    /// Alpha: 8 bits
    Rgba32 = 0x05,
    #[fallback]
    Invalid,
}

impl ColorFormat {
    /// The stride in bytes of the format.
    pub const fn stride(&self) -> u32 {
        match self {
            Self::Rgb565 => 2,
            Self::Rgb24 => 3,
            Self::Rgbx32 => 4,
            Self::Rgba16 => 2,
            Self::Rgba24 => 3,
            Self::Rgba32 => 4,
            Self::Invalid => 0,
        }
    }
}

impl TryFrom<u32> for ColorFormat {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Rgb565,
            0x01 => Self::Rgb24,
            0x02 => Self::Rgbx32,
            0x03 => Self::Rgba16,
            0x04 => Self::Rgba24,
            0x05 => Self::Rgba32,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid color format: {value} (expected 0-5)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl ColorFormat {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

/// A buffer of vertex colors.
///
/// This data cannot be used on its own. It is indexed into by the indices in the shape draw commands.
#[derive(Debug, Clone)]
pub struct ColorBuffer {
    index: u32,
    components: ColorComponents,
    format: ColorFormat,
    stride: u8,
    colors: Vec<glam::U8Vec4>,
}

pub fn deserialize_color(
    reader: &mut RefCursor<[u8]>,
    format: ColorFormat,
) -> SlipstreamResult<glam::U8Vec4> {
    Ok(match format {
        ColorFormat::Rgb565 => {
            let short = reader.read_u16::<BigEndian>()?;
            let r = ((short & 0xf8_00) >> 11) as u8; // Take 5 bits
            let g = ((short & 0x07_e0) >> 5) as u8; // Then another 6 bits
            let b = (short & 0x00_1f) as u8; // and lastly another 5 bits

            // rescale the components to the full 0-255 range.
            let r = (r << 3) | (r >> 2);
            let g = (g << 2) | (g >> 4);
            let b = (b << 3) | (b >> 2);

            glam::u8vec4(r, g, b, 255)
        }
        ColorFormat::Rgb24 => {
            let triad = reader.read_u24::<BigEndian>()?;
            let r = ((triad & 0xff_00_00) >> 16) as u8;
            let g = ((triad & 0x00_ff_00) >> 8) as u8;
            let b = (triad & 0x00_00_ff) as u8;

            // does not need rescaling since components are 8 bits.

            glam::u8vec4(r, g, b, 255)
        }
        ColorFormat::Rgbx32 => {
            let word = reader.read_u32::<BigEndian>()?;
            let r = ((word & 0xff_00_00_00) >> 24) as u8;
            let g = ((word & 0x00_ff_00_00) >> 16) as u8;
            let b = ((word & 0x00_00_ff_00) >> 8) as u8;
            // and we ignore the alpha??

            glam::u8vec4(r, g, b, 255)
        }
        ColorFormat::Rgba16 => {
            let short = reader.read_u16::<BigEndian>()?;
            let r = ((short & 0xf0_00) >> 12) as u8;
            let g = ((short & 0x0f_00) >> 8) as u8;
            let b = ((short & 0x00_f0) >> 4) as u8;
            let a = (short & 0x00_0f) as u8;

            // map from 4 bits to full 8-bit 0-255 range
            let r = (r << 4) | r;
            let g = (g << 4) | g;
            let b = (b << 4) | b;
            let a = (a << 4) | a;

            glam::u8vec4(r, g, b, a)
        }
        ColorFormat::Rgba24 => {
            let triad = reader.read_u24::<BigEndian>()?;
            let r = ((triad & 0xfc_00_00) >> 18) as u8;
            let g = ((triad & 0x03_f0_00) >> 12) as u8;
            let b = ((triad & 0x00_0f_c0) >> 6) as u8;
            let a = (triad & 0x00_00_3f) as u8;

            let r = (r << 2) | (r >> 4);
            let g = (g << 2) | (g >> 4);
            let b = (b << 2) | (b >> 4);
            let a = (a << 2) | (a >> 4);

            glam::u8vec4(r, g, b, a)
        }
        ColorFormat::Rgba32 => {
            let comps = reader.read_u8_array::<4>()?;
            glam::U8Vec4::from_array(comps)
        }
        ColorFormat::Invalid => {
            return Err(InvalidInputError {
                reason: String::from("cannot read color of invalid format"),
                location: Some(reader.position()),
            }
            .into());
        }
    })
}

impl Visitable for ColorBuffer {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_colors(self)
    }
}

impl DeserializeSection for ColorBuffer {
    const NAME: &str = "Colors";
    const KIND: IrNodeType = IrNodeType::ColorBuffer;

    fn deserialize_section(
        reader: &mut RefCursor<[u8]>,
        _header_start: u64,
    ) -> SlipstreamResult<Self> {
        let _length = reader.read_u32::<BigEndian>()?;
        let _mdl0_offset = reader.read_i32::<BigEndian>()?;
        let _data_offset = reader.read_i32::<BigEndian>()?;
        let _name_offset = reader.read_i32::<BigEndian>()?;
        let index = reader.read_u32::<BigEndian>()?;
        let components = ColorComponents::deserialize(reader)?;
        let format = ColorFormat::deserialize(reader)?;
        let stride = reader.read_u8()?;
        let _padding = reader.read_u8()?;
        let color_count = reader.read_u16::<BigEndian>()?;

        let mut colors = Vec::with_capacity(color_count as usize);
        for _ in 0..color_count {
            let color = deserialize_color(reader, format)?;
            colors.push(color);
        }

        Ok(Self {
            index,
            components,
            format,
            stride,
            colors,
        })
    }
}
