use std::ops::ControlFlow;

use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::cursor::RefCursor;
use slipstream_shared::error::{CorruptionError, SlipstreamResult};

use crate::node::node::IrNodeType;
use crate::mdl0::section::DeserializeSection;
use crate::visitor::{Visitable, Visitor};

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
    pub bone_index: u16,
    pub matrix_index: u16,
}

impl MapNode {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let bone_index = reader.read_u16::<BigEndian>()?;
        let matrix_index = reader.read_u16::<BigEndian>()?;

        Ok(Self {
            bone_index,
            matrix_index,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Weight {
    pub bone_id: u16,
    pub weight: f32,
}

impl Weight {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let bone_id = reader.read_u16::<BigEndian>()?;
        let weight = reader.read_f32::<BigEndian>()?;

        Ok(Self { bone_id, weight })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Weights {
    pub weight_id: u16,
    pub weights: Vec<Weight>,
}

impl Weights {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let weight_id = reader.read_u16::<BigEndian>()?;
        let weight_count = reader.read_u8()?;

        let mut weights = Vec::with_capacity(weight_count as usize);
        for _ in 0..weight_count {
            weights.push(Weight::deserialize(reader)?);
        }

        Ok(Self { weight_id, weights })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Draw {
    pub material_index: u16,
    pub object_index: u16,
    pub bone_index: u16,
    pub z_index: u8,
}

impl Draw {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let material_index = reader.read_u16::<BigEndian>()?;
        let object_index = reader.read_u16::<BigEndian>()?;
        let bone_index = reader.read_u16::<BigEndian>()?;
        let priority = reader.read_u8()?;

        Ok(Self {
            material_index,
            object_index,
            bone_index,
            z_index: priority,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightIndex {
    pub matrix_id: u16,
    pub weight_index: u16,
}

impl WeightIndex {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let matrix_id = reader.read_u16::<BigEndian>()?;
        let weight_index = reader.read_u16::<BigEndian>()?;

        Ok(Self {
            matrix_id,
            weight_index,
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
        dbg!(byte);

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
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()> {
        visitor.visit_definitions(self)
    }
}

impl DeserializeSection for Definitions {
    const NAME: &str = "Definitions";
    const KIND: IrNodeType = IrNodeType::Definitions;

    fn deserialize_section(
        reader: &mut RefCursor<[u8]>,
        _header_start: u64,
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
