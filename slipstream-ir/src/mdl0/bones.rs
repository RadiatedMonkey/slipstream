use bitfield_struct::bitfield;
use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, InvalidInputError, SlipstreamError, SlipstreamResult},
};

use crate::{
    encoding::ReadArrayExt,
    index::IndexGroup,
    node::{
        arena::{IrArena, IrNodeKey},
        node::IrNodeType,
    },
    visitor::{Visitable, Visitor},
};

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct BoneFlags {
    pub use_identity: bool,
    pub translation_isotropic: bool,
    pub rotation_isotropic: bool,
    pub scale_isotropic: bool,
    pub scale_uniform: bool,
    pub apply_scale_compensate: bool,
    pub apply_child_scale_compensate: bool,
    pub disable_classic_scale: bool,
    pub is_visible: bool,
    pub is_display_matrix: bool,
    pub is_billboard_child: bool,
    #[bits(21)]
    pub _unused: u32,
}

/// Configures the way billboarding is used for this object.
///
/// This can be used to make something always face the camera.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BillboardSetting {
    /// No influence.
    Disabled,
    /// Influenced by rotation of parent node. Z-axis is parallel to camera lens axis.
    Billboard,
    /// Influenced by rotation of parent node. Z-axis points toward camera direction.
    PerspectiveBillboard,
    /// Not influenced by rotation of parent node, restricted by camera's up vector.
    /// Z-axis is parallel to camera lens axis.
    CameraBillboard,
    /// Not influenced by rotation of parent node, restricted by camera's up vector.
    /// Z-axis points toward camera direction.
    CameraPerspectiveBillboard,
    /// Influenced by rotation of parent node and rotates only around Y-axis.
    /// Z-axis is parallel to camera lens axis.
    YBillboard,
    /// Influenced by rotation of parent node and rotates only around Y-axis.
    /// Z-axis points toward camera direction.
    YPerspectiveBillboard,
}

impl BillboardSetting {
    /// Returns the amount of items in this enum.
    pub fn len() -> usize {
        Self::YPerspectiveBillboard as usize + 1
    }
}

impl TryFrom<u32> for BillboardSetting {
    type Error = SlipstreamError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Disabled,
            1 => Self::Billboard,
            2 => Self::PerspectiveBillboard,
            3 => Self::CameraBillboard,
            4 => Self::CameraPerspectiveBillboard,
            5 => Self::YBillboard,
            6 => Self::YPerspectiveBillboard,
            v => {
                return Err(CorruptionError {
                    reason: format!("invalid bone flag billboard setting: {v} (expected 0-6)"),
                    ..Default::default()
                }
                .into());
            }
        })
    }
}

impl BillboardSetting {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let word = reader.read_u32::<BigEndian>()?;
        Self::try_from(word)
    }
}

/// A bone that already has all its data deserialized but without resolved references.
///
/// When constructing the skeleton, a new [`Bone`] is created that contains proper references
/// to other bones.
#[derive(Debug, Clone, PartialEq)]
struct UnresolvedBone {
    pub bone_start: u32,
    pub mdl0_offset: i32,
    pub name_offset: i32,
    /// The index of this bone in `Bones` section of the MDL0 file.
    pub index: u32,
    pub id: u32,
    pub flags: BoneFlags,
    /// Configures how billboarding is used for this bone.
    pub billboard_setting: BillboardSetting,
    pub billboard_transform: u32,
    pub scaling_vector: [f32; 3],
    pub rotation_vector: [f32; 3],
    pub translation_vector: [f32; 3],
    pub bounding_volume_min: [f32; 3],
    pub bounding_volume_max: [f32; 3],
    /// The offset in bytes to the parent of this bone.
    pub parent_offset: i32,
    pub first_child_offset: i32,
    pub next_sibling_offset: i32,
    pub previous_sibling_offset: i32,
    pub user_data_offset: i32,
    pub transform_matrix: [f32; 12],
    pub inverse_matrix: [f32; 12],
}

impl UnresolvedBone {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let start = reader.position();

