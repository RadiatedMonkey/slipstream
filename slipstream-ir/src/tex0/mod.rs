use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{RefCursor, SlipstreamResult, error::UnsupportedError, try_unwrap};

use crate::{
    brres::{BFileHeader, BFileType},
    encoding::Deserialize,
    node::{
        arena::{IrArena, IrNodeDescriptor, IrNodeKey},
        node::{ContentSlot, IrNodeType},
    },
    visitor::{
        Visitable, Visitor, VisitorContext, VisitorContextMut, VisitorContextNode,
        VisitorContextNodeMut,
    },
};

pub const TEX0_MAGIC: [u8; 4] = [0x54, 0x45, 0x58, 0x30]; // "TEX0"

#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::FromRepr)]
#[repr(u32)]
pub enum TextureFormat {
    I4 = 0x00,
    I8 = 0x01,
    Ia4 = 0x02,
    Ia8 = 0x03,
    Rgb565 = 0x04,
    Rgb5a3 = 0x05,
    Rgba32 = 0x06,
    // there is no 0x07 apparently
    C4 = 0x08,
    C8 = 0x09,
    C14x2 = 0x0a,
    // also no 0x0b, 0x0c and 0x0d
    Cmpr = 0x0e,
}

impl Deserialize for TextureFormat {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        try_unwrap!(Self::from_repr(word), "invalid texture format: {word:#04x}")
    }
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub size: glam::U16Vec2,
    pub format: TextureFormat,
    pub mipmap_count: u32,
}

impl Visitable for Texture {
    fn accept(&self, node: VisitorContextNode, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_texture(VisitorContext::new(node, self));
        ControlFlow::Continue(())
    }

    fn accept_mut(
        &mut self,
        node: VisitorContextNodeMut,
        visitor: &mut dyn Visitor,
    ) -> std::ops::ControlFlow<()> {
        visitor.visit_texture_mut(VisitorContextMut::new(node, self));
        ControlFlow::Continue(())
    }
}

#[tracing::instrument(skip_all, fields(name, parent_id))]
pub fn deserialize(
    reader: &mut RefCursor<[u8]>,
    parent_id: IrNodeKey,
    arena: &IrArena,
    name: String,
) -> SlipstreamResult<IrNodeKey> {
    let subfile_header = BFileHeader::deserialize(reader, BFileType::Tex0)?;

    let flag = reader.read_u32::<BigEndian>()?;
    let pixel_width = reader.read_u16::<BigEndian>()?;
    let pixel_height = reader.read_u16::<BigEndian>()?;
    let format = TextureFormat::deserialize(reader)?;
    let mipmap_count = reader.read_u32::<BigEndian>()?;
    let min_mipmap_used = reader.read_f32::<BigEndian>()?;
    let max_mipmap_used = reader.read_f32::<BigEndian>()?;
    let _unused = reader.read_u32::<BigEndian>()?;

    let key = arena.insert(IrNodeDescriptor {
        label: name,
        ty: IrNodeType::Texture,
        parent: Some(parent_id),
        children: Vec::new(),
        contents: ContentSlot::eager(Box::new(Texture {
            size: glam::u16vec2(pixel_width, pixel_height),
            format,
            mipmap_count,
        })),
    });

    Ok(key)
}
