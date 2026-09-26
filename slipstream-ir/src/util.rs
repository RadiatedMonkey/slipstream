use bitfield_struct::bitenum;
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, InvalidInputError, SlipstreamError, SlipstreamResult},
};

use crate::{encoding::ReadArrayExt, mdl0::normals::NormalFormat};

/// The data type used to store vertex data.
///
/// These formats are used for positions and UVs.
/// Normals and colors use their own formats.
///
/// The [`NormalFormat`] is a subset of this enum and can be infallibly converted into
/// this.
///
/// [`NormalFormat`]: crate::format::mdl0::normals::NormalFormat
#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum VertexFormat {
    Uint8 = 0,
    Int8 = 1,
    Uint16 = 2,
    Int16 = 3,
    Float32 = 4,
    /// Fallback value for `bitenum`, this variant should never be used.
    #[fallback]
    Invalid = 5,
}

impl From<NormalFormat> for VertexFormat {
    fn from(value: NormalFormat) -> Self {
        match value {
            NormalFormat::Int8 => Self::Int8,
            NormalFormat::Int16 => Self::Int16,
            NormalFormat::Float32 => Self::Float32,
            NormalFormat::Invalid => Self::Invalid,
        }
    }
}

impl TryFrom<u32> for VertexFormat {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Uint8,
            1 => Self::Int8,
            2 => Self::Uint16,
            3 => Self::Int16,
            4 => Self::Float32,
            v => {
                return Err(CorruptionError {
                    reason: format!("invalid vertex format: {v} (expected 0-4)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl VertexFormat {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        if word >= Self::Invalid as u32 {
            return Err(CorruptionError {
                reason: format!("invalid vector format: {word}, expected (0-4)"),
                ..Default::default()
            }
            .into());
        }

        Ok(Self::from_bits(word as u8))
    }
}

/// The divisor type to use for the vector.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VectorDivisor {
    /// This is mainly used in the GX `LoadCP` opcode which does not store
    /// the normal divisors.
    ///
    /// It automatically assumes the full range of the form maps to [-1, 1].
    Normalize,
    /// Sets a custom divisor.
    Custom(u8),
}

/// Deserializes a single value of the given format.
pub fn deserialize_scalar(
    reader: &mut RefCursor<[u8]>,
    format: VertexFormat,
    divisor: VectorDivisor,
) -> SlipstreamResult<f32> {
    let factor = match (format, divisor) {
        (VertexFormat::Int8, VectorDivisor::Normalize) => 1.0 / 128.0,
        (VertexFormat::Int16, VectorDivisor::Normalize) => 1.0 / 32768.0,
        (_, VectorDivisor::Custom(divisor)) => 1.0 / 2.0f32.powi(divisor as i32),
        _ => {
            return Err(InvalidInputError {
                reason: format!("invalid format-divisor combination: {format:?} and {divisor:?}"),
                location: Some(reader.position()),
            }
            .into());
        }
    };

    Ok(match format {
        VertexFormat::Uint8 => reader.read_u8()? as f32 * factor,
        VertexFormat::Int8 => reader.read_i8()? as f32 * factor,
        VertexFormat::Uint16 => reader.read_u16::<BigEndian>()? as f32 * factor,
        VertexFormat::Int16 => reader.read_i16::<BigEndian>()? as f32 * factor,
        VertexFormat::Float32 => reader.read_f32::<BigEndian>()?,
        VertexFormat::Invalid => {
            return Err(InvalidInputError {
                reason: format!("cannot deserialize scalar data with format `Invalid`"),
                ..Default::default()
            }
            .into());
        }
    })
}

/// Deserializes `count` amount of the values of the given format.
pub fn deserialize_scalar_data(
    reader: &mut RefCursor<[u8]>,
    count: usize,
    format: VertexFormat,
    divisor: VectorDivisor,
) -> SlipstreamResult<Vec<f32>> {
    let mut data = Vec::with_capacity(count);
    for _ in 0..count {
        data.push(deserialize_scalar(reader, format, divisor)?);
    }
    Ok(data)
}

/// Deserializes a single vector of the given format and size.
pub fn deserialize_vector<const N: usize>(
    reader: &mut RefCursor<[u8]>,
    format: VertexFormat,
    divisor: VectorDivisor,
) -> SlipstreamResult<[f32; N]> {
    let factor = match (format, divisor) {
        (VertexFormat::Int8, VectorDivisor::Normalize) => 1.0 / 128.0,
        (VertexFormat::Int16, VectorDivisor::Normalize) => 1.0 / 32768.0,
        (_, VectorDivisor::Custom(divisor)) => 1.0 / 2.0f32.powi(divisor as i32),
        _ => {
            return Err(InvalidInputError {
                reason: format!("invalid format-divisor combination: {format:?} and {divisor:?}"),
                location: Some(reader.position()),
            }
            .into());
        }
    };

    Ok(match format {
        VertexFormat::Uint8 => {
            let raw_comps = reader.read_u8_array::<N>()?;
            std::array::from_fn(|i| raw_comps[i] as f32 * factor)
        }
        VertexFormat::Int8 => {
            let raw_comps = reader.read_i8_array::<N>()?;
            std::array::from_fn(|i| raw_comps[i] as f32 * factor)
        }
        VertexFormat::Uint16 => {
            let raw_comps = reader.read_u16_array::<N, BigEndian>()?;
            std::array::from_fn(|i| raw_comps[i] as f32 * factor)
        }
        VertexFormat::Int16 => {
            let raw_comps = reader.read_i16_array::<N, BigEndian>()?;
            std::array::from_fn(|i| raw_comps[i] as f32 * factor)
        }
        VertexFormat::Float32 => reader.read_f32_array::<N, BigEndian>()?,
        VertexFormat::Invalid => {
            return Err(InvalidInputError {
                reason: format!("cannot deserialize vector data with format `Invalid`"),
                ..Default::default()
            }
            .into());
        }
    })
}

/// Deserializes a list of vectors of the given `count` and `format`.
pub fn deserialize_vector_data<const N: usize>(
    reader: &mut RefCursor<[u8]>,
    count: usize,
    format: VertexFormat,
    divisor: VectorDivisor,
) -> SlipstreamResult<Vec<[f32; N]>> {
    let mut data = Vec::with_capacity(count);
    for _ in 0..count {
        data.push(deserialize_vector::<N>(reader, format, divisor)?);
    }
    Ok(data)
}
