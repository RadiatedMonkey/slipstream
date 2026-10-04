//! Translates between Wii models and wgpu ones.

use crate::viewer::pipeline::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT, TARGET_FORMAT};
use crate::viewer::translation::{IntermediateModel, ModelContents};
use slipstream_ir::gx::GxOpCode;
use slipstream_ir::gx::draw::{
    DrawOpCode, InlineNormal, InlinePosition, NormalData, NormalIndex, OpVertex, PositionData,
};
use slipstream_ir::mdl0::definitions::{
    BoneId, DRAW_OPA_NAME, Definitions, MatrixId, NODE_MIX_NAME, NODE_TREE_NAME, WeightId,
};
use slipstream_ir::mdl0::normals::NormalBuffer;
use slipstream_ir::mdl0::polygon::{BoneBind, Polygon};
use slipstream_ir::mdl0::vertices::VertexBuffer;
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_ir::node::guard::ContentReadGuard;
use slipstream_ir::node::node::IrNodeType;
use slipstream_ir::visitor::{Visitable, Visitor, VisitorContext};
use slipstream_shared::error::{InvalidInputError, SlipstreamError, SlipstreamResult};
use slipstream_shared::{try_unwrap, verify};
use std::collections::HashMap;
use std::ops::{ControlFlow, Deref};
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct VertexKey {
    /// The matrix that transforms this vertex.
    pub transform: Option<u8>,
    pub position: VertexAttrKey,
    pub normal: VertexAttrKey,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum VertexAttrKey {
    #[default]
    NotPresent,
    Indexed(u16),
    Inline(u16),
}

type VertexIndex = u16;

pub const MAX_BONE_INFLUENCES: usize = 4;

#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct TranslatedVertex {
    /// The position of the vertex.
    pub position: [f32; 3],
    /// The normal of the vertex.
    pub normal: [f32; 3],
    pub bone_indices: [u32; MAX_BONE_INFLUENCES],
    pub bone_weights: [f32; MAX_BONE_INFLUENCES],
}

#[derive(Default, Debug)]
pub struct IntermediatePolygon {
    /// Maps a vertex index to a location in `vertices`.
    pub map: HashMap<VertexKey, VertexIndex>,
    /// This will become the new index buffer.
    pub indices: Vec<VertexIndex>,
    /// This will become the new vertex buffer.
    pub vertices: Vec<TranslatedVertex>,
    /// List of matrix IDs. The vertices index into this array to find the matrices
    /// that transform them.
    pub bone_translation: Vec<MatrixId>,
    /// Buffer of positions that are stored inline in the draw command.
    pub inline_positions: Vec<[f32; 3]>,
    /// Buffer of normals that are stored inline in the draw command.
    pub inline_normals: Vec<[f32; 3]>,
}

impl IntermediatePolygon {
    pub fn insert_inline_position(&mut self, position: [f32; 3]) -> VertexAttrKey {
        self.inline_positions.push(position);
        VertexAttrKey::Inline(self.inline_positions.len() as u16 - 1)
    }

    pub fn insert_inline_normal(&mut self, normal: [f32; 3]) -> VertexAttrKey {
        self.inline_normals.push(normal);
        VertexAttrKey::Inline(self.inline_normals.len() as u16 - 1)
    }
}

