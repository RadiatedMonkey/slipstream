use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

use crate::{
    node::node::IrNodeType,
    visitor::{Visitable, Visitor},
};
use crate::mdl0::section::DeserializeSection;

#[derive(Debug, Clone, PartialEq)]
pub struct PaletteLink {
    pub offset1: u32,
    pub offset2: u32,
}

impl PaletteLink {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let offset1 = reader.read_u32::<BigEndian>()?;
        let offset2 = reader.read_u32::<BigEndian>()?;

        Ok(Self { offset1, offset2 })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaletteLinks {
    pub links: Vec<PaletteLink>,
}

impl Visitable for PaletteLinks {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_palette_links(self)
    }
}

impl DeserializeSection for PaletteLinks {
    const NAME: &str = "Palette links";
    const KIND: IrNodeType = IrNodeType::PaletteLinks;

    #[tracing::instrument(skip_all, fields(header_start = _header_start))]
    fn deserialize_section(
        reader: &mut RefCursor<[u8]>,
        _header_start: u64,
    ) -> SlipstreamResult<Self> {
        let link_count = reader.read_u32::<BigEndian>()?;

        let mut links = Vec::with_capacity(link_count as usize);
        for _ in 0..link_count {
            links.push(PaletteLink::deserialize(reader)?);
        }

        Ok(Self { links })
    }
}
