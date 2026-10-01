use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use slipstream_derive::Inspect;
use slipstream_shared::cursor::{MutCursor, SizeEstimate};
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

use crate::mdl0::section::{DeserializeContents, SerializeContents};
use crate::{
    node::node::IrNodeType,
    visitor::{Visitable, Visitor},
};
use crate::node::node::IrNode;
use crate::visitor::{VisitorContext, VisitorContextMut, VisitorContextNode, VisitorContextNodeMut};

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

    fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.write_u32::<BigEndian>(self.offset1)?;
        writer.write_u32::<BigEndian>(self.offset2)?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Inspect)]
pub struct TextureLinks {
    pub links: Vec<TextureLink>,
}

impl Visitable for TextureLinks {
    fn accept(&self, node: VisitorContextNode<'_>, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_texture_links(VisitorContext::new(node, self))
    }

    fn accept_mut(&mut self, node: VisitorContextNodeMut<'_>, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_texture_links_mut(VisitorContextMut::new(node, self))
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

impl SerializeContents for TextureLinks {
    fn serialize_contents(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.write_u32::<BigEndian>(self.links.len() as u32)?;
        for link in &self.links {
            link.serialize(writer)?;
        }
        Ok(())
    }
}

impl SizeEstimate for TextureLinks {
    #[inline]
    fn estimate_size(&self) -> usize {
        4 + self.links.len() * 8
    }
}