impl ModelContents<'_> {
    fn try_inspect_positions<F, T>(&self, index: usize, inspect_fn: F) -> SlipstreamResult<T>
    where
        F: FnOnce(&VertexBuffer) -> SlipstreamResult<T>,
    {
        let key = try_unwrap!(self.vertices.get(index), "vertex buffer index out of range")?;

        self.try_inspect_inner(*key, inspect_fn)
    }

    fn try_inspect_normals<F, T>(&self, index: usize, inspect_fn: F) -> SlipstreamResult<T>
    where
        F: FnOnce(&NormalBuffer) -> SlipstreamResult<T>,
    {
        let key = try_unwrap!(self.normals.get(index), "normal buffer index out of range")?;

        self.try_inspect_inner(*key, inspect_fn)
    }

    fn translate_vertex(
        &self,
        model: &IntermediateModel,
        scratch: &IntermediatePolygon,
        polygon: &Polygon,
        vertex_key: &VertexKey,
    ) -> SlipstreamResult<TranslatedVertex> {
        const POSITION_DEFAULT: [f32; 3] = [0.0; 3];
        const NORMAL_DEFAULT: [f32; 3] = [0.0, 1.0, 0.0];
        const WEIGHTS_DEFAULT: [f32; MAX_BONE_INFLUENCES] = {
            let mut def = [0.0; MAX_BONE_INFLUENCES];
            def[0] = 1.0;
            def
        };

        let (bone_indices, bone_weights) = match &polygon.bone_bind {
            BoneBind::Rigid(rigid) => {
                let mut bone_indices = [0; MAX_BONE_INFLUENCES];
                bone_indices[0] = *rigid;

                (bone_indices, WEIGHTS_DEFAULT)
            }
            BoneBind::Mixed(mixed) => {
                let pn_id = if let Some(id) = vertex_key.transform {
                    id
                } else {
                    tracing::error!("Missing GX_VA_PNMTXIDX for vertex, attaching it to matrix 0");
                    0
                };

                let bone_id = *try_unwrap!(
                    mixed.entries.get(pn_id as usize),
                    "bone table index out of range: {pn_id}"
                )?;

                let matrix_id = try_unwrap!(
                    model.bone_map.get_matrix(BoneId(bone_id)),
                    "bone map entry out of range: {bone_id}"
                )?;

                // Check if weights are involved
                match &model.bone_weights {
                    Some(weights) => {
                        let mut resolved = try_unwrap!(
                            weights.get_by_matrix_id(matrix_id),
                            "bone weights lookup out of range: {matrix_id:?}"
                        )?
                        .to_vec();

                        tracing::info!(
                            pn_id = ?pn_id,
                            bone_id = ?bone_id,
                            matrix_id = ?matrix_id,
                            influences = ?resolved,
                            "vertex bone mapping"
                        );

                        if resolved.len() > MAX_BONE_INFLUENCES {
                            tracing::warn!(
                                "vertex has {} bone influences, truncated to {MAX_BONE_INFLUENCES} strongest influences",
                                resolved.len()
                            );

                            resolved.sort_unstable_by(|left, right| {
                                right.weight.total_cmp(&left.weight)
                            });
                        }

                        let mut bone_indices = [0; MAX_BONE_INFLUENCES];
                        let mut bone_weights = WEIGHTS_DEFAULT;

                        let mut sum = 0.0;
                        for (i, infl) in resolved.iter().take(MAX_BONE_INFLUENCES).enumerate() {
                            tracing::info!(
                                influence_bone = ?infl.bone_id,
                                influence_weight = ?infl.weight
                            );

                            bone_indices[i] = infl.bone_id.0 as u32;
                            bone_weights[i] = infl.weight;
                            sum += infl.weight;
                        }

                        if sum != 1.0 {
                            tracing::warn!(
                                "vertex weights do not add up to 1.0, normalizing the weights..."
                            );

                            // Then normalize the influences back to a sum of 1.0
                            let factor = 1.0 / sum;
                            for weight in &mut bone_weights {
                                *weight *= factor;
                            }
                        }

                        (bone_indices, bone_weights)
                    }
                    None => {
                        // The polygon has no bone table, so we assume every matrix index is a
                        // global index already.
                        //
                        // tracing::error!(
                        //     "Polygon has mixed bone bind but model does not specify bone weights"
                        // );

                        tracing::info!(
                            pn_id = ?pn_id,
                            bone_id = ?bone_id,
                            matrix_id = ?matrix_id,
                            "vertex bone mapping"
                        );
                        
                        let mut bone_indices = [0; MAX_BONE_INFLUENCES];
                        bone_indices[0] = bone_id as u32;

                        (bone_indices, WEIGHTS_DEFAULT)
                    }
                }
            }
        };

        let position = match vertex_key.position {
            VertexAttrKey::NotPresent => POSITION_DEFAULT,
            VertexAttrKey::Indexed(idx) => {
                self.try_inspect_positions(polygon.vertex_array_id as usize, |buf| {
                    try_unwrap!(
                        buf.get_xyz(idx as usize),
                        "vertex {idx} did not exist in vertex buffer"
                    )
                })?
            }
            VertexAttrKey::Inline(idx) => *scratch
                .inline_positions
                .get(idx as usize)
                .expect("inline position index out of range"),
        };

        let normal = match vertex_key.normal {
            VertexAttrKey::NotPresent => NORMAL_DEFAULT,
            VertexAttrKey::Indexed(idx) => {
                self.try_inspect_normals(polygon.normal_array_id as usize, |buf| {
                    try_unwrap!(
                        buf.get_normal(idx as usize),
                        "normal {idx} did not exist in normal buffer"
                    )
                })?
            }
            VertexAttrKey::Inline(idx) => *scratch
                .inline_normals
                .get(idx as usize)
                .expect("inline normal index out of range"),
        };

        Ok(TranslatedVertex {
            position,
            normal,
            bone_indices,
            bone_weights,
        })
    }

    fn resolve_vertex(
        &self,
        model: &IntermediateModel,
        scratch: &mut IntermediatePolygon,
        polygon: &Polygon,
        vertex: &OpVertex,
    ) -> SlipstreamResult<crate::viewer::translation::vertex::VertexIndex> {
        let mut vertex_key = VertexKey::default();

        vertex_key.transform = vertex.pn_matrix_index;

        match &vertex.position {
            PositionData::NotPresent => vertex_key.position = VertexAttrKey::NotPresent,
            PositionData::Index8(idx) => vertex_key.position = VertexAttrKey::Indexed(*idx as u16),
            PositionData::Index16(idx) => vertex_key.position = VertexAttrKey::Indexed(*idx),
            PositionData::Direct(x) => {
                let position = match x {
                    InlinePosition::Xy(xy) => [xy[0], xy[1], 0.0],
                    InlinePosition::Xyz(xyz) => *xyz,
                };

                vertex_key.position = scratch.insert_inline_position(position);
            }
        }

        match &vertex.normals {
            NormalData::NotPresent => vertex_key.normal = VertexAttrKey::NotPresent,
            NormalData::Index8(idx) => match idx {
                NormalIndex::Single(x) => vertex_key.normal = VertexAttrKey::Indexed(*x as u16),
                NormalIndex::Triple(x) => vertex_key.normal = VertexAttrKey::Indexed(x[0] as u16),
            },
            NormalData::Index16(idx) => match idx {
                NormalIndex::Single(x) => vertex_key.normal = VertexAttrKey::Indexed(*x),
                NormalIndex::Triple(x) => vertex_key.normal = VertexAttrKey::Indexed(x[0]),
            },
            NormalData::Direct(x) => {
                let normal = match x {
                    InlineNormal::Single(x) => *x,
                    InlineNormal::Packed(x) => [x[0], x[1], x[2]],
                };

                vertex_key.normal = scratch.insert_inline_normal(normal);
            }
        }

        // Check if the index has already been seen before. In case it has not
        // a new index will be generated.
        let vertex_index = match scratch.map.get(&vertex_key) {
            Some(&x) => x,
            None => {
                let translated = self.translate_vertex(model, scratch, polygon, &vertex_key)?;
                scratch.vertices.push(translated);

                let index =
                    scratch.vertices.len() as crate::viewer::translation::vertex::VertexIndex - 1;
                scratch.map.insert(vertex_key, index);
                index
            }
        };

        Ok(vertex_index)
    }

    fn resolve_triangle_list(
        &self,
        model: &IntermediateModel,
        scratch: &mut IntermediatePolygon,
        polygon: &Polygon,
        vertices: &[OpVertex],
    ) -> SlipstreamResult<()> {
        scratch.indices.reserve(vertices.len());
        for vertex in vertices {
            let resolved = self.resolve_vertex(model, scratch, polygon, vertex)?;
            scratch.indices.push(resolved);
        }

        Ok(())
    }

    fn resolve_triangle_strip(
        &self,
        model: &IntermediateModel,
        scratch: &mut IntermediatePolygon,
        polygon: &Polygon,
        vertices: &[OpVertex],
    ) -> SlipstreamResult<()> {
        let expanded_len = (vertices.len() - 2) * 3;

        scratch.indices.reserve(expanded_len);
        for (i, [v1, v2, v3]) in vertices.array_windows().enumerate() {
            let r1 = self.resolve_vertex(model, scratch, polygon, v1)?;
            let r2 = self.resolve_vertex(model, scratch, polygon, v2)?;
            let r3 = self.resolve_vertex(model, scratch, polygon, v3)?;

            if i % 2 == 0 {
                scratch.indices.extend([r1, r2, r3]);
            } else {
                scratch.indices.extend([r2, r1, r3]);
            }
        }

        Ok(())
    }

    pub fn translate_polygon(
        &self,
        model: &IntermediateModel,
        scratch: &mut IntermediatePolygon,
        polygon: &Polygon,
    ) -> SlipstreamResult<()> {
        for call in &polygon.vertex_data_gx.commands {
            match call {
                GxOpCode::DrawTriangles(DrawOpCode { vertices }) => {
                    self.resolve_triangle_list(model, scratch, polygon, vertices)?
                }
                GxOpCode::DrawTriangleStrip(DrawOpCode { vertices }) => {
                    self.resolve_triangle_strip(model, scratch, polygon, vertices)?
                }
                _ => {}
            }
        }

        Ok(())
    }
}