        let length = reader.read_u32::<BigEndian>()?;
        let mdl0_offset = reader.read_i32::<BigEndian>()?;
        let name_offset = reader.read_i32::<BigEndian>()?;
        let index = reader.read_u32::<BigEndian>()?;
        let id = reader.read_u32::<BigEndian>()?;
        let flags = BoneFlags::from_bits(reader.read_u32::<BigEndian>()?);
        let billboard_setting = BillboardSetting::deserialize(reader)?;
        let billboard_transform = reader.read_u32::<BigEndian>()?;

        let scaling_vector = reader.read_f32_array::<3, BigEndian>()?;
        let rotation_vector = reader.read_f32_array::<3, BigEndian>()?;
        let translation_vector = reader.read_f32_array::<3, BigEndian>()?;
        let bounding_volume_min = reader.read_f32_array::<3, BigEndian>()?;
        let bounding_volume_max = reader.read_f32_array::<3, BigEndian>()?;
        let parent_offset = reader.read_i32::<BigEndian>()?;
        let first_child_offset = reader.read_i32::<BigEndian>()?;
        let next_sibling_offset = reader.read_i32::<BigEndian>()?;
        let previous_sibling_offset = reader.read_i32::<BigEndian>()?;
        let user_data_offset = reader.read_i32::<BigEndian>()?;
        let transform_matrix = reader.read_f32_array::<12, BigEndian>()?;
        let inverse_matrix = reader.read_f32_array::<12, BigEndian>()?;

        reader.set_position(start + length as u64);

        Ok(Self {
            bone_start: start as u32,
            mdl0_offset,
            name_offset,
            index,
            id,
            flags,
            billboard_setting,
            billboard_transform,
            scaling_vector,
            rotation_vector,
            translation_vector,
            bounding_volume_min,
            bounding_volume_max,
            parent_offset,
            first_child_offset,
            next_sibling_offset,
            previous_sibling_offset,
            user_data_offset,
            transform_matrix,
            inverse_matrix,
        })
    }
}

/// A bone combined with its name.
#[derive(Debug)]
pub struct LabeledBone {
    pub name: String,
    pub bone: UnresolvedBone,
}

#[derive(Debug)]
pub struct Bone {
    pub index: u32,
    pub id: u32,
    pub flags: BoneFlags,
    pub billboard_setting: BillboardSetting,
    pub billboard_reference: Option<IrNodeKey>,
    pub scaling_vector: [f32; 3],
    pub rotation_vector: [f32; 3],
    pub translation_vector: [f32; 3],
    pub bounding_volume_min: [f32; 3],
    pub bounding_volume_max: [f32; 3],
    pub parent: Option<IrNodeKey>,
    pub user_data_offset: i32,
    pub transform_matrix: [f32; 12],
    pub inverse_matrix: [f32; 12],
}

impl Bone {
    /// Converts raw bone data into a more usable format by resolving the file offsets
    /// to proper node references. This ensures the editor knows which other files this one refers to.
    pub fn from_bone(
        bone: &UnresolvedBone,
        billboard_id: Option<IrNodeKey>,
        parent_id: Option<IrNodeKey>,
    ) -> Self {
        Self {
            index: bone.index,
            id: bone.id,
            flags: bone.flags.clone(),
            billboard_setting: bone.billboard_setting,
            billboard_reference: billboard_id,
            scaling_vector: bone.scaling_vector,
            rotation_vector: bone.rotation_vector,
            translation_vector: bone.translation_vector,
            bounding_volume_min: bone.bounding_volume_min,
            bounding_volume_max: bone.bounding_volume_max,
            parent: parent_id,
            user_data_offset: bone.user_data_offset,
            transform_matrix: bone.transform_matrix,
            inverse_matrix: bone.inverse_matrix,
        }
    }
}

impl Visitable for Bone {
    fn accept(&self, visitor: &mut dyn Visitor) {
        visitor.visit_bone(self)
    }
}

