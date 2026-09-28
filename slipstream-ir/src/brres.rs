use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::cursor::RefCursor;
use slipstream_shared::error::{
    CorruptionError, IncorrectFormat, RangeError, SlipstreamError, SlipstreamResult,
    UnsupportedError,
};

use crate::arc::UnknownFile;
use crate::encoding::ReadArrayExt;
use crate::index::IndexGroup;
use crate::mdl0::{self, MDL0_MAGIC};
use crate::node::arena::{IrArena, IrNodeDescriptor, IrNodeKey};
use crate::node::node::{ContentSlot, IrNode, IrNodeType};

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
    pub size: u32,
    pub root_offset: u16,
    pub section_count: u16,
}

impl BrresHeader {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
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
            root_offset,
            size,
            section_count,
        })
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
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
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

    /// Obtains the starting index of the specified section.
    pub fn get_section_start(&self, section_index: usize) -> SlipstreamResult<u32> {
        let offset = *self.offsets.get(section_index).ok_or_else(|| {
            SlipstreamError::from(RangeError {
                requested: section_index as u64,
                range: 0..self.offsets.len() as u64,
                ..Default::default()
            })
        })?;

        Ok((self.header_start as i32 + offset) as u32)
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
}

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
        // Chr0Subfile::MAGIC => Chr0Subfile::deserialize_lazy(reader),
        _ => {
            let key = arena.insert(IrNodeDescriptor {
                label: String::from("<unparsed>"),
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

/// Deserializes the contents of NW4R directories.
///
/// These are the actual roots of MDL0, CHR0, etc files.
fn deserialize_nw4r_subdirectories(
    reader: &mut RefCursor<[u8]>,
    label: String,
    parent_key: IrNodeKey,
    arena: &IrArena
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

    arena.insert_at(dir_key, IrNodeDescriptor {
        label,
        ty: IrNodeType::Nw4rDirectory,
        parent: Some(parent_key),
        children: bfiles,
        ..Default::default()
    });

    Ok(dir_key)
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

        section_dirs.push(deserialize_nw4r_subdirectories(reader, label, parent_key, arena)?);
    }

    Ok(section_dirs)
}

/// Deserializes the root of a BRRES file.
pub fn deserialize(
    reader: &mut RefCursor<[u8]>,
    parent_key: Option<IrNodeKey>,
    arena: &IrArena,
    label: String,
) -> SlipstreamResult<IrNodeKey> {
    let header = BrresHeader::deserialize(reader)?;
    reader.set_position(header.root_offset as u64); // Skip to root start

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

// pub fn deserialize(
//     reader: &mut RefCursor<[u8]>,
//     parent_id: IrNodeKey,
//     arena: &IrArena,
//     name: String,
// ) -> SlipstreamResult<IrNodeKey> {
//     let mut reader = reader.clone();
//     let brres_key = arena.reserve_key();
//
//     let arena2 = arena.clone();
//
//     let name2 = name.clone();
//     let parse_brres = move |_data| {
//         tracing::trace!("Triggered deferred parse of `{name2}`");
//
//         let header = BrresHeader::deserialize(&mut reader)?;
//
//         // Skip to root start
//         reader.set_position(header.root_offset as u64);
//
//         let _root = RootSection::deserialize(&mut reader)?;
//         let root_index = IndexGroup::deserialize(&mut reader)?;
//
//         let mut directories = Vec::with_capacity(root_index.entries.len());
//         for subdirectory in &root_index.entries[1..] {
//             let subdirectory_label = root_index
//                 .get_entry_name(&mut reader, subdirectory)?
//                 .to_owned();
//             let subdirectory_start = root_index.get_entry_data_start(subdirectory);
//             let subdirectory_key = arena.reserve_key();
//
//             tracing::trace!(
//                 "Discovered folder `{subdirectory_label}` at location {}",
//                 reader.position()
//             );
//
//             reader.set_position(subdirectory_start);
//
//             let subfile_index = IndexGroup::deserialize(&mut reader)?;
//             let mut subfiles = Vec::with_capacity(subfile_index.entries.len());
//
//             // Skip root subfile
//             for subfile in &subfile_index.entries[1..] {
//                 let subfile_label = subfile_index
//                     .get_entry_name(&mut reader, subfile)?
//                     .to_owned();
//
//                 let subfile_start = subfile_index.get_entry_data_start(subfile);
//
//                 tracing::trace!(
//                     "Discovered file `{subdirectory_label}/{subfile_label}` at location `{}`",
//                     reader.position()
//                 );
//
//                 reader.set_position(subfile_start);
//
//                 // Skip over unimplemented formats for testing for now
//                 {
//                     let magic = &reader.remaining()[..4];
//                     if magic != MDL0_MAGIC && magic != Chr0Subfile::MAGIC {
//                         tracing::error!("SKIPPING {}", String::from_utf8_lossy(magic));
//                         continue;
//                     }
//                 }
//
//                 let file =
//                     tracing::trace_span!("deserialize_subfile", %subdirectory_label, %subfile_label)
//                         .in_scope(|| {
//                             deserialize_bfile(&mut reader, subdirectory_key, &arena2, subfile_label)
//                         })?;
//
//                 subfiles.push(file);
//             }
//
//             arena.insert_at(
//                 subdirectory_key,
//                 IrNodeDescriptor {
//                     label: subdirectory_label,
//                     ty: IrNodeType::BrresDirectory,
//                     parent: parent_id,
//                     children: subfiles,
//                     ..Default::default()
//                 },
//             );
//
//             directories.push(subdirectory_key);
//         }
//
//         Ok(VirtualNodeBody {
//             inspectable: None,
//             children: directories,
//         })
//     };
//
//     arena.insert_at(brres_key, IrNodeDescriptor {
//         label: name,
//         ty: IrNodeType::ArcDirectory { empty:  },
//
//     });
//
//     Ok(brres_key)
// }
