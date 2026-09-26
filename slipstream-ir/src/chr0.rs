use bitfield_struct::{bitenum, bitfield};
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, SlipstreamError, SlipstreamResult},
};

use crate::{
    brres::{BFile, BFileHeader, BFileType},
    index::IndexGroup,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnimationPolicy {
    OneTime,
    Loop,
}

impl TryFrom<u32> for AnimationPolicy {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::OneTime,
            0x01 => Self::Loop,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid animation policy: {value} (expected 0 or 1)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl AnimationPolicy {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let policy = reader.read_u32::<BigEndian>()?;
        Self::try_from(policy)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ScalingRule {
    Standard,
    Softimage,
    Maya,
}

impl TryFrom<u32> for ScalingRule {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Standard,
            0x01 => Self::Softimage,
            0x02 => Self::Maya,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid scaling rule: {} (expected 0, 1 or 2)", value),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl ScalingRule {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let rule = reader.read_u32::<BigEndian>()?;
        Self::try_from(rule)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chr0Header {
    /// The amount of animation frames stored in this file.
    pub frame_count: u16,
    pub anim_data_count: u16,
    /// Whether the animation loops or is a one time animation.
    pub anim_policy: AnimationPolicy,
    pub scaling_rule: ScalingRule,
}

impl Chr0Header {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let frame_count = reader.read_u16::<BigEndian>()?;
        let anim_data_count = reader.read_u16::<BigEndian>()?;
        let anim_policy = AnimationPolicy::deserialize(reader)?;
        let scaling_rule = ScalingRule::deserialize(reader)?;

        Ok(Self {
            frame_count,
            anim_data_count,
            anim_policy,
            scaling_rule,
        })
    }
}

/// The format used to store the animation.
///
/// This affects the quality and behavior of the animation.
#[bitenum]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum AnimationFormat {
    /// The bone is kept in a fixed place. Fixed animations are simply a single value indicating where to
    /// put the bone.
    Fixed = 0b000,
    /// A 4-byte format where the animation is evaluated by smoothly interpolating between
    /// a few explicitly defined keyframes. Each defined keyframe specifies its tangent to create
    /// smooth curves between frames.
    Interpolated4 = 0b001,
    /// A 6-byte format where the animation is evaluated by smoothly interpolating between
    /// a few explicitly defined keyframes. Each defined keyframe specifies its tangent to create
    /// smooth curves between frames.
    Interpolated6 = 0b010,
    /// A 12-byte format where the animation is evaluated by smoothly interpolating between
    /// a few explicitly defined keyframes. Each defined keyframe specifies its tangent to create
    /// smooth curves between frames.
    Interpolated12 = 0b011,
    /// Specifies every single keyframe explicitly, but allows for more erratic information instead of the smoothly
    /// interpolated animations of interpolation formats.
    ///
    /// The frame are 1 byte in size.
    Linear1 = 0b100,
    /// Specifies every single keyframe explicitly, but allows for more erratic information instead of the smoothly
    /// interpolated animations of interpolation formats.
    ///
    /// The frame are 4 bytes in size.
    Linear4 = 0b110,
    /// A fallback for [`bitenum`], this variant should never be used.
    #[fallback]
    Invalid,
}

impl AnimationFormat {
    /// Whether this format is a linear format.
    pub fn is_linear(&self) -> bool {
        const LINEAR_MASK: u8 = 0b100;
        (*self as u8) & LINEAR_MASK != 0
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct AnimationCode {
    #[bits(1)]
    _unused: bool,
    pub use_identity: bool,
    pub rotation_translation_isotropic: bool,
    pub scale_isotropic: bool,
    pub scale_uniform: bool,
    pub rotation_isotropic: bool,
    pub translation_isotropic: bool,
    pub use_model_scale: bool,
    pub use_model_rotation: bool,
    pub use_model_translation: bool,
    pub apply_scale_compensate: bool,
    pub apply_child_scale_compensate: bool,
    pub disable_classic_scale: bool,
    pub scale_x_fixed: bool,
    pub scale_y_fixed: bool,
    pub scale_z_fixed: bool,
    pub rotation_x_fixed: bool,
    pub rotation_y_fixed: bool,
    pub rotation_z_fixed: bool,
    pub x_fixed: bool,
    pub y_fixed: bool,
    pub z_fixed: bool,
    pub has_scale: bool,
    pub has_rotation: bool,
    pub has_translation: bool,
    #[bits(2)]
    pub scale_format: AnimationFormat,
    #[bits(3)]
    pub rotation_format: AnimationFormat,
    #[bits(2)]
    pub translation_format: AnimationFormat,
}

impl AnimationCode {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}

/// The type of animation that is applied to the bone.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentType {
    /// The bone stays in place throughout the entire animation.
    Fixed(f32),
    /// The bone is animated.
    Animated(AnimationType),
}

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
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
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
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
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
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
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
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
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
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
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
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
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

#[derive(Debug, Clone, PartialEq)]
pub enum AnimationType {
    Interpolated4(I4Animation),
    Interpolated6(I6Animation),
    Interpolated12(I12Animation),
    Linear1(L1Animation),
    Linear4(L4Animation),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentData {
    pub x: ComponentType,
    pub y: ComponentType,
    pub z: ComponentType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationData {
    pub scale: Option<ComponentData>,
    pub rotation: Option<ComponentData>,
    pub translation: Option<ComponentData>,
}

impl AnimationData {
    /// Deserializes the current keyframe.
    fn deserialize_key_frame(
        reader: &mut RefCursor<[u8]>,
        bone_data_start: u32,
        header_frame_count: u16,
        format: AnimationFormat,
    ) -> SlipstreamResult<AnimationType> {
        let orig_position = reader.position();

        let frame_offset = reader.read_i32::<BigEndian>()? as i64;
        let frame_start = bone_data_start as i64 + frame_offset;
        reader.set_position(frame_start as u64);

        let frames = match format {
            AnimationFormat::Interpolated4 => {
                AnimationType::Interpolated4(I4Animation::deserialize(reader)?)
            }
            AnimationFormat::Interpolated6 => {
                AnimationType::Interpolated6(I6Animation::deserialize(reader)?)
            }
            AnimationFormat::Interpolated12 => {
                AnimationType::Interpolated12(I12Animation::deserialize(reader)?)
            }
            AnimationFormat::Linear1 => {
                // Does Brawlcrate simply just display them differently?
                tracing::error!("FIXME: LINEAR1 ANIMATIONS DO NOT WORK PROPERLY RIGHT NOW");
                AnimationType::Linear1(L1Animation::deserialize(reader, header_frame_count)?)
            }
            _ => todo!("animation frame format {format:?}"),
        };

        reader.set_position(orig_position + 4);
        Ok(frames)
    }

    /// Returns the scale data of the current keyframe.
    fn deserialize_scale(
        reader: &mut RefCursor<[u8]>,
        bone_data_start: u32,
        header_frame_count: u16,
        anim_ty_code: &AnimationCode,
    ) -> SlipstreamResult<ComponentData> {
        tracing::trace!(
            "Reading scale animations (iso: {}, x fixed: {}, y fixed: {}, z fixed: {})",
            anim_ty_code.scale_isotropic(),
            anim_ty_code.scale_x_fixed(),
            anim_ty_code.scale_y_fixed(),
            anim_ty_code.scale_z_fixed()
        );

        if anim_ty_code.scale_isotropic() {
            let iso_scale;

            // Only one piece of data is stored, rather than for each component
            if anim_ty_code.scale_x_fixed() {
                iso_scale = ComponentType::Fixed(reader.read_f32::<BigEndian>()?);
            } else {
                let frame = Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_ty_code.scale_format(),
                )?;
                iso_scale = ComponentType::Animated(frame);
            }

            Ok(ComponentData {
                x: iso_scale.clone(),
                y: iso_scale.clone(),
                z: iso_scale,
            })
        } else {
            let x_scale;
            if anim_ty_code.scale_x_fixed() {
                x_scale = ComponentType::Fixed(reader.read_f32::<BigEndian>()?);
            } else {
                let frame = Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_ty_code.scale_format(),
                )?;
                x_scale = ComponentType::Animated(frame);
            }

            let y_scale;
            if anim_ty_code.scale_y_fixed() {
                y_scale = ComponentType::Fixed(reader.read_f32::<BigEndian>()?);
            } else {
                let frame = Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_ty_code.scale_format(),
                )?;
                y_scale = ComponentType::Animated(frame);
            }

            let z_scale;
            if anim_ty_code.scale_z_fixed() {
                z_scale = ComponentType::Fixed(reader.read_f32::<BigEndian>()?);
            } else {
                let frame = Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_ty_code.scale_format(),
                )?;
                z_scale = ComponentType::Animated(frame);
            }

            Ok(ComponentData {
                x: x_scale,
                y: y_scale,
                z: z_scale,
            })
        }
    }

    /// Reads the rotation data of the current keyframe.
    fn deserialize_rotation(
        reader: &mut RefCursor<[u8]>,
        bone_data_start: u32,
        header_frame_count: u16,
        anim_code: &AnimationCode,
    ) -> SlipstreamResult<ComponentData> {
        tracing::trace!(
            "Reading rotation animations (iso: {}, x fixed: {}, y fixed: {}, z fixed: {})",
            anim_code.rotation_isotropic(),
            anim_code.rotation_x_fixed(),
            anim_code.rotation_y_fixed(),
            anim_code.rotation_z_fixed()
        );

        if anim_code.rotation_isotropic() {
            let iso_rot = if anim_code.rotation_x_fixed() {
                ComponentType::Fixed(reader.read_f32::<BigEndian>()?)
            } else {
                let frame = Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_code.rotation_format(),
                )?;
                ComponentType::Animated(frame)
            };

            Ok(ComponentData {
                x: iso_rot.clone(),
                y: iso_rot.clone(),
                z: iso_rot,
            })
        } else {
            let x_rot = if anim_code.rotation_x_fixed() {
                ComponentType::Fixed(reader.read_f32::<BigEndian>()?)
            } else {
                let frame = Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_code.rotation_format(),
                )?;
                ComponentType::Animated(frame)
            };

            let y_rot = if anim_code.rotation_y_fixed() {
                ComponentType::Fixed(reader.read_f32::<BigEndian>()?)
            } else {
                let frame = Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_code.rotation_format(),
                )?;
                ComponentType::Animated(frame)
            };

            let z_rot = if anim_code.rotation_z_fixed() {
                ComponentType::Fixed(reader.read_f32::<BigEndian>()?)
            } else {
                let frame = Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_code.rotation_format(),
                )?;
                ComponentType::Animated(frame)
            };

            Ok(ComponentData {
                x: x_rot,
                y: y_rot,
                z: z_rot,
            })
        }
    }

    /// Reads the translation data of the current keyframe.
    fn deserialize_translation(
        reader: &mut RefCursor<[u8]>,
        bone_data_start: u32,
        header_frame_count: u16,
        anim_code: &AnimationCode,
    ) -> SlipstreamResult<ComponentData> {
        tracing::trace!(
            "Reading translation animations (iso: {}, x fixed: {}, y fixed: {}, z fixed: {})",
            anim_code.translation_isotropic(),
            anim_code.x_fixed(),
            anim_code.y_fixed(),
            anim_code.z_fixed()
        );

        if anim_code.translation_isotropic() {
            let iso_trans = if anim_code.x_fixed() {
                ComponentType::Fixed(reader.read_f32::<BigEndian>()?)
            } else {
                ComponentType::Animated(Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_code.translation_format(),
                )?)
            };

            Ok(ComponentData {
                x: iso_trans.clone(),
                y: iso_trans.clone(),
                z: iso_trans,
            })
        } else {
            let trans_x = if anim_code.x_fixed() {
                ComponentType::Fixed(reader.read_f32::<BigEndian>()?)
            } else {
                ComponentType::Animated(Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_code.translation_format(),
                )?)
            };

            let trans_y = if anim_code.y_fixed() {
                ComponentType::Fixed(reader.read_f32::<BigEndian>()?)
            } else {
                ComponentType::Animated(Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_code.translation_format(),
                )?)
            };

            let trans_z = if anim_code.z_fixed() {
                ComponentType::Fixed(reader.read_f32::<BigEndian>()?)
            } else {
                ComponentType::Animated(Self::deserialize_key_frame(
                    reader,
                    bone_data_start,
                    header_frame_count,
                    anim_code.translation_format(),
                )?)
            };

            Ok(ComponentData {
                x: trans_x,
                y: trans_y,
                z: trans_z,
            })
        }
    }

    /// Deserializes all (translation, rotation and scale) components of a single keyframe.
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        bone_data_start: u32,
        header_frame_count: u16,
        anim_code: &AnimationCode,
    ) -> SlipstreamResult<Self> {
        let scale = if anim_code.has_scale() {
            Some(Self::deserialize_scale(
                reader,
                bone_data_start,
                header_frame_count,
                anim_code,
            )?)
        } else {
            None
        };

        let rotation = if anim_code.has_rotation() {
            Some(Self::deserialize_rotation(
                reader,
                bone_data_start,
                header_frame_count,
                anim_code,
            )?)
        } else {
            None
        };

        let translation = if anim_code.has_translation() {
            Some(Self::deserialize_translation(
                reader,
                bone_data_start,
                header_frame_count,
                anim_code,
            )?)
        } else {
            None
        };

        // If a component is fixed, the current float is simply the value for the bone.
        Ok(AnimationData {
            scale,
            rotation,
            translation,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedBone {
    /// Name of the bone that this animates.
    pub name: String,
    pub anim_code: AnimationCode,
    pub anim_data: AnimationData,
}

impl AnimatedBone {
    #[tracing::instrument(skip(reader, bone_data_start, header_frame_count))]
    pub fn deserialize(
        reader: &mut RefCursor<[u8]>,
        bone_data_start: u32,
        header_frame_count: u16,
        name: String,
    ) -> SlipstreamResult<Self> {
        // Points to the same string as the file name in the index group entry,
        // so we don't need it.
        let _bone_name_offset = reader.read_u32::<BigEndian>()?;
        let anim_code = AnimationCode::deserialize(reader)?;
        let anim_data =
            AnimationData::deserialize(reader, bone_data_start, header_frame_count, &anim_code)?;

        Ok(Self {
            name: name.to_owned(),
            anim_code,
            // anim_flags,
            anim_data,
        })
    }
}

/// A CHR0 subfile stores character model animations.
///
/// These animations are based on the bones of the character.
#[derive(Debug, Clone, PartialEq)]
pub struct Chr0Subfile {
    /// General header for BRRES subfiles.
    pub subfile_header: BFileHeader,
    /// CHR0-specific header data.
    pub chr0_header: Chr0Header,
    /// Per-bone animation data.
    pub bones: Vec<AnimatedBone>,
    /// This index group lists all the individual bones in the CHR0 file.
    pub bones_group: IndexGroup,
}

impl Chr0Subfile {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let subfile_header = BFileHeader::deserialize(reader, BFileType::Chr0)?;

        reader.set_position(reader.position() + 4); // there are 4 bytes of padding between the headers
        let chr0_header = Chr0Header::deserialize(reader)?;

        // This subgroup references all the bones in the animation file.
        let bones_group = IndexGroup::deserialize(reader)?;
        let mut bones = Vec::with_capacity(bones_group.entries.len());

        for entry in &bones_group.entries[1..] {
            let name = bones_group.get_entry_name(reader, entry)?.to_owned();

            let data_start = bones_group.get_entry_data_start(entry);
            reader.set_position(data_start as u64);

            tracing::trace!("Reading CHR0 animations for bone `{name}` at location {data_start}");
            bones.push(AnimatedBone::deserialize(
                reader,
                data_start,
                chr0_header.frame_count,
                name,
            )?);
        }

        Ok(Self {
            subfile_header,
            chr0_header,
            bones_group,
            bones,
        })
    }
}

impl BFile for Chr0Subfile {
    /// The first 4 bytes of a CHR0 file: "CHR0"
    const MAGIC: [u8; 4] = [0x43, 0x48, 0x52, 0x30];
}
