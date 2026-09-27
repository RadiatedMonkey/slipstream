use crate::node::node::{ContentSlot, IrNode, IrNodeType};

slotmap::new_key_type! { pub struct IrNodeKey; }

pub struct IrNodeDescriptor {
    pub label: String,
    pub ty: IrNodeType,
    pub children: Vec<IrNodeKey>,
    pub contents: ContentSlot,
}

impl Default for IrNodeDescriptor {
    fn default() -> Self {
        Self {
            label: String::from("<null>"),
            ty: IrNodeType::Unknown,
            children: Vec::new(),
            contents: ContentSlot::None,
        }
    }
}

#[derive(Default)]
pub struct IrArena {
    map: slotmap::SlotMap<IrNodeKey, IrNode>,
}

impl IrArena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, desc: IrNodeDescriptor) -> IrNodeKey {
        todo!()
    }
}
