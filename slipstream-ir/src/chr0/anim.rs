use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{RefCursor, SlipstreamResult};

/// A 4-byte animation frame.
#[derive(Debug, Clone, PartialEq)]
pub struct I4Frame {
    /// The index of this frame in the animation.
    pub index: u8,
    pub step: u8,
    /// The tangent line to the interpolation slope of the animation.
    pub tangent: f32,
}

impl I4Frame {
    pub const INDEX_MASK: u32 = 0xff000000; // Top 8 bits
    pub const STEP_MASK: u32 = 0x00fff000; // Middle 12 bits
    pub const TANGENT_MASK: u32 = 0x00000fff; // Bottom 12 bits
}

impl I4Frame {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;

        let index = ((word & Self::INDEX_MASK) >> 24) as u8;
        let step = ((word & Self::STEP_MASK) >> 12) as u8;
        let tangent = (word >> 20) as f32 / 32.0;

        Ok(Self {
            index,
            step,
            tangent,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct I6Frame {
    pub index: f32,
    pub step: f32,
    pub tangent: f32,
}

impl I6Frame {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let index = (reader.read_u16::<BigEndian>()? as f32) / 32.0f32;
        let step = reader.read_u16::<BigEndian>()? as f32;
        let tangent = (reader.read_u16::<BigEndian>()? as f32) / 256.0f32;

        Ok(Self {
            index,
            step,
            tangent,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct I12Frame {
    pub index: f32,
    pub value: f32,
    pub tangent: f32,
}

impl I12Frame {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let index = reader.read_f32::<BigEndian>()?;
        let value = reader.read_f32::<BigEndian>()?;
        let tangent = reader.read_f32::<BigEndian>()?;

        Ok(Self {
            index,
            value,
            tangent,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct I4Animation {
    pub frame_scale: f32,
    pub step: f32,
    pub base: f32,
    pub frames: Vec<I4Frame>,
}

impl I4Animation {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let frame_count = reader.read_u16::<BigEndian>()?;
        tracing::trace!(
            "Reading {frame_count} I4 frames at location {}",
            reader.position()
        );

        let _unknown1 = reader.read_u16::<BigEndian>()?;
        let frame_scale = reader.read_f32::<BigEndian>()?;
        let step = reader.read_f32::<BigEndian>()?;
        let base = reader.read_f32::<BigEndian>()?;

        let mut frames = Vec::with_capacity(frame_count as usize);
        for _ in 0..frame_count {
            frames.push(I4Frame::deserialize(reader)?);
        }

        Ok(Self {
            frame_scale,
            step,
            base,
            frames,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct I6Animation {
    pub frame_scale: f32,
    pub step: f32,
    pub base: f32,
    pub frames: Vec<I6Frame>,
}

impl I6Animation {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let frame_count = reader.read_u16::<BigEndian>()?;
        tracing::trace!(
            "Reading {frame_count} I6 frames at location {}",
            reader.position()
        );

        let _unknown1 = reader.read_u16::<BigEndian>()?;
        let frame_scale = reader.read_f32::<BigEndian>()?;
        let step = reader.read_f32::<BigEndian>()?;
        let base = reader.read_f32::<BigEndian>()?;

        let mut frames = Vec::with_capacity(frame_count as usize);
        for _ in 0..frame_count {
            frames.push(I6Frame::deserialize(reader)?);
        }

        Ok(Self {
            frame_scale,
            step,
            base,
            frames,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct I12Animation {
    pub frame_scale: f32,
    pub frames: Vec<I12Frame>,
}

impl I12Animation {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let frame_count = reader.read_u16::<BigEndian>()?;
        tracing::trace!(
            "Reading {frame_count} I12 frames at location {}",
            reader.position()
        );

        let _unknown1 = reader.read_u16::<BigEndian>()?;
        let frame_scale = reader.read_f32::<BigEndian>()?;

        let mut frames = Vec::with_capacity(frame_count as usize);
        for _ in 0..frame_count {
            frames.push(I12Frame::deserialize(reader)?);
        }

        Ok(Self {
            frame_scale,
            frames,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct L1Animation {
    pub step: f32,
    pub base: f32,
    pub frames: Vec<f32>,
}

impl L1Animation {
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        header_frame_count: u16,
    ) -> SlipstreamResult<Self> {
        tracing::trace!(
            "Reading {header_frame_count} L1 frames at location {}",
            reader.position()
        );

        // reader.set_position(reader.position() - 4);

        let step = reader.read_f32::<BigEndian>()?;
        let base = reader.read_f32::<BigEndian>()?;

        let mut frames = Vec::with_capacity(header_frame_count as usize);
        for _ in 0..header_frame_count {
            frames.push(reader.read_u8()? as f32);
        }

        Ok(Self { step, base, frames })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct L4Animation {
    pub frames: Vec<f32>,
}
