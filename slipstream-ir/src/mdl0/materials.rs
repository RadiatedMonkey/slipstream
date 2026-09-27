use std::ops::ControlFlow;

use bitfield_struct::{bitenum, bitfield};
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    assert_u8,
    cursor::RefCursor,
    error::{CorruptionError, SlipstreamError, SlipstreamResult},
};

use crate::{
    encoding::ReadArrayExt,
    gx::load_bp::{AlphaFunction, BlendMode, ConstantAlpha, DepthTest, LoadBpOpCode},
    mdl0::TextureMatrixMode,
    node::node::IrNodeType,
    section::DeserializeSection,
    visitor::{Visitable, Visitor},
};

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct MaterialFlags {
    pub xlu_material: bool,
    #[bits(23)]
    _padding: u32,
    pub dont_send_tex_matrix: bool,
    pub dont_send_uv: bool,
    pub dont_send_generator_mode: bool,
    pub dont_send_lighting_channel: bool,
    pub dont_send_indirect_matrix: bool,
    pub dont_send_uv_scale: bool,
    pub dont_send_tev_color: bool,
    pub dont_send_pixel_display: bool,
}

impl MaterialFlags {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CullMode {
    None = 0x00,
    Front = 0x01,
    Back = 0x02,
    All = 0x03,
}

impl TryFrom<u32> for CullMode {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::None,
            0x01 => Self::Front,
            0x02 => Self::Back,
            0x03 => Self::All,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid culling mode: {value:#04x} (expected 0-3)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl CullMode {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IndirectMethod {
    Wrap = 0x00,
    NormalMap = 0x01,
    NormalMapSpecular = 0x02,
    Fur = 0x03,
    Reserved1 = 0x04,
    Reserved2 = 0x05,
    User0 = 0x06,
    User1 = 0x07,
}

impl TryFrom<u8> for IndirectMethod {
    type Error = SlipstreamError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Wrap,
            0x01 => Self::NormalMap,
            0x02 => Self::NormalMapSpecular,
            0x03 => Self::Fur,
            0x04 => Self::Reserved1,
            0x05 => Self::Reserved2,
            0x06 => Self::User0,
            0x07 => Self::User1,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid indirect method: {value:#04x} (expected 0-7)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl IndirectMethod {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let byte = reader.read_u8()?;
        Self::try_from(byte)
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct UsedTextureMaps {
    pub tex0_used: bool,
    pub tex1_used: bool,
    pub tex2_used: bool,
    pub tex3_used: bool,
    pub tex4_used: bool,
    pub tex5_used: bool,
    pub tex6_used: bool,
    pub tex7_used: bool,
    #[bits(24)]
    _padding: u32,
}

impl UsedTextureMaps {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct UsedPalettes {
    pub pal0_used: bool,
    pub pal1_used: bool,
    pub pal2_used: bool,
    pub pal3_used: bool,
    pub pal4_used: bool,
    pub pal5_used: bool,
    pub pal6_used: bool,
    pub pal7_used: bool,
    #[bits(24)]
    _padding: u32,
}

impl UsedPalettes {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct LayerSettings {
    pub enable_layer: bool,
    pub scale_fixed: bool,
    pub rotation_fixed: bool,
    pub translation_fixed: bool,
    #[bits(28)]
    _padding: u32,
}

impl LayerSettings {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerCoordinates {
    pub scale_coordinate: [f32; 2],
    pub rotation_coordinate: f32,
    pub translation_coordinate: [f32; 2],
}

impl LayerCoordinates {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let scale_coordinate = reader.read_f32_array::<2, BigEndian>()?;
        let rotation_coordinate = reader.read_f32::<BigEndian>()?;
        let translation_coordinate = reader.read_f32_array::<2, BigEndian>()?;

        Ok(Self {
            scale_coordinate,
            rotation_coordinate,
            translation_coordinate,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TextureMapMode {
    Uv = 0x00,
    EnvCamera = 0x01,
    Projection = 0x02,
    EnvLight = 0x03,
    EnvSpec = 0x04,
}

impl TryFrom<u8> for TextureMapMode {
    type Error = SlipstreamError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Uv,
            0x01 => Self::EnvCamera,
            0x02 => Self::Projection,
            0x03 => Self::EnvLight,
            0x04 => Self::EnvSpec,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid texture map mode: {value} (expected 0-4)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl TextureMapMode {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let byte = reader.read_u8()?;
        Self::try_from(byte)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureMatrixSettings {
    pub scn0_camera_ref: i8,
    pub scn0_light_ref: i8,
    pub map_mode: TextureMapMode,
    pub enable_identity_matrix_effect: bool,
    pub texture_matrix: glam::Mat4,
}

impl TextureMatrixSettings {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let scn0_camera_ref = reader.read_i8()?;
        let scn0_light_ref = reader.read_i8()?;
        let map_mode = TextureMapMode::deserialize(reader)?;
        let enable_identity_matrix_effect = reader.read_u8()? != 0;
        let tm = reader.read_f32_array::<12, BigEndian>()?;

        let texture_matrix = glam::mat4(
            glam::vec4(tm[0], tm[4], tm[8], 0.0),
            glam::vec4(tm[1], tm[5], tm[9], 0.0),
            glam::vec4(tm[2], tm[6], tm[10], 0.0),
            glam::vec4(tm[3], tm[7], tm[11], 1.0),
        );

        Ok(Self {
            scn0_camera_ref,
            scn0_light_ref,
            map_mode,
            enable_identity_matrix_effect,
            texture_matrix,
        })
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct LightingChannelFlags {
    pub material_color_enabled: bool,
    pub material_alpha_enabled: bool,
    pub ambient_color_enabled: bool,
    pub ambient_alpha_enabled: bool,
    pub raster_color_enabled: bool,
    pub raster_alpha_enabled: bool,
    #[bits(26)]
    _padding: u32,
}

impl LightingChannelFlags {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LightChannelSettings {
    pub flags: LightingChannelFlags,
    pub material_color: glam::U8Vec4,
    pub ambient_color: glam::U8Vec4,
    pub color_light_channel_control: u32,
    pub alpha_light_channel_control: u32,
}

impl LightChannelSettings {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let flags = LightingChannelFlags::deserialize(reader)?;
        let material_color = glam::U8Vec4::from_array(reader.read_u8_array::<4>()?);
        let ambient_color = glam::U8Vec4::from_array(reader.read_u8_array::<4>()?);
        let color_light_channel_control = reader.read_u32::<BigEndian>()?;
        let alpha_light_channel_control = reader.read_u32::<BigEndian>()?;

        Ok(Self {
            flags,
            material_color,
            ambient_color,
            color_light_channel_control,
            alpha_light_channel_control,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialMode {
    pub alpha_fn: AlphaFunction,
    pub depth_test: DepthTest,
    pub write_mask: u32,
    pub blend_mode: BlendMode,
    pub constant_alpha: ConstantAlpha,
}

impl MaterialMode {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        assert_u8!(reader, 0x61, "expected LoadBP opcode");
        let LoadBpOpCode::AlphaFunction(alpha_fn) = LoadBpOpCode::deserialize(reader)? else {
            return Err(CorruptionError {
                reason: format!("excepted LoadBP opcode of type AlphaFunction (0xf3)"),
                location: Some(reader.position()),
            }
            .into());
        };

        assert_u8!(reader, 0x61, "expected LoadBP opcode");
        let LoadBpOpCode::DepthTest(depth_test) = LoadBpOpCode::deserialize(reader)? else {
            return Err(CorruptionError {
                reason: format!("excepted LoadBP opcode of type DepthTest (0x40)"),
                location: Some(reader.position()),
            }
            .into());
        };

        assert_u8!(reader, 0x61, "expected LoadBP opcode");
        let LoadBpOpCode::WriteMask(write_mask) = LoadBpOpCode::deserialize(reader)? else {
            return Err(CorruptionError {
                reason: format!("excepted LoadBP opcode of type WriteMask (0xFE)"),
                location: Some(reader.position()),
            }
            .into());
        };

        assert_u8!(reader, 0x61, "expected LoadBP opcode");
        let LoadBpOpCode::BlendMode(blend_mode) = LoadBpOpCode::deserialize(reader)? else {
            return Err(CorruptionError {
                reason: format!("excepted LoadBP opcode of type BlendMode (0x41)"),
                location: Some(reader.position()),
            }
            .into());
        };

        assert_u8!(reader, 0x61, "expected LoadBP opcode");
        let LoadBpOpCode::ConstantAlpha(constant_alpha) = LoadBpOpCode::deserialize(reader)? else {
            return Err(CorruptionError {
                reason: format!("excepted LoadBP opcode of type ConstantAlpha (0x42)"),
                location: Some(reader.position()),
            }
            .into());
        };

        // Then 7 bytes of padding
        reader.set_position(reader.position() + 7);

        Ok(Self {
            alpha_fn,
            depth_test,
            write_mask,
            blend_mode,
            constant_alpha,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MinificationFilter {
    Nearest = 0x00,
    Linear = 0x01,
    NearestMipmapNearest = 0x02,
    LinearMipmapNearest = 0x03,
    NearestMipmapLinear = 0x04,
    LinearMipmapLinear = 0x05,
}

impl TryFrom<u32> for MinificationFilter {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Nearest,
            0x01 => Self::Linear,
            0x02 => Self::NearestMipmapNearest,
            0x03 => Self::LinearMipmapNearest,
            0x04 => Self::NearestMipmapLinear,
            0x05 => Self::LinearMipmapLinear,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid minification filter: {value:#04x} (expected 0-5)"),
                    location: None,
                }
                .into());
            }
        })
    }
}

impl MinificationFilter {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MagnificationFilter {
    Nearest = 0x00,
    Linear = 0x01,
}

impl TryFrom<u32> for MagnificationFilter {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Nearest,
            0x01 => Self::Linear,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid magnification filter: {value:#04x} (expected 0, 1)"),
                    location: None,
                }
                .into());
            }
        })
    }
}

impl MagnificationFilter {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AnisotropyFiltering {
    One = 0x00,
    Two = 0x01,
    Four = 0x02,
}

impl TryFrom<u32> for AnisotropyFiltering {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::One,
            0x01 => Self::Two,
            0x02 => Self::Four,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid anisotropy filtering: {value:#04x} (expected 0-2)"),
                    location: None,
                }
                .into());
            }
        })
    }
}

impl AnisotropyFiltering {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WrapMode {
    Clamp = 0x00,
    Repeat = 0x01,
    Mirror = 0x02,
}

impl TryFrom<u32> for WrapMode {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Clamp,
            0x01 => Self::Repeat,
            0x02 => Self::Mirror,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid wrap mode: {value:#04x} (expected 0-2)"),
                    location: None,
                }
                .into());
            }
        })
    }
}

impl WrapMode {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureReference {
    pub texture_id: i32,
    pub palette_id: i32,
    pub texture_data_id: u32,
    pub palette_data_id: u32,
    pub uwrap: WrapMode,
    pub vwrap: WrapMode,
    pub min_filter: MinificationFilter,
    pub mag_filter: MagnificationFilter,
    pub lod_bias: f32,
    pub max_anisotropy_filtering: AnisotropyFiltering,
    pub clamp_bias: u8,
    pub texel_interpolate: u8,
}

impl TextureReference {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let texture_id = reader.read_i32::<BigEndian>()?;
        let palette_id = reader.read_i32::<BigEndian>()?;
        let _texture_pointer = reader.read_i32::<BigEndian>()?;
        let _palette_pointer = reader.read_i32::<BigEndian>()?;
        let texture_data_id = reader.read_u32::<BigEndian>()?;
        let palette_data_id = reader.read_u32::<BigEndian>()?;
        let uwrap = WrapMode::deserialize(reader)?;
        let vwrap = WrapMode::deserialize(reader)?;
        let min_filter = MinificationFilter::deserialize(reader)?;
        let mag_filter = MagnificationFilter::deserialize(reader)?;
        let lod_bias = reader.read_f32::<BigEndian>()?;
        let max_anisotropy_filtering = AnisotropyFiltering::deserialize(reader)?;
        let clamp_bias = reader.read_u8()?;
        let texel_interpolate = reader.read_u8()?;

