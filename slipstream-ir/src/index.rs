use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use slipstream_shared::{
    cursor::{MutCursor, RefCursor, SizeEstimate},
    error::SlipstreamResult,
};

use crate::encoding::ReadStringExt;

/// Header of a BRRES index group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexGroupHeader {
    /// The length in bytes of the index group.
    pub length: u32,
    /// The number of entries in this index, excluding the root file.
    pub number: u32,
}

impl IndexGroupHeader {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        Ok(Self {
            length: reader.read_u32::<BigEndian>()?,
            number: reader.read_u32::<BigEndian>()?,
        })
    }

    pub fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.write_u32::<BigEndian>(self.length)?;
        writer.write_u32::<BigEndian>(self.number)?;
        Ok(())
    }
}

impl SizeEstimate for IndexGroupHeader {
    #[inline]
    fn estimate_size(&self) -> usize {
        8
    }
}

/// A section in an index group.
///
/// These build a tree that enables the Wii to efficiently find files in archives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexGroupEntry {
    pub entry_id: u16,
    pub flag: u16,
    pub left_index: u16,
    pub right_index: u16,
    /// Pointer to the the name of this entry.
    ///
    /// Use [`get_entry_name`] to obtain the name associated with this entry.
    ///
    /// [`get_entry_name`]: IndexGroup::get_entry_name
    pub name_pointer: u32,
    /// Pointer to the data of this entry.
    ///
    /// Use [`get_entry_data_start`] to get a pointer to the start of the actual data.
    ///
    /// [`get_entry_data_start`]: IndexGroup::get_entry_data_start
    pub data_pointer: u32,
}

impl IndexGroupEntry {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let entry_id = reader.read_u16::<BigEndian>()?;
        let flag = reader.read_u16::<BigEndian>()?;
        let left_index = reader.read_u16::<BigEndian>()?;
        let right_index = reader.read_u16::<BigEndian>()?;
        let name_pointer = reader.read_u32::<BigEndian>()?;
        let data_pointer = reader.read_u32::<BigEndian>()?;

        Ok(Self {
            entry_id,
            flag,
            left_index,
            right_index,
            name_pointer,
            data_pointer,
        })
    }

    pub fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.write_u16::<BigEndian>(self.entry_id)?;
        writer.write_u16::<BigEndian>(self.flag)?;
        writer.write_u16::<BigEndian>(self.left_index)?;
        writer.write_u16::<BigEndian>(self.right_index)?;
        writer.write_u32::<BigEndian>(self.name_pointer)?; // must be substituted
        writer.write_u32::<BigEndian>(self.data_pointer)?; // must be substituted

        todo!("substitute offsets");

        Ok(())
    }
}

impl SizeEstimate for IndexGroupEntry {
    #[inline]
    fn estimate_size(&self) -> usize {
        16
    }
}

/// Describes locations of the subfiles in this BRRES file.
///
/// This is used for any kind of object that might contain subentries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexGroup {
    /// Index into the BRRES file where this group starts. Generally this is just right after the BRRES header.
    pub group_start: u32,
    /// The index group header.
    pub header: IndexGroupHeader,
    /// `header.number + 1` entries.
    ///
    /// The first entry is a dummy entry that has no name.
    pub entries: Vec<IndexGroupEntry>,
}

impl IndexGroup {
    /// Obtains the name of the given index group entry using its name pointer.
    ///
    /// `data` should be the entire BRRES file (including header).
    ///
    /// # Conditions
    /// - The given index group entry must be owned by the current index group.
    ///
    /// Violating these conditions will not cause unsoundness but will either cause a panic due to invalid
    /// UTF-8 or return incorrect strings.
    pub fn get_entry_name(
        &self,
        reader: &mut RefCursor<[u8]>,
        entry: &IndexGroupEntry,
    ) -> SlipstreamResult<String> {
        if entry.name_pointer == 0 {
            return Ok(String::new()); // This entry has no name.
        }

        // Move cursor to name and then back after reading.
        let name_start = self.group_start + entry.name_pointer;
        reader.set_position(name_start as u64 - 4);

        let name = reader.read_u32_string::<BigEndian>()?;

        Ok(name)
    }

    /// Obtains a pointer to the start of the data section of the given entry.
    pub fn get_entry_data_start(&self, entry: &IndexGroupEntry) -> u64 {
        (self.group_start + entry.data_pointer) as u64
    }

    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let group_start = reader.position() as u32;
        let header = IndexGroupHeader::deserialize(reader)?;

        let mut entries = Vec::with_capacity(header.number as usize);
        for _ in 0..header.number + 1 {
            let entry = IndexGroupEntry::deserialize(reader)?;
            entries.push(entry);
        }

        debug_assert_eq!(
            reader.position() as u32 - group_start,
            header.length,
            "an incorrect number of index group entries was read"
        );

        Ok(Self {
            group_start,
            header,
            entries,
        })
    }

    pub fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        self.header.serialize(writer)?;

        for entry in &self.entries {
            entry.serialize(writer)?;
        }

        Ok(())
    }
}

impl SizeEstimate for IndexGroup {
    #[inline]
    fn estimate_size(&self) -> usize {
        self.header.estimate_size()
            + self.entries.len() * self.entries.first().map(|f| f.estimate_size()).unwrap_or(0)
    }
}

#[inline]
fn get_highest_bit(value: u8) -> u16 {
    7 - value.leading_zeros() as u16
}

// The index ID is computed by comparing the two strings.
//
// 1. Find the last non-equal character of both names and store the `index`.
//      1. If the length of the subject filename is greater than the length of the object filename,
//         `index` is the length of the subject filename minus 1.
//      2. Otherwise compare each pair of characters, starting from the end, until a difference is found.
// 2. Compare both characters of position `index` and find the highest bit that is not equal. If `index` exceeds
//    the length of the object, assume character value 0. Store the bit index (7 for `0x80`, .., 0 for `0x01`) as `bitnum`.
// 3. Calculate `id = (index << 3) | bitnum`
fn compute_index_id(object: &[u8], subject: &[u8]) -> u16 {
    if object.len() < subject.len() {
        let last_idx = subject.len() - 1;
        return ((last_idx as u16) << 3) | get_highest_bit(subject[subject.len() - 1]);
    }

    (0..subject.len())
        .rev()
        .find_map(|i| {
            let ch = object[i] ^ subject[i];
            (ch != 0).then(|| ((i as u16) << 3) | get_highest_bit(ch))
        })
        .unwrap_or(u16::MAX)
}
