use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use slipstream_derive::Inspect;
use slipstream_shared::cursor::{MutCursor, RefCursor, SizeEstimate};
use slipstream_shared::error::{CorruptionError, SlipstreamResult};
use slipstream_shared::verify;

use crate::mdl0::section::{DeserializeContents, SerializeContents};
use crate::node::node::{IrNode, IrNodeType};
use crate::visitor::{Visitable, Visitor, VisitorContext, VisitorContextMut, VisitorContextNode, VisitorContextNodeMut};

pub const NODE_TREE_NAME: &str = "NodeTree";
pub const NODE_MIX_NAME: &str = "NodeMix";
pub const DRAW_OPA_NAME: &str = "DrawOpa";

/// Simple newtype that makes types with many IDs a lot clearer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MatrixId(pub u16);

/// Simple newtype that makes types with many IDs a lot clearer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BoneId(pub u16);

/// Simple newtype that makes types with many IDs a lot clearer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WeightId(pub u16);

/// The opcode IDs for the possible commands in the definitions section of an MDL0 file.
#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::FromRepr)]
#[repr(u8)]
pub enum DefinitionOpCodeId {
    Nop = 0x00,
    End = 0x01,
    MapNode = 0x02,
    /// Also referred to as `NodeMix`.
    Weights = 0x03,
    Draw = 0x04,
    WeightIndex = 0x05,
    DuplicateMatrix = 0x06,
}

/// Maps a bone index to a matrix index.
///
/// This is contained in the bytecode section of the file. It assigns a transformation
/// matrix to each of the bones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapNode {
    pub bone_index: BoneId,
    pub matrix_index: MatrixId,
}

impl MapNode {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let bone_index = BoneId(reader.read_u16::<BigEndian>()?);
        let matrix_index = MatrixId(reader.read_u16::<BigEndian>()?);

        Ok(Self {
            bone_index,
            matrix_index,
        })
    }

    fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.reserve(4);

        writer.write_u16::<BigEndian>(self.bone_index.0)?;
        writer.write_u16::<BigEndian>(self.matrix_index.0)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoneWeight {
    /// The bone that this weight affects.
    pub bone_id: BoneId,
    /// A value between 0.0 and 1.0 that determines how much geometry "sticks" to this bone
    /// when moving. A vertex can be affected by multiple bones. The weights must add up to 1.0.
    pub weight: f32,
}

impl BoneWeight {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let bone_id = BoneId(reader.read_u16::<BigEndian>()?);
        let weight = reader.read_f32::<BigEndian>()?;

        Ok(Self { bone_id, weight })
    }

    fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        // memory is reserved in `Weights` instead.
        writer.write_u16::<BigEndian>(self.bone_id.0)?;
        writer.write_f32::<BigEndian>(self.weight)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Weights {
    pub id: WeightId,
    pub weights: Vec<BoneWeight>,
}

impl Weights {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let id = WeightId(reader.read_u16::<BigEndian>()?);
        let weight_count = reader.read_u8()?;

        let mut weights = Vec::with_capacity(weight_count as usize);
        for _ in 0..weight_count {
            weights.push(BoneWeight::deserialize(reader)?);
        }

        Ok(Self { id, weights })
    }

    fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        let weights_len = self.weights.len();
        verify!(
            weights_len < u8::MAX as usize,
            "Weight count {weights_len} exceeds maximum of 255"
        );

        writer.reserve(2 + 1 + weights_len * 6);

        writer.write_u16::<BigEndian>(self.id.0)?;
        writer.write_u8(self.weights.len() as u8)?;
        for weight in &self.weights {
            weight.serialize(writer)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Draw {
    pub material_index: u16,
    pub object_index: u16,
    pub bone_index: BoneId,
    pub z_index: u8,
}

impl Draw {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let material_index = reader.read_u16::<BigEndian>()?;
        let object_index = reader.read_u16::<BigEndian>()?;
        let bone_index = BoneId(reader.read_u16::<BigEndian>()?);
        let priority = reader.read_u8()?;

        Ok(Self {
            material_index,
            object_index,
            bone_index,
            z_index: priority,
        })
    }

    fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.reserve(7);

        writer.write_u16::<BigEndian>(self.material_index)?;
        writer.write_u16::<BigEndian>(self.object_index)?;
        writer.write_u16::<BigEndian>(self.bone_index.0)?;
        writer.write_u8(self.z_index)?;

        Ok(())
    }
}

