use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallDisplayList {
    pub address: u32,
    pub size: u32,
}

impl CallDisplayList {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let address = reader.read_u32::<BigEndian>()?;
        let size = reader.read_u32::<BigEndian>()?;

        Ok(Self { address, size })
    }
}
