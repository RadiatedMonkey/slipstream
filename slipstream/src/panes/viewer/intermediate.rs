//! Translates between Wii models and wgpu ones.

use crate::panes::viewer::pipeline::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT, TARGET_FORMAT};
use slipstream_ir::gx::GxOpCode;
use slipstream_ir::gx::draw::{DrawOpCode, InlineNormal, InlinePosition, OpVertex, PositionData};
use slipstream_ir::mdl0::normals::NormalBuffer;
use slipstream_ir::mdl0::polygon::Polygon;
use slipstream_ir::mdl0::vertices::VertexBuffer;
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_ir::node::guard::ContentReadGuard;
use slipstream_ir::node::node::IrNodeType;
use slipstream_ir::visitor::{Visitable, Visitor, VisitorContext};
use slipstream_shared::error::{InvalidInputError, SlipstreamError, SlipstreamResult};
use slipstream_shared::{try_unwrap, verify};
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct VertexKey {
    pub position: VertexAttrKey,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum VertexAttrKey {
    #[default]
    NotPresent,
    Indexed(u16),
    Inline(u16),
}

#[derive(Debug)]
pub struct DrawableModel {}

type VertexIndex = u16;

#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct TranslatedVertex {
    pub position: [f32; 3],
}

#[derive(Default, Debug)]
pub struct IntermediateModel {
    pub polygons: Vec<IntermediatePolygon>,
}

#[derive(Default, Debug)]
pub struct IntermediatePolygon {
    /// Maps a vertex index to a location in `vertices`.
    pub map: HashMap<VertexKey, VertexIndex>,
    /// This will become the new index buffer.
    pub indices: Vec<VertexIndex>,
    /// This will become the new vertex buffer.
    pub vertices: Vec<TranslatedVertex>,
    pub inline_positions: Vec<[f32; 3]>,
}

impl IntermediatePolygon {
    pub fn insert_inline_position(&mut self, position: [f32; 3]) -> VertexAttrKey {
        self.inline_positions.push(position);
        VertexAttrKey::Inline(self.inline_positions.len() as u16 - 1)
    }
}

#[derive(Clone)]
pub struct ModelTranslator<'a> {
    arena: &'a IrArena,
    vertices: Vec<IrNodeKey>,
    normals: Vec<IrNodeKey>,
    polygons: Vec<IrNodeKey>,
}

impl<'a> ModelTranslator<'a> {
    pub fn from_root(root: IrNodeKey, arena: &'a IrArena) -> SlipstreamResult<Self> {
        let mut visitor = Self {
            arena,
            vertices: Vec::new(),
            normals: Vec::new(),
            polygons: Vec::new(),
        };
        arena.walk(root, &mut visitor)?;
        Ok(visitor)
    }

    /// Retrieves the given key from the map, downcasts it to `U`
    /// and runs `inspect_fn` on it.
    fn try_inspect_inner<F, T, U>(&self, key: IrNodeKey, inspect_fn: F) -> SlipstreamResult<T>
    where
        U: Visitable,
        F: FnOnce(&U) -> SlipstreamResult<T>,
    {
        let out = self
            .arena
            .inspect(key, |node| {
                let buf = try_unwrap!(
                    node.contents.get_or_try_init()?,
                    "vertex buffer had no content"
                )?;

                let buf = try_unwrap!(
                    buf.as_any().downcast_ref::<U>(),
                    "vertex buffer had an incorrect `Visitable` type"
                )?;

                inspect_fn(buf)
            })
            .transpose()?;

        try_unwrap!(out, "vertex buffer {key:?} did not exist")
    }

    fn try_inspect_positions<F, T>(&self, index: usize, inspect_fn: F) -> SlipstreamResult<T>
    where
        F: FnOnce(&VertexBuffer) -> SlipstreamResult<T>,
    {
        let key = try_unwrap!(self.vertices.get(index), "vertex buffer index out of range")?;

        self.try_inspect_inner(*key, inspect_fn)
    }

    fn translate_vertex(
        &self,
        scratch: &IntermediatePolygon,
        polygon: &Polygon,
        vertex_key: &VertexKey,
    ) -> SlipstreamResult<TranslatedVertex> {
        const POSITION_DEFAULT: [f32; 3] = [0.0; 3];

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

        Ok(TranslatedVertex { position })
    }

