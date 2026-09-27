pub mod bones;
pub mod colors;
pub mod definitions;
pub mod materials;
pub mod normals;
pub mod pal_links;
pub mod polygon;
pub mod tevs;
pub mod tex_links;
pub mod uvs;
pub mod vertices;

use crate::brres::{self, BFileHeader, BFileType};
use crate::encoding::ReadArrayExt;
use crate::node::arena::{IrArena, IrNodeKey};
use crate::node::defer::Deferred;
use crate::node::node::{IrNode, IrNodeType};
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::cursor::RefCursor;
use slipstream_shared::error::{
    CorruptionError, SlipstreamError, SlipstreamResult, UnsupportedError,
};

pub const MDL0_MAGIC: [u8; 4] = [0x4d, 0x44, 0x4c, 0x30]; // "MDL0"

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalingMode {
    Standard,
    Softimage,
    Maya,
}

impl TryFrom<u32> for ScalingMode {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Standard,
            1 => Self::Softimage,
            2 => Self::Maya,
            v => {
                return Err(CorruptionError {
                    reason: format!("invalid scaling mode: {v} (except 0, 1, 2)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl ScalingMode {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureMatrixMode {
    Maya,
    Xsi,
    ThreeDsMax,
}

impl TryFrom<u32> for TextureMatrixMode {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Maya,
            1 => Self::Xsi,
            2 => Self::ThreeDsMax,
            v => {
                return Err(CorruptionError {
                    reason: format!("invalid texture matrix mode: {v} (expected 0, 1, 2)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl TextureMatrixMode {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

pub const MDL0_SECTION_NAMES: &[&str] = &[
    "Bytecode",
    "Bones",
    "Vertices",
    "Normals",
    "Colors",
    "UVs",
    "Fur vectors",
    "Fur layers",
    "Materials",
    "TEVs",
    "Shapes",
    "Texture links",
    "Palette links",
    "User data",
];

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SectionType {
    DrawLists,
    Bones,
    Vertices,
    Normals,
    Colors,
    UvCoordinates,
    FurVectors,
    FurLayers,
    Materials,
    Tevs,
    Shapes,
    TextureLinks,
    PaletteLinks,
    UserData,
}

impl TryFrom<u32> for SectionType {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::DrawLists,
            1 => Self::Bones,
            2 => Self::Vertices,
            3 => Self::Normals,
            4 => Self::Colors,
            5 => Self::UvCoordinates,
            6 => Self::FurVectors,
            7 => Self::FurLayers,
            8 => Self::Materials,
            9 => Self::Tevs,
            10 => Self::Shapes,
            11 => Self::TextureLinks,
            12 => Self::PaletteLinks,
            13 => Self::UserData,
            v => {
                return Err(CorruptionError {
                    reason: format!("invalid MDL0 section ID: {v} (expected 0-13)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

pub trait SectionDeserialize: Sized {
    fn deserialize_section(
        reader: &mut RefCursor<[u8]>,
        header_start: u32,
    ) -> SlipstreamResult<Self>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mdl0Header {
    pub file_header_offset: i32,
    pub scaling_mode: ScalingMode,
    pub texture_matrix_mode: TextureMatrixMode,
    pub vertex_count: i32,
    pub face_count: i32,
    pub matrix_count: u32,
    pub require_normalized_matrix_array: bool,
    pub require_texture_matrix_array: bool,
    pub enable_bounding_volume_data: bool,
    pub matrix_table_offset: i32,
    pub bounding_volume_minimum: [f32; 3],
    pub bounding_volume_maximum: [f32; 3],
}

impl Mdl0Header {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let start = reader.position();

        let header_length = reader.read_u32::<BigEndian>()?;
        let file_header_offset = reader.read_i32::<BigEndian>()?;
        let scaling_mode = ScalingMode::deserialize(reader)?;
        let texture_matrix_mode = TextureMatrixMode::deserialize(reader)?;
        let vertex_count = reader.read_i32::<BigEndian>()?;
        let face_count = reader.read_i32::<BigEndian>()?;
        let _unused1 = reader.read_i32::<BigEndian>()?;
        let matrix_count = reader.read_u32::<BigEndian>()?;
        let require_normalized_matrix_array = reader.read_u8()? != 0;
        let require_texture_matrix_array = reader.read_u8()? != 0;
        let enable_bounding_volume_data = reader.read_u8()? != 0;
        let _unknown1 = reader.read_u8()?;
        let matrix_table_offset = reader.read_i32::<BigEndian>()?;
        let bounding_volume_minimum = reader.read_f32_array::<3, BigEndian>()?;
        let bounding_volume_maximum = reader.read_f32_array::<3, BigEndian>()?;

        reader.set_position(start + header_length as u64);

        Ok(Self {
            file_header_offset,
            scaling_mode,
            texture_matrix_mode,
            vertex_count,
            face_count,
            matrix_count,
            require_normalized_matrix_array,
            require_texture_matrix_array,
            enable_bounding_volume_data,
            matrix_table_offset,
            bounding_volume_minimum,
            bounding_volume_maximum,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoneLinkTable {
    /// Maps a matrix index to the singular bone index driving it.
    pub rigid_bones: Vec<u32>,
    /// Indices of matrices that have multiple bones affecting them.
    pub mixed_bones: Vec<u32>,
    /// Indices of bones that do not deform any geometry.
    pub unconnected_bones: Vec<u32>,
}

impl BoneLinkTable {
    pub fn len(&self) -> usize {
        self.rigid_bones.len() + self.mixed_bones.len() + self.unconnected_bones.len()
    }
}

impl BoneLinkTable {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let entry_count = reader.read_u32::<BigEndian>()?;

        let mut rigid = Vec::new();
        let mut mixed = Vec::new();
        let mut unconnected = Vec::new();

        let mut is_mixed = false;
        for i in 0..entry_count {
            let word = reader.read_u32::<BigEndian>()?;
            if word == 0xFFFFFFFF {
                mixed.push(i);
                is_mixed = true;
            } else {
                if is_mixed {
                    unconnected.push(i);
                } else {
                    rigid.push(i);
                }
            }
        }

        Ok(Self {
            rigid_bones: rigid,
            mixed_bones: mixed,
            unconnected_bones: unconnected,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub bone_link_table: BoneLinkTable,
}

#[tracing::instrument(skip_all, fields(name, parent_id))]
pub fn deserialize(
    reader: &mut RefCursor<[u8]>,
    parent_id: IrNodeKey,
    arena: &IrArena,
    name: String,
) -> SlipstreamResult<IrNodeKey> {
    tracing::trace!("Opening {name}");

    let subfile_header = BFileHeader::deserialize(reader, BFileType::Mdl0)?;
    if subfile_header.subfile_version != 11 {
        return Err(UnsupportedError {
            reason: format!(
                "MDL0 version {} is not supported, only version 11 is",
                subfile_header.subfile_version
            ),
            location: Some(reader.position()),
        }
        .into());
    }

    let expected_sections =
        brres::get_section_count(BFileType::Mdl0, subfile_header.subfile_version)?;

    if subfile_header.offsets.len() != expected_sections {
        return Err(CorruptionError {
            reason: format!(
                "invalid section count, expected {}, got {}",
                expected_sections,
                subfile_header.offsets.len()
            ),
            location: Some(reader.position()),
        }
        .into());
    }

    let _mdl0_header = Mdl0Header::deserialize(reader)?;

    let bone_link_table = BoneLinkTable::deserialize(reader)?;
    let mdl_node_id = arena.next_id();

    let mut files = Vec::with_capacity(subfile_header.offsets.len());
    for (i, &section_offset) in subfile_header.offsets.iter().enumerate() {
        // Loops over sections like `Bones`, `Vertices`, `Normals`...

        if section_offset == 0 {
            // Section does not exist, skip it
            continue;
        }

        let section_ty = SectionType::try_from(i as u32)?;

        let mut reader = reader.clone();
        let arena2 = arena.clone();

        let section_id = arena.next_id();
        let section_start = subfile_header.header_start as i64 + section_offset as i64;
        reader.set_position(section_start as u64);

        tracing::debug!("Parsing {section_ty:?}");

        let node_body = match section_ty {
            SectionType::DrawLists => {
                definitions::deserialize_virtual(&mut reader, parent_id, &arena2)
            }
            SectionType::Bones => bones::deserialize_skeleton(&mut reader, parent_id, &arena2),
            SectionType::Vertices => vertices::deserialize_virtual(
                &mut reader,
                subfile_header.header_start,
                parent_id,
                &arena2,
            ),
            SectionType::Normals => normals::deserialize_normals_section(
                &mut reader,
                subfile_header.header_start,
                parent_id,
                &arena2,
            ),
            SectionType::Colors => colors::deserialize_virtual(&mut reader, parent_id, &arena2),
            SectionType::UvCoordinates => uvs::deserialize_virtual(
                &mut reader,
                subfile_header.header_start,
                parent_id,
                &arena2,
            ),
            SectionType::Materials => materials::deserialize_virtual(
                &mut reader,
                subfile_header.header_start,
                parent_id,
                &arena2,
            ),
            SectionType::Tevs => tevs::deserialize_virtual(&mut reader, parent_id, &arena2),
            SectionType::Shapes => polygon::deserialize_virtual(
                &mut reader,
                subfile_header.header_start,
                parent_id,
                &arena2,
            ),
            SectionType::TextureLinks => {
                tex_links::deserialize_virtual(&mut reader, parent_id, &arena2)
            }
            _ => Ok(VirtualNodeBody {
                children: Vec::new(),
                inspectable: None,
            }),
        }?;

        let node = IrNode::from(IrNode {
            label: MDL0_SECTION_NAMES[i].to_owned(),
            key: section_id,
            parent: Some(mdl_node_id),
            ty: IrNodeType::BrresDirectory,
            body: Deferred::evaluated(node_body),
        });

        arena.insert(section_id, node);
        files.push(section_id);
    }

    let node = IrNode::from(IrNode {
        label: name,
        key: mdl_node_id,
        parent: Some(parent_id),
        ty: IrNodeType::Mdl0Root,
        body: Deferred::evaluated(VirtualNodeBody {
            children: files,
            inspectable: Some(Box::new(Model { bone_link_table })),
        }),
    });

    arena.insert(mdl_node_id, node);
    Ok(mdl_node_id)
}