/// Builds a nested tree of bones as nodes and returns the root node of the skeleton.
fn build_skeleton_tree(
    reader: &mut RefCursor<[u8]>,
    parent_id: IrNodeKey,
    bones: &[LabeledBone],
    arena: &IrArena,
) -> SlipstreamResult<IrNodeKey> {
    /// The offset between the start of the bone and the bone's index.
    const BONE_INDEX_OFFSET: u64 = 3 * 4;

    // Create a virtual node for each of the bones.
    //
    // These will later be attached to each other to form a skeleton.
    let virtual_bones = bones
        .iter()
        .map(|bone| {
            let id = arena.next_id();
            let node = VirtualNode {
                label: bone.name.clone(),
                id,
                ty: IrNodeType::Bone { end: true }, // Set as final bone by default.
                parent: None,
                body: Deferred::evaluated(VirtualNodeBody {
                    children: Vec::new(),
                    inspectable: None,
                }),
            };

            arena.insert(id, node);
            id
        })
        .collect::<Vec<_>>();

    let mut found_root = None; // The bone that was determined to be the root of the skeleton.
    for (i, bone) in bones.iter().enumerate() {
        let curr_id = virtual_bones[i];

        if bone.bone.parent_offset == 0 {
            found_root = Some(i);

            // Set the body of this bone to the virtual bone data.
            let node = arena
                .get(curr_id)
                .expect("virtual node that was just added does not exist");

            let mut lock = node.write();
            lock.body.inspect_mut(|body| {
                body.inspectable = Some(Box::new(Bone::from_bone(&bone.bone, None, None)));
            });

            lock.parent = Some(parent_id);
            lock.ty = IrNodeType::Bone { end: false };

            continue; // No parent
        }

        let parent_start = bone.bone.bone_start as i64 + bone.bone.parent_offset as i64;
        reader.set_position(parent_start as u64 + BONE_INDEX_OFFSET);

        // Index into `virtual_bones` of the parent.
        let parent_index = reader.read_u32::<BigEndian>()?;
        if parent_index == i as u32 {
            // Ensure a bone is not its own parent.
            // This would cause cyclical references.

            return Err(InvalidInputError {
                reason: format!("bone `{}` is its own parent", bone.name),
                ..Default::default()
            }
            .into());
        }

        let parent_id = virtual_bones[parent_index as usize];
        let parent_node = arena.get(parent_id).ok_or_else(|| {
            SlipstreamError::from(InvalidInputError {
                reason: format!("virtual node {parent_id} does not exist"),
                ..Default::default()
            })
        })?;

        {
            let mut lock = parent_node.write();

            // Change the file tree kind to reflect that it now has children.
            lock.ty = IrNodeType::Bone { end: false };

            // Add this child to its parent.
            lock.body.inspect_mut(|body| {
                body.children.push(curr_id);
            });
        }

        let curr_node = arena.get(curr_id).ok_or_else(|| {
            SlipstreamError::from(InvalidInputError {
                reason: format!("virtual node {curr_id} does not exist"),
                ..Default::default()
            })
        })?;

        {
            // Set the bone's parent and its data.
            let mut lock = curr_node.write();
            lock.parent = Some(parent_id);
            lock.body.inspect_mut(|body| {
                body.inspectable =
                    Some(Box::new(Bone::from_bone(&bone.bone, None, Some(parent_id))));
            });
        }
    }

    let Some(root_index) = found_root else {
        return Err(InvalidInputError {
            reason: String::from("skeleton contained no root bone"),
            ..Default::default()
        }
        .into());
    };

    tracing::trace!("Skeleton constructed");

    let root = virtual_bones[root_index];

    Ok(root)
}

/// Deserializes the entire `Bones` section of an MDL0 file.
///
/// The parser automatically builds a proper file tree of bones that the outliner
/// can display. References between bones are resolved to use node IDs instead.
#[tracing::instrument(skip_all, fields(parent_id))]
pub fn deserialize_skeleton(
    reader: &mut RefCursor<[u8]>,
    parent_id: IrNodeKey,
    arena: &IrArena,
) -> SlipstreamResult<VirtualNodeBody> {
    let section_index = IndexGroup::deserialize(reader)?;
    let mut bones = Vec::with_capacity(section_index.entries.len() - 1);

    for entry in &section_index.entries[1..] {
        let name = section_index.get_entry_name(reader, entry)?;

        let data_start = section_index.get_entry_data_start(entry);
        reader.set_position(data_start as u64);

        let bone = UnresolvedBone::deserialize(reader)?;
        bones.push(LabeledBone { name, bone });
    }

    let node = build_skeleton_tree(reader, parent_id, &bones, arena)?;

    Ok(VirtualNodeBody {
        inspectable: None,
        children: vec![node],
    })
}
