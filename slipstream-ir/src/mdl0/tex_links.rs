use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

use crate::{
    node::node::IrNodeType,
    section::DeserializeSection,
    visitor::{Visitable, Visitor},
};

#[derive(Debug, Clone, PartialEq)]
pub struct TextureLink {
    pub offset1: u32,
    pub offset2: u32,
}

impl TextureLink {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let offset1 = reader.read_u32::<BigEndian>()?;
        let offset2 = reader.read_u32::<BigEndian>()?;

        Ok(Self { offset1, offset2 })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureLinks {
    pub links: Vec<TextureLink>,
}

impl Visitable for TextureLinks {
    fn accept(&self, visitor: &mut dyn Visitor) {
        visitor.visit_texture_links(self)
    }
}

impl DeserializeSection for TextureLinks {
    const KIND: IrNodeType = IrNodeType::TextureLinks;

    #[tracing::instrument(skip_all, fields(header_start = _header_start))]
    fn deserialize_section(
        reader: &mut RefCursor<[u8]>,
        _header_start: u64,
    ) -> SlipstreamResult<Self> {
        let link_count = reader.read_u32::<BigEndian>()?;

        let mut links = Vec::with_capacity(link_count as usize);
        for _ in 0..link_count {
            links.push(TextureLink::deserialize(reader)?);
        }

        Ok(Self { links })
    }
}
