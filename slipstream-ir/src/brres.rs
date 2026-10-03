use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use slipstream_shared::cursor::{MutCursor, RefCursor};
use slipstream_shared::error::{
    CorruptionError, IncorrectFormat, RangeError, SlipstreamError, SlipstreamResult,
    UnsupportedError,
};

use crate::arc::UnknownFile;
use crate::encoding::{ReadArrayExt, WriteArrayExt};
use crate::index::IndexGroup;
use crate::mdl0::{self, MDL0_MAGIC};
use crate::node::arena::{IrArena, IrNodeDescriptor, IrNodeKey};
use crate::node::node::IrNodeType::Mdl0Root;
use crate::node::node::{ContentSlot, IrNode, IrNodeType};
use crate::tex0::{self, TEX0_MAGIC, Texture};
use crate::visitor::{Visitor, VisitorContext, VisitorContextNode};

/// Equals "bres". This is always at the start of a BRRES file.
pub const BRRES_MAGIC: [u8; 4] = [0x62, 0x72, 0x65, 0x73];

const LE_BOM: [u8; 2] = [0xFF, 0xFE];
const BE_BOM: [u8; 2] = [0xFE, 0xFF];

/// Returns the amount of sections a subfile has, which depends on the subfile type and version.
///
/// This info comes from [`BRRES Subfiles (File Format)`](https://mkwiiki.org/wiki/BRRES_Subfiles_(File_Format))
pub fn get_section_count(ty: BFileType, version: u32) -> SlipstreamResult<usize> {
    Ok(match ty {
        BFileType::Root => 0,
        BFileType::Mdl0 => match version {
            8 => 11,
            11 => 14,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid MDL0 version: {version} (must be 8, 11)"),
                    ..Default::default()
                }
                .into());
            }
        },
        BFileType::Tex0 => match version {
            1 => 1,
            2 => 2,
            3 => 1,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid TEX0 version: {version} (must be 1, 2 or 3)"),
                    ..Default::default()
                }
                .into());
            }
        },
        BFileType::Chr0 => match version {
            // 3 => 1,
            3 => {
                return Err(UnsupportedError {
                    reason: "CHR0 version 3".to_owned(),
                    ..Default::default()
                }
                .into());
            }
            5 => 2,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid CHR0 version: {version} (must be 3, 5)"),
                    ..Default::default()
                }
                .into());
            }
        },
        BFileType::Pat0 => match version {
            4 => 6,
            _ => {
                return Err(CorruptionError {
                    reason: format!("invalid PAT0 version: {version} (must be 4)"),
                    ..Default::default()
                }
                .into());
            }
        },
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BrresHeader {
    pub start: u64,
    pub size: u32,
    pub root_offset: u16,
    pub section_count: u16,
}

impl BrresHeader {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let start = reader.position();

        let magic = reader.read_u8_array::<4>()?;
        if magic != BRRES_MAGIC {
            return Err(IncorrectFormat {
                expected_magic: BRRES_MAGIC.to_vec(),
                found_magic: magic.to_vec(),
                location: Some(reader.position()),
            }
            .into());
        }

        let is_be = match reader.read_u8_array::<2>()? {
            BE_BOM => true,
            LE_BOM => false,
            bom => {
                return Err(CorruptionError {
                    reason: format!(
                        "byte order mark is incorrect, expected FEFF or FFFE, found {bom:x?}"
                    ),
                    ..Default::default()
                }
                .into());
            }
        };

        if !is_be {
            return Err(UnsupportedError {
                reason: "little endian brres files are not supported".to_owned(),
                ..Default::default()
            }
            .into());
        }

        let _padding = reader.read_u16::<BigEndian>()?;
        let size = reader.read_u32::<BigEndian>()?;
        let root_offset = reader.read_u16::<BigEndian>()?;
        let section_count = reader.read_u16::<BigEndian>()?;

        Ok(Self {
            start,
            root_offset,
            size,
            section_count,
        })
    }

    pub fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.write_u8_array(BRRES_MAGIC)?;
        // Our files are always big endian
        writer.write_u8_array(BE_BOM)?;
        writer.write_u16::<BigEndian>(0)?; // padding
        writer.write_u32::<BigEndian>(self.size)?;
        writer.write_u16::<BigEndian>(self.root_offset)?;
        writer.write_u16::<BigEndian>(self.section_count)?;

        Ok(())
    }
}

/// Files in a BRRES archive.
///
/// In the code these are referred to as `BFiles` simply to avoid confusion with files in an
/// ARC archive, general files or even sections of models.
pub trait BFile {
    /// The magic of the given bfile.
    const MAGIC: [u8; 4];
}