    fn resolve_vertex(
        &self,
        scratch: &mut IntermediatePolygon,
        polygon: &Polygon,
        vertex: &OpVertex,
    ) -> SlipstreamResult<VertexIndex> {
        let mut vertex_key = VertexKey::default();
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

        // Check if the index has already been seen before. In case it has not
        // a new index will be generated.
        let vertex_index = match scratch.map.get(&vertex_key) {
            Some(&x) => x,
            None => {
                let translated = self.translate_vertex(scratch, polygon, &vertex_key)?;
                scratch.vertices.push(translated);

                let index = scratch.vertices.len() as VertexIndex - 1;
                scratch.map.insert(vertex_key, index);
                index
            }
        };

        Ok(vertex_index)
    }

    #[tracing::instrument(skip_all, fields(vertex_count = vertices.len()))]
    fn resolve_triangle_list(
        &self,
        scratch: &mut IntermediatePolygon,
        polygon: &Polygon,
        vertices: &[OpVertex],
    ) -> SlipstreamResult<()> {
        scratch.indices.reserve(vertices.len());
        for vertex in vertices {
            let resolved = self.resolve_vertex(scratch, polygon, vertex)?;
            scratch.indices.push(resolved);
        }

        Ok(())
    }

    #[tracing::instrument(skip_all, fields(vertex_count = vertices.len()))]
    fn resolve_triangle_strip(
        &self,
        scratch: &mut IntermediatePolygon,
        polygon: &Polygon,
        vertices: &[OpVertex],
    ) -> SlipstreamResult<()> {
        let expanded_len = (vertices.len() - 2) * 3;

        scratch.indices.reserve(expanded_len);
        for (i, [v1, v2, v3]) in vertices.array_windows().enumerate() {
            let r1 = self.resolve_vertex(scratch, polygon, v1)?;
            let r2 = self.resolve_vertex(scratch, polygon, v2)?;
            let r3 = self.resolve_vertex(scratch, polygon, v3)?;

            if i % 2 == 0 {
                scratch.indices.extend([r1, r2, r3]);
            } else {
                scratch.indices.extend([r2, r1, r3]);
            }
        }

        Ok(())
    }

    fn translate_polygon(
        &self,
        scratch: &mut IntermediatePolygon,
        polygon: &Polygon,
    ) -> SlipstreamResult<()> {
        for call in &polygon.vertex_data_gx.commands {
            match call {
                GxOpCode::DrawTriangles(DrawOpCode { vertices }) => {
                    self.resolve_triangle_list(scratch, polygon, vertices)?
                }
                GxOpCode::DrawTriangleStrip(DrawOpCode { vertices }) => {
                    self.resolve_triangle_strip(scratch, polygon, vertices)?
                }
                _ => tracing::error!("TODO"),
            }
        }

        Ok(())
    }

    /// Converts the raw buffers to an [`IntermediateModel`].
    ///
    /// This intermediate model can then be converted into wgpu buffers and commands
    /// in the next step.
    pub fn to_intermediate(&self, arena: &IrArena) -> SlipstreamResult<IntermediateModel> {
        struct PolygonVisitor<'a> {
            model: &'a ModelTranslator<'a>,
            scratch: &'a mut IntermediatePolygon,
            result: SlipstreamResult<()>,
        }

        impl Visitor for PolygonVisitor<'_> {
            fn visit_polygon(&mut self, context: VisitorContext<'_, Polygon>) -> ControlFlow<()> {
                tracing::trace!("Translating `{}`", context.meta.label);
                self.result = self.model.translate_polygon(self.scratch, context.content);

                ControlFlow::Break(())
            }
        }

        let mut intermediate = Vec::with_capacity(self.polygons.len());
        for &polygon in &self.polygons {
            let mut scratch = IntermediatePolygon::default();

            let mut visitor = PolygonVisitor {
                model: self,
                scratch: &mut scratch,
                result: Ok(()),
            };
            let _ = arena.visit(polygon, &mut visitor);
            visitor.result?;

            intermediate.push(scratch);
        }

        Ok(IntermediateModel {
            polygons: intermediate,
        })
    }
}

impl Visitor for ModelTranslator<'_> {
    fn visit_vertices(&mut self, vertices: VisitorContext<'_, VertexBuffer>) -> ControlFlow<()> {
        self.vertices.push(vertices.meta.key);
        ControlFlow::Break(())
    }

    fn visit_normals(&mut self, normals: VisitorContext<'_, NormalBuffer>) -> ControlFlow<()> {
        self.normals.push(normals.meta.key);
        ControlFlow::Break(())
    }

    fn visit_polygon(&mut self, polygon: VisitorContext<'_, Polygon>) -> ControlFlow<()> {
        self.polygons.push(polygon.meta.key);
        ControlFlow::Break(())
    }
}
