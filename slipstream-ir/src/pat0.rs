use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

use crate::{
    brres::{BFile, BFileHeader, BFileType},
    encoding::ReadStringExt,
    index::IndexGroup,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pat0Header {
    pub frame_count: u16,
    pub base_number: u16,
    pub string_number: u16,
    pub cyclic: bool,
}

impl Pat0Header {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let _unknown12 = reader.read_u32::<BigEndian>()?; // 2 + 2 unknown bytes
        let frame_count = reader.read_u16::<BigEndian>()?;
        let base_number = reader.read_u16::<BigEndian>()?;
        let string_number = reader.read_u16::<BigEndian>()?;
        let _unknown34 = reader.read_u32::<BigEndian>()?; // 2 + 2 unknown bytes
        let cyclic = reader.read_u16::<BigEndian>()? != 0;

        Ok(Self {
            frame_count,
            base_number,
            string_number,
            cyclic,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationSection {}

/// Both section #1 and section #3 have the same format of `string_number` amount of u32s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U32Section {
    pub offsets: Vec<u32>,
}

impl U32Section {
    pub fn deserialize(reader: &mut RefCursor<[u8]>, string_number: u16) -> SlipstreamResult<Self> {
        let mut offsets = Vec::with_capacity(string_number as usize);
        for _ in 0..string_number {
            offsets.push(reader.read_u32::<BigEndian>()?);
        }

        Ok(Self { offsets })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pat0Subfile {
    pub subfile_header: BFileHeader,
    pub pat0_header: Pat0Header,
}

impl Pat0Subfile {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let subfile_header = BFileHeader::deserialize(reader, BFileType::Pat0)?;
        let pat0_header = Pat0Header::deserialize(reader)?;

        let name_start = subfile_header.header_start as i64 + subfile_header.name_offset as i64;
        reader.set_position(name_start as u64);

        let pat0_name = reader.read_null_string::<BigEndian>()?;
        dbg!(pat0_name);

        dbg!(&subfile_header, pat0_header);

        let index_group = IndexGroup::deserialize(reader)?;
        let name = index_group.get_entry_name(reader, &index_group.entries[1])?;
        dbg!(name);

        todo!();
    }
}

impl BFile for Pat0Subfile {
    const MAGIC: [u8; 4] = [0x50, 0x41, 0x54, 0x30]; // "PAT0"
}
