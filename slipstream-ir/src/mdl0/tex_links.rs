use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt};
use slipstream_derive::Inspect;
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

use crate::mdl0::section::DeserializeContents;
use crate::{
    node::node::IrNodeType,
    visitor::{Visitable, Visitor},
};

#[derive(Debug, Clone, PartialEq, Inspect)]
pub struct TextureLink {
    #[inspect(rename = "Offset 1", min = 15)]
    pub offset1: u32,
    #[inspect(rename = "Offset 2")]
    pub offset2: u32,
}

impl TextureLink {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let offset1 = reader.read_u32::<BigEndian>()?;
        let offset2 = reader.read_u32::<BigEndian>()?;

        Ok(Self { offset1, offset2 })
    }
}

#[derive(Debug, Clone, PartialEq, Inspect)]
pub struct TextureLinks {
    pub links: Vec<TextureLink>,
}

impl Visitable for TextureLinks {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_texture_links(self)
    }

    fn accept_mut(&mut self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_texture_links_mut(self)
    }
}

impl DeserializeContents for TextureLinks {
    const NAME: &str = "Texture links";
    const KIND: IrNodeType = IrNodeType::TextureLinks;

    #[tracing::instrument(skip_all, fields(header_start = _header_start))]
    fn deserialize_contents(
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
