use std::ops::ControlFlow;

use crate::encoding::ReadArrayExt;
use crate::gx::GxBytecode;

use crate::mdl0::section::DeserializeContents;
use crate::node::node::{IrNode, IrNodeType};
use crate::visitor::{Visitable, Visitor, VisitorContext, VisitorContextMut, VisitorContextNode, VisitorContextNodeMut};
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::cursor::RefCursor;
use slipstream_shared::error::SlipstreamResult;

#[derive(Debug, Clone, PartialEq)]
pub struct Tev {
    pub tex_scales: [u8; 8],
    pub bytecode: GxBytecode,
}

impl Visitable for Tev {
    fn accept(&self, node: VisitorContextNode<'_>, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_tev(VisitorContext::new(node, self))
    }

    fn accept_mut(&mut self, node: VisitorContextNodeMut<'_>, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_tev_mut(VisitorContextMut::new(node, self))
    }
}

impl DeserializeContents for Tev {
    const NAME: &str = "Shaders";
    const KIND: IrNodeType = IrNodeType::Tevs;

    #[tracing::instrument(skip_all, fields(header_start = _header_start))]
    fn deserialize_contents(
        reader: &mut RefCursor<[u8]>,
        _header_start: u64,
    ) -> SlipstreamResult<Self> {
        let _length = reader.read_u32::<BigEndian>()?;
        let _mdl0_offset = reader.read_i32::<BigEndian>()?;
        let _index = reader.read_i32::<BigEndian>()?;
        let _stage_count = reader.read_u8()?;

        reader.set_position(reader.position() + 3); // padding

        let tex_scales = reader.read_u8_array::<8>()?;

        reader.set_position(reader.position() + 8); // padding

        let bytecode = GxBytecode::deserialize_tev_data(reader)?;

        Ok(Self {
            tex_scales,
            bytecode,
        })
    }
}
