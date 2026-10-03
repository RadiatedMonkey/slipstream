use slipstream_ir::mdl0::definitions::{
    BoneId, BoneWeight, BytecodeCommand, Definitions, MatrixId, WeightId,
};
use slipstream_shared::SlipstreamResult;
use std::collections::HashMap;

/// Maps bones to matrices in the matrix table.
#[derive(Default, Debug)]
pub struct BoneMap {
    /// Maps bones to their corresponding matrices.
    map: HashMap<BoneId, MatrixId>,
}

impl BoneMap {
    pub fn get_matrix(&self, bone_index: BoneId) -> Option<MatrixId> {
        self.map.get(&bone_index).copied()
    }

    pub fn from_definitions(definitions: &Definitions) -> SlipstreamResult<Self> {
        let mut map = HashMap::with_capacity(definitions.commands.len());
        for cmd in &definitions.commands {
            match cmd {
                BytecodeCommand::MapNode(mapping) => {
                    map.insert(mapping.bone_index, mapping.matrix_index);
                }
                _ => tracing::warn!("Unexpected command `{cmd:?}` in `NodeTree`, skipping it"),
            }
        }

        Ok(Self { map })
    }
}

#[derive(Default, Debug)]
pub struct BoneWeights {
    // Maps a matrix to its corresponding entry in `weights`.
    pub indices: HashMap<MatrixId, WeightId>,
    weights: HashMap<WeightId, Vec<BoneWeight>>,
}

impl BoneWeights {
    pub fn get_by_matrix_id(&self, matrix: MatrixId) -> Option<&[BoneWeight]> {
        let index = self.indices.get(&matrix)?;
        self.weights.get(index).map(|v| v.as_slice())
    }

    pub fn get_by_weight_id(&self, weight: WeightId) -> Option<&[BoneWeight]> {
        self.weights.get(&weight).map(|v| v.as_slice())
    }

    pub fn from_definitions(definitions: &Definitions) -> SlipstreamResult<Self> {
        let mut index_map = HashMap::new();
        let mut weight_map = HashMap::new();

        for cmd in &definitions.commands {
            match cmd {
                BytecodeCommand::Weights(weights) => {
                    weight_map.insert(weights.id, weights.weights.clone());

                    let weight_sum = weights.weights.iter().fold(0.0, |acc, w| acc + w.weight);

                    if weight_sum != 1.0 {
                        tracing::error!(
                            "Weights of ID {} do not add up to 1.0 ({weight_sum})",
                            weights.id.0
                        );
                    }
                }
                BytecodeCommand::WeightIndex(index) => {
                    index_map.insert(index.matrix_id, index.weight_id);
                }
                _ => tracing::warn!("Unexpected command `{cmd:?}` in `NodeMix`, skipping it"),
            }
        }

        Ok(Self {
            indices: index_map,
            weights: weight_map,
        })
    }
}
