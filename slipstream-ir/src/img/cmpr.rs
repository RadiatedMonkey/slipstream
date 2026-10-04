use slipstream_shared::{RefCursor, SlipstreamResult};

pub struct CmprDescriptor {
    pub width: u16,
    pub height: u16,
    pub mipmap_count: u32,
}

pub struct CmprImage {
    pub size: glam::U16Vec2,
}

impl CmprImage {
    pub fn from_reader(
        reader: &mut RefCursor<[u8]>,
        descriptor: CmprDescriptor,
    ) -> SlipstreamResult<Self> {
        todo!()
    }
}
