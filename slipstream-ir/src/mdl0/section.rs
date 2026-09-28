use slipstream_shared::{cursor::RefCursor, error::SlipstreamResult};

use crate::{
    index::IndexGroup,
    node::{
        arena::{IrArena, IrNodeDescriptor, IrNodeKey},
        node::{ContentSlot, IrNodeType},
    },
    visitor::Visitable,
};

pub trait DeserializeSection: Sized {
    const NAME: &str;
    const KIND: IrNodeType;

    fn deserialize_section(
        reader: &mut RefCursor<[u8]>,
        header_start: u64,
    ) -> SlipstreamResult<Self>;
}

/// Deserializes an MDL0 section (normals, vertices, etc) that has a simple layout.
///
/// This function returns the key to the deserialized section node.
///
/// This means that the entries do not have any subfiles (such as the hierarchical layout of the bones).
#[tracing::instrument(skip_all)]
pub fn deserialize_leaf_section<T: DeserializeSection>(
    reader: &mut RefCursor<[u8]>,
    header_start: u32,
    parent: IrNodeKey,
    arena: &IrArena,
) -> SlipstreamResult<IrNodeKey> {
    let index = IndexGroup::deserialize(reader)?;
    let section_key = arena.reserve_key();

    let mut entries = Vec::with_capacity(index.entries.len() - 1);
    for entry in &index.entries[1..] {
        let label = index.get_entry_name(reader, entry)?;
        let data_start = index.get_entry_data_start(entry);

        reader.set_position(data_start);

        let key = arena.insert(IrNodeDescriptor {
            label,
            ty: T::KIND,
            parent: Some(section_key),
            contents: ContentSlot::lazy(reader.clone(), T::KIND),
            ..Default::default()
        });

        entries.push(key);
    }

    arena.insert_at(
        section_key,
        IrNodeDescriptor {
            label: T::NAME.to_owned(),
            ty: T::KIND,
            parent: Some(parent),
            children: entries,
            ..Default::default()
        },
    );

    Ok(section_key)
}