        // Skip over padding
        reader.set_position(reader.position() + 2);

        Ok(Self {
            texture_id,
            palette_id,
            texture_data_id,
            palette_data_id,
            uwrap,
            vwrap,
            min_filter,
            mag_filter,
            lod_bias,
            max_anisotropy_filtering,
            clamp_bias,
            texel_interpolate,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialBuffer {
    pub index: u32,
    pub flags: MaterialFlags,
    pub texture_gen_count: u8,
    pub light_channel_count: u8,
    pub shader_stages: u8,
    pub indirect_shader_stages: u8,
    pub cull_mode: CullMode,
    pub depth_test: u8,
    pub light_set_index: i8,
    pub fog_index: i8,
    pub indirect_methods: [IndirectMethod; 4],
    pub light_normal_map_refs: [i8; 4],
    pub shader_offset: i32,
    pub texture_count: u32,
    pub material_ref_offset: i32,
    pub fur_data_offset: i32,
    pub user_data_offset: i32,
    pub material_bytecode: MaterialMode,
    pub used_texture_maps: UsedTextureMaps,
    // pub precompiled_texture_code: [u8; 160],
    pub used_palettes: UsedPalettes,
    // pub precompiled_palette_code: [u8; 160],
    pub layer_settings: LayerSettings,
    pub texture_matrix_mode: TextureMatrixMode,
    pub layer_coordinates: [LayerCoordinates; 8],
    pub texture_matrix_settings: [TextureMatrixSettings; 8],
    pub light_channel_settings: [LightChannelSettings; 2],
    pub texture_references: Vec<TextureReference>,
}

impl Visitable for MaterialBuffer {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_material(self)
    }
}

impl DeserializeSection for MaterialBuffer {
    const NAME: &str = "Materials";
    const KIND: IrNodeType = IrNodeType::Material;

    fn deserialize_section(
        reader: &mut RefCursor<[u8]>,
        _header_start: u64,
    ) -> SlipstreamResult<Self> {
        let material_start = reader.position();
        let _length = reader.read_u32::<BigEndian>()?;
        let _mdl0_offset = reader.read_i32::<BigEndian>()?;
        let _name_offset = reader.read_i32::<BigEndian>()?;
        let index = reader.read_u32::<BigEndian>()?;
        let flags = MaterialFlags::deserialize(reader)?;
        let texture_gen_count = reader.read_u8()?;
        let light_channel_count = reader.read_u8()?;
        let shader_stages = reader.read_u8()?;
        let indirect_shader_stages = reader.read_u8()?;
        let cull_mode = CullMode::deserialize(reader)?;
        let depth_test = reader.read_u8()?;
        let light_set_index = reader.read_i8()?;
        let fog_index = reader.read_i8()?;
        let _padding = reader.read_u8()?;
        let indirect_methods = [
            IndirectMethod::deserialize(reader)?,
            IndirectMethod::deserialize(reader)?,
            IndirectMethod::deserialize(reader)?,
            IndirectMethod::deserialize(reader)?,
        ];
        let light_normal_map_refs = reader.read_i8_array::<4>()?;
        let shader_offset = reader.read_i32::<BigEndian>()?;
        let texture_count = reader.read_u32::<BigEndian>()?;
        let material_ref_offset = reader.read_i32::<BigEndian>()?;
        let fur_data_offset = reader.read_i32::<BigEndian>()?;
        let user_data_offset = reader.read_i32::<BigEndian>()?;
        let mode_offset = reader.read_i32::<BigEndian>()?; // does not exist in v9 MDL0 or lower.

        let used_texture_maps = UsedTextureMaps::deserialize(reader)?;
        let _precompiled_texture_code = reader.read_u8_array::<160>()?;
        let used_palettes = UsedPalettes::deserialize(reader)?;
        let _precompiled_palette_code = reader.read_u8_array::<160>()?;
        let layer_settings = LayerSettings::deserialize(reader)?;
        let texture_matrix_mode = TextureMatrixMode::deserialize(reader)?;
        let light_channel_settings = [
            LightChannelSettings::deserialize(reader)?,
            LightChannelSettings::deserialize(reader)?,
        ];
        let layer_coordinates = [
            LayerCoordinates::deserialize(reader)?,
            LayerCoordinates::deserialize(reader)?,
            LayerCoordinates::deserialize(reader)?,
            LayerCoordinates::deserialize(reader)?,
            LayerCoordinates::deserialize(reader)?,
            LayerCoordinates::deserialize(reader)?,
            LayerCoordinates::deserialize(reader)?,
            LayerCoordinates::deserialize(reader)?,
        ];

        let texture_matrix_settings = [
            TextureMatrixSettings::deserialize(reader)?,
            TextureMatrixSettings::deserialize(reader)?,
            TextureMatrixSettings::deserialize(reader)?,
            TextureMatrixSettings::deserialize(reader)?,
            TextureMatrixSettings::deserialize(reader)?,
            TextureMatrixSettings::deserialize(reader)?,
            TextureMatrixSettings::deserialize(reader)?,
            TextureMatrixSettings::deserialize(reader)?,
        ];

        let material_bytecode = {
            let mode_start = material_start as i64 + mode_offset as i64;
            reader.set_position(mode_start as u64);

            MaterialMode::deserialize(reader)?
        };

        let texture_references = {
            let ref_start = material_start as i64 + material_ref_offset as i64;
            reader.set_position(ref_start as u64);

            let mut refs = Vec::with_capacity(texture_count as usize);
            for _ in 0..texture_count {
                refs.push(TextureReference::deserialize(reader)?);
            }

            refs
        };

        tracing::error!("TODO SHADER DATA");

        Ok(Self {
            index,
            flags,
            texture_gen_count,
            light_channel_count,
            shader_stages,
            indirect_shader_stages,
            cull_mode,
            depth_test,
            light_set_index,
            fog_index,
            indirect_methods,
            light_normal_map_refs,
            shader_offset,
            texture_count,
            material_ref_offset,
            fur_data_offset,
            user_data_offset,
            material_bytecode,
            used_texture_maps,
            used_palettes,
            layer_settings,
            texture_matrix_mode,
            layer_coordinates,
            texture_matrix_settings,
            light_channel_settings,

            texture_references,
        })
    }
}