/// The first subfile in a BRRES file.
///
/// This only contains the total size of the BRRES file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootSection {
    /// Size of the entire BRRES file.
    pub size: u32,
}

impl RootSection {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let magic = reader.read_u8_array::<4>()?;
        if magic != Self::MAGIC {
            return Err(IncorrectFormat {
                expected_magic: Self::MAGIC.to_vec(),
                found_magic: magic.to_vec(),
                location: Some(reader.position()),
            }
            .into());
        }

        Ok(RootSection {
            size: reader.read_u32::<BigEndian>()?,
        })
    }

    pub fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.write_u8_array(Self::MAGIC)?;
        writer.write_u32::<BigEndian>(self.size)?;
        Ok(())
    }
}

impl BFile for RootSection {
    /// Magic of the root subsection: `root`.
    const MAGIC: [u8; 4] = [0x72, 0x6f, 0x6f, 0x74]; // "root"
}

/// The header of a BRRES subfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BFileHeader {
    /// Start position of this header. This is used to compute subfile section positions using their
    /// offsets.
    pub header_start: u32,
    /// Length of this subfile.
    pub subfile_length: u32,
    /// Version of ths subfile. For MDL0 this is either 8 or 11.
    pub subfile_version: u32,
    /// The offset to the outer BRRES file.
    pub brres_offset: i32,
    /// Offsets within this BRRES file. The number of offsets is implied by the version.
    /// The number can be obtained using the [`get_section_count`] function.
    pub offsets: Vec<i32>,
    /// String offset to the name of this subfile.
    /// This offset is relative to [`header_start`](Self::header_start).
    ///
    /// Note that the offset points to the start of the string data, the length prefix is 4 bytes ahead of it.
    pub name_offset: i32,
}

impl BFileHeader {
    /// Deserializes a section of the given type.
    pub fn deserialize(reader: &mut RefCursor<[u8]>, ty: BFileType) -> SlipstreamResult<Self> {
        let header_start = reader.position() as u32 - 4; // Subtract 4 for magic.
        let subfile_length = reader.read_u32::<BigEndian>()?;
        let subfile_version = reader.read_u32::<BigEndian>()?;
        let brres_offset = reader.read_i32::<BigEndian>()?;

        let section_count = get_section_count(ty, subfile_version)?;

        let mut offsets = Vec::with_capacity(section_count);
        for _ in 0..section_count {
            offsets.push(reader.read_i32::<BigEndian>()?);
        }

        let name_offset = reader.read_i32::<BigEndian>()?;

        Ok(Self {
            header_start,
            subfile_length,
            subfile_version,
            brres_offset,
            offsets,
            name_offset,
        })
    }

    pub fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.write_u32::<BigEndian>(self.subfile_length)?;
        writer.write_u32::<BigEndian>(self.subfile_version)?;
        writer.write_i32::<BigEndian>(self.brres_offset)?; // needs to be substituted

        for offset in &self.offsets {
            writer.write_i32::<BigEndian>(*offset)?;
        }

        writer.write_i32::<BigEndian>(self.name_offset)?; // needs to be substituted

        todo!("substitute offsets");

        Ok(())
    }

    /// Obtains the starting index of the specified section.
    pub fn get_section_start(&self, section_index: usize) -> SlipstreamResult<u64> {
        let offset = *self.offsets.get(section_index).ok_or_else(|| {
            SlipstreamError::from(RangeError {
                requested: section_index as u64,
                range: 0..self.offsets.len() as u64,
                ..Default::default()
            })
        })?;

        Ok((self.header_start as i64 + offset as i64) as u64)
    }
}

/// The filetype of a bfile.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BFileType {
    /// The root file only contains metadata about the size of the entire BRRES file.
    Root,
    /// Contains model data, this includes everything from vertices to bones, normals, etc.
    Mdl0,
    /// Character animations controlling the bones of a mesh.
    Chr0,
    Pat0,
    Tex0,
}

#[tracing::instrument(skip_all, fields(name))]
fn deserialize_bfile(
    reader: &mut RefCursor<[u8]>,
    parent_id: IrNodeKey,
    arena: &IrArena,
    name: String,
) -> SlipstreamResult<IrNodeKey> {
    // Check magic
    let magic = reader.read_u8_array::<4>()?;

    match magic {
        MDL0_MAGIC => mdl0::deserialize(reader, parent_id, arena, name),
        TEX0_MAGIC => tex0::deserialize(reader, parent_id, arena, name),
        // Chr0Subfile::MAGIC => Chr0Subfile::deserialize_lazy(reader),
        _ => {
            let key = arena.insert(IrNodeDescriptor {
                label: String::from("<unimplemented>"),
                ty: IrNodeType::Unknown,
                contents: ContentSlot::eager(Box::new(UnknownFile {
                    reader: reader.clone(),
                })),
                ..Default::default()
            });

            Ok(key)
        }
    }
}