/// Maps a matrix to its corresponding weights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightIndex {
    pub matrix_id: MatrixId,
    pub weight_id: WeightId,
}

impl WeightIndex {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let matrix_id = MatrixId(reader.read_u16::<BigEndian>()?);
        let weight_id = WeightId(reader.read_u16::<BigEndian>()?);

        Ok(Self {
            matrix_id,
            weight_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateMatrix {
    pub dest: u16,
    pub src: u16,
}

impl DuplicateMatrix {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let dest = reader.read_u16::<BigEndian>()?;
        let src = reader.read_u16::<BigEndian>()?;

        Ok(Self { dest, src })
    }

    fn serialize(&self, writer: &mut MutCursor) -> SlipstreamResult<()> {
        writer.reserve(4);

        let dest = writer.write_u16::<BigEndian>(self.dest)?;
        let src = writer.write_u16::<BigEndian>(self.src)?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BytecodeCommand {
    MapNode(MapNode),
    Weights(Weights),
    Draw(Draw),
    WeightIndex(WeightIndex),
    DuplicateMatrix(DuplicateMatrix),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Definitions {
    pub commands: Vec<BytecodeCommand>,
}

impl Definitions {
    fn read_opcode_id(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<DefinitionOpCodeId> {
        let byte = reader.read_u8()?;
        DefinitionOpCodeId::from_repr(byte).ok_or_else(|| {
            CorruptionError {
                reason: String::from("invalid definition opcode ID"),
                location: Some(reader.position()),
            }
            .into()
        })
    }
}

impl Visitable for Definitions {
    fn accept(&self, node: VisitorContextNode<'_>, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_definitions(VisitorContext::new(node, self))
    }

    fn accept_mut(&mut self, node: VisitorContextNodeMut<'_>, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_definitions_mut(VisitorContextMut::new(node, self))
    }
}

impl DeserializeContents for Definitions {
    const NAME: &str = "Definitions";
    const KIND: IrNodeType = IrNodeType::Definitions;

    #[tracing::instrument(skip_all)]
    fn deserialize_contents(
        reader: &mut RefCursor<[u8]>
    ) -> SlipstreamResult<Self> {
        let mut commands = Vec::new();

        let mut opcode = Self::read_opcode_id(reader)?;
        while opcode != DefinitionOpCodeId::End {
            let command = match opcode {
                DefinitionOpCodeId::Nop => {
                    opcode = Self::read_opcode_id(reader)?;

                    continue;
                }
                DefinitionOpCodeId::MapNode => {
                    BytecodeCommand::MapNode(MapNode::deserialize(reader)?)
                }
                DefinitionOpCodeId::Weights => {
                    BytecodeCommand::Weights(Weights::deserialize(reader)?)
                }
                DefinitionOpCodeId::Draw => BytecodeCommand::Draw(Draw::deserialize(reader)?),
                DefinitionOpCodeId::WeightIndex => {
                    BytecodeCommand::WeightIndex(WeightIndex::deserialize(reader)?)
                }
                DefinitionOpCodeId::DuplicateMatrix => {
                    BytecodeCommand::DuplicateMatrix(DuplicateMatrix::deserialize(reader)?)
                }
                _ => unreachable!(),
            };

            commands.push(command);
            opcode = Self::read_opcode_id(reader)?;
        }

        Ok(Self { commands })
    }
}

impl SerializeContents for Definitions {
    fn serialize_contents(
        &self,
        writer: &mut slipstream_shared::cursor::MutCursor,
    ) -> SlipstreamResult<()> {
        todo!()
    }
}

impl SizeEstimate for Definitions {
    #[inline]
    fn estimate_size(&self) -> usize {
        // I chose 5 bytes as a reasonable average size for commands.
        self.commands.len() * 5
    }
}
