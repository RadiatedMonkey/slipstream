use bitfield_struct::bitfield;
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct IndexedLoad {
    /// The XF slot to pull the data from.
    #[bits(16)]
    pub index: u16,
    /// Address into the indexed array.
    #[bits(12)]
    pub address: u16,
    /// Transfer count - 1
    #[bits(4)]
    pub transfer_count_one: u8,
}

impl IndexedLoad {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Self::from_bits(word))
    }
}