fn serialize_bfile(writer: &mut MutCursor, node: &IrNode) -> SlipstreamResult<()> {
    struct BFileVisitor<'a> {
        result: SlipstreamResult<()>,
        writer: &'a mut MutCursor,
    }

    impl Visitor for BFileVisitor<'_> {
        fn visit_mdl0(&mut self, bfile: VisitorContext<'_, mdl0::Model>) -> ControlFlow<()> {
            todo!()
        }

        fn visit_tex0(&mut self, bfile: VisitorContext<'_, tex0::Texture>) -> ControlFlow<()> {
            todo!()
        }
    }

    let mut visitor = BFileVisitor {
        result: Ok(()),
        writer,
    };

    match node.contents.get() {
        Some(c) => c.accept(VisitorContextNode::from(node), &mut visitor),
        None => todo!("serialize unparsed file"), // this can be transferred straight over rather than deserializing and then serializing
    };

    Ok(())
}

/// Deserializes the contents of NW4R directories.
///
/// These are the actual roots of MDL0, CHR0, ßetc files.
#[tracing::instrument(skip_all, fields(label))]
fn deserialize_nw4r_subdirectories(
    reader: &mut RefCursor<[u8]>,
    label: String,
    parent_key: IrNodeKey,
    arena: &IrArena,
) -> SlipstreamResult<IrNodeKey> {
    let index = IndexGroup::deserialize(reader)?;

    let dir_key = arena.reserve_key();

    let mut bfiles = Vec::with_capacity(index.entries.len() - 1);
    for bfile in &index.entries[1..] {
        let label = index.get_entry_name(reader, bfile)?;
        let data_start = index.get_entry_data_start(bfile);

        reader.set_position(data_start);

        {
            let magic = &reader.remaining()[..4];
            tracing::trace!("Magic is {}", String::from_utf8_lossy(magic));
        }

        bfiles.push(deserialize_bfile(reader, dir_key, arena, label)?);
    }

    arena.insert_at(
        dir_key,
        IrNodeDescriptor {
            label,
            ty: IrNodeType::Nw4rDirectory,
            parent: Some(parent_key),
            children: bfiles,
            ..Default::default()
        },
    );

    Ok(dir_key)
}

fn serialize_nw4r_subdirectories(writer: &mut MutCursor) -> SlipstreamResult<()> {
    let index: IndexGroup = todo!("index group");
    index.serialize(writer)?;

    Ok(())
}

/// Deserializes the directories with an NW4R suffix,
/// i.e. `3DModels(NW4R)` or `Textures(NW4R)`
fn deserialize_nw4r_directories(
    reader: &mut RefCursor<[u8]>,
    parent_key: IrNodeKey,
    arena: &IrArena,
) -> SlipstreamResult<Vec<IrNodeKey>> {
    let index = IndexGroup::deserialize(reader)?;

    let mut section_dirs = Vec::with_capacity(index.entries.len() - 1);
    for section_dir in &index.entries[1..] {
        let label = index.get_entry_name(reader, section_dir)?;
        let data_start = index.get_entry_data_start(section_dir);

        reader.set_position(data_start);

        tracing::trace!("Deserializing BRRES NW4R directory `{label}`");

        section_dirs.push(deserialize_nw4r_subdirectories(
            reader, label, parent_key, arena,
        )?);
    }

    Ok(section_dirs)
}

/// Deserializes the root of a BRRES file.
#[tracing::instrument(skip_all, fields(label))]
pub fn deserialize(
    reader: &mut RefCursor<[u8]>,
    parent_key: Option<IrNodeKey>,
    arena: &IrArena,
    label: String,
) -> SlipstreamResult<IrNodeKey> {
    let header = BrresHeader::deserialize(reader)?;
    reader.set_position(header.start + header.root_offset as u64); // Skip to root start

    let _root = RootSection::deserialize(reader)?;

    let brres_key = arena.reserve_key();
    let brres_subdirectories = deserialize_nw4r_directories(reader, brres_key, arena)?;

    arena.insert_at(
        brres_key,
        IrNodeDescriptor {
            label,
            ty: IrNodeType::BrresFile,
            parent: parent_key,
            children: brres_subdirectories,
            ..Default::default()
        },
    );

    Ok(brres_key)
}
