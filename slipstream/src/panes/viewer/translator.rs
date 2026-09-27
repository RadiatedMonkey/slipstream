//! Translates between Wii models and wgpu ones.

use bytemuck::Zeroable;
use parking_lot::{
    ArcRwLockReadGuard, MappedMutexGuard, MappedRwLockReadGuard, Mutex, MutexGuard, RwLockReadGuard,
};
use slipstream_ir::gx::GxOpCode;
use slipstream_ir::gx::draw::{DrawOpCode, InlineNormal, InlinePosition, NormalData, OpVertex, PositionData};
use slipstream_ir::mdl0::normals::NormalBuffer;
use slipstream_ir::mdl0::polygon::Polygon;
use slipstream_ir::mdl0::vertices::VertexBuffer;
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_ir::node::node::IrNodeType;
use slipstream_shared::error::{InvalidInputError, SlipstreamError, SlipstreamResult};
use std::sync::Arc;
use std::{any::Any, borrow::Cow, collections::HashMap};
use wgpu::util::DeviceExt;

use crate::panes::viewer::pipeline::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT, TARGET_FORMAT};

/// A vertex with all data interleaved.
///
/// While the Wii stores every single buffer type separately and uses separate index buffer,
/// this is not actually possible on modern hardware. We solve this by creating wgpu compatible
/// vertex buffers with all data interleaved. All buffer indices are resolved and turned into a single index buffer.
///
/// Some data might be upgraded to higher quality formats automatically as this prevents having to support
/// many different formats in the shaders.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct InterleavedVertex {
    /// Position data can be stored in either 2 or 3 component format.
    /// 2 component formats are automatically upgraded to 3 components by setting the Z component to 0.
    position: [f32; 3],
    /// The MDL0 file format either stores a single normal or the normal + tangent + binormal.
    /// The translator discards the binormal and always stores the normal and tangent. If the
    /// normal buffer did not contain tangents, a dummy value will be used.
    ///
    /// Discarding the binormal reduces buffer size, since the binormal can easy be calculated from a cross
    /// product between the normal and tangent.
    normals: [f32; 3],
}

pub type IndexTy = u32;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum IndexAttrKey {
    /// The attribute was stored as an index into a buffer.
    Physical(IndexTy),
    /// The attribute was stored inline. It has been added to the direct map
    /// and a new synthetic index into the map has been generated.
    Synthetic(IndexTy),
    /// The attribute was marked as not present.
    #[default]
    NotPresent,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct IndexKey {
    pub position: IndexAttrKey,
    pub normals: IndexAttrKey,
}

#[derive(Debug, Default, Clone)]
pub struct InlineScratchBuffers {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

impl InlineScratchBuffers {
    pub fn insert_position(&mut self, position: InlinePosition) -> IndexAttrKey {
        self.positions.push(position.to_xyz());
        IndexAttrKey::Synthetic(self.positions.len() as u32 - 1)
    }

    pub fn insert_normal(&mut self, normal: InlineNormal) -> IndexAttrKey {
        todo!()
    }
}

/// Keeps track of all rendering pipelines.
///
/// The pipelines are stored by vertex buffer layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineDescriptor {
    layouts: Vec<Option<wgpu::VertexBufferLayout<'static>>>,
}

pub struct PipelineRegistry {
    device: wgpu::Device,
    pipelines: HashMap<PipelineDescriptor, wgpu::RenderPipeline>,
    layout: wgpu::PipelineLayout,
}

impl PipelineRegistry {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("registry pipeline layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        Self {
            device: device.clone(),
            pipelines: HashMap::new(),
            layout,
        }
    }

    pub fn create(&mut self, descriptor: PipelineDescriptor) {
        let module = self
            .device
            .create_shader_module(wgpu::include_wgsl!("../../../shaders/viewer.wgsl"));

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("model pipeline"),
                layout: Some(&self.layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &descriptor.layouts,
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[],
                        zero_initialize_workgroup_memory: false,
                    },
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[],
                        zero_initialize_workgroup_memory: false,
                    },
                    targets: &[Some(wgpu::ColorTargetState {
                        format: TARGET_FORMAT,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multisample: wgpu::MultisampleState {
                    count: MSAA_SAMPLE_COUNT,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });

        self.pipelines.insert(descriptor, pipeline);
    }

    pub fn get(&self, descriptor: &PipelineDescriptor) -> &wgpu::RenderPipeline {
        self.pipelines
            .get(descriptor)
            .expect("render pipeline did not exist")
    }
}

pub struct ModelPolygon {
    pipeline: wgpu::RenderPipeline,
    index_count: u32,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl ModelPolygon {
    pub const fn index_format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), self.index_format());
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    pub fn from_scratch(
        device: &wgpu::Device,
        shape: &InspectableReadGuard<Shape>,
        scratch: &PolygonScratch,
        pipeline_registry: &mut PipelineRegistry,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} vertex buffer", shape.label())),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&scratch.vertices),
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} index buffer", shape.label())),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(&scratch.indices),
        });

        let pipeline = pipeline_registry.get(&scratch.pipeline_descriptor);
        Self {
            pipeline: pipeline.clone(),
            vertex_buffer,
            index_buffer,
            index_count: scratch.indices.len() as u32,
        }
    }
}

pub struct DrawableModel {
    polygons: Vec<ModelPolygon>,
}

impl DrawableModel {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub struct VertexIndex(pub IndexTy);

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TranslationStats {
    pub reused_indices: usize,
}

#[derive(Debug, Clone)]
pub struct PolygonScratch {
    pub pipeline_descriptor: PipelineDescriptor,
    /// Maps MDL0's multi-indices into a single index pointing into the `vertices` array.
    ///
    /// This map ensures that we do not duplicate vertices and allows for easy
    /// building of an index buffer later.
    pub vertex_map: HashMap<IndexKey, VertexIndex>,
    pub vertices: Vec<InterleavedVertex>,
    /// The actual new index buffer,
    /// These indices point into the `vertices` array.
    pub indices: Vec<VertexIndex>,
    /// Direct draw call values are stored in here. They are given a new separate ID
    /// that is used to refer to them in the vertex map.
    pub direct_data: InlineScratchBuffers,
}

impl PolygonScratch {
    /// Creates the vertex buffer layout for the given shape.
    fn discover_layout(polygon: &Polygon) -> PipelineDescriptor {
        /// The basic layout that every pipeline will always contain.
        ///
        /// This contains just the position and the normal.
        const BASIC_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
            array_stride: 6 * size_of::<f32>() as u64,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // Normal
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 3 * size_of::<f32>() as u64,
                    shader_location: 1,
                },
            ],
            step_mode: wgpu::VertexStepMode::Vertex,
        };

        PipelineDescriptor {
            layouts: vec![Some(BASIC_LAYOUT)],
        }
    }

    pub fn new(polygon: &Polygon) -> Self {
        let layout = Self::discover_layout(polygon);

        Self {
            pipeline_descriptor: layout,
            vertex_map: HashMap::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            direct_data: InlineScratchBuffers::default(),
        }
    }

    /// Inserts a translated vertex into the scratch buffer.
    pub fn insert_vertex(&mut self, vertex: InterleavedVertex) -> usize {
        self.vertices.push(vertex);
        self.vertices.len() - 1
    }
}

#[derive(Clone)]
pub struct ModelScratch {
    pub map: IrArena,

    pub positions: Vec<IrNodeKey>,
    pub normals: Vec<IrNodeKey>,
    pub colors: Vec<IrNodeKey>,
    pub uvs: Vec<IrNodeKey>,
    pub shapes: Vec<IrNodeKey>,
}

const POSITION_DEFAULT: [f32; 3] = [0.0; 3];
const NORMAL_DEFAULT: [f32; 3] = [0.0, 1.0, 0.0];

impl ModelScratch {
    pub fn new(map: IrArena) -> Self {
        Self {
            map,
            positions: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
            uvs: Vec::new(),
            shapes: Vec::new(),
        }
    }

    /// Creates a new interleaved vertex by resolving all indices in the key.
    fn generate_interleaved_vertex(
        &self,
        shape: &Shape,
        index_key: &IndexKey,
        scratch: &PolygonScratch,
    ) -> SlipstreamResult<InterleavedVertex> {
        let position = match index_key.position {
            IndexAttrKey::NotPresent => POSITION_DEFAULT,
            IndexAttrKey::Physical(idx) => {
                let position_buffer = self.get_vertices(shape.vertex_array_id as usize)?;
                position_buffer
                    .get_xyz(idx as usize)
                    .expect("vertex did not exist in position buffer")
            }
            IndexAttrKey::Synthetic(idx) => *scratch
                .direct_data
                .positions
                .get(idx as usize)
                .expect("position did not exist in direct data buffer"),
        };

        let normals = match index_key.normals {
            IndexAttrKey::NotPresent => NORMAL_DEFAULT,
            IndexAttrKey::Physical(idx) => {
                let normal_buffer = self.get_normals(shape.normal_array_id as usize)?;
                normal_buffer
                    .get_normal(idx as usize)
                    .expect("vertex did not exist in normal buffer")
            }
            IndexAttrKey::Synthetic(idx) => *scratch
                .direct_data
                .normals
                .get(idx as usize)
                .expect("normal did not exist in direct data buffer"),
        };

        Ok(InterleavedVertex { position, normals })
    }

    /// Resolves the given vertex, returning its assigned index.
    fn resolve_vertex(
        &self,
        polygon: &Polygon,
        vertex: &OpVertex,
        scratch: &mut PolygonScratch,
        stats: &mut TranslationStats,
    ) -> SlipstreamResult<VertexIndex> {
        let mut index_key = IndexKey::default();

        // Generate the index.

        match &vertex.position {
            PositionData::NotPresent => index_key.position = IndexAttrKey::NotPresent,
            PositionData::Index8(idx) => {
                index_key.position = IndexAttrKey::Physical(*idx as IndexTy)
            }
            PositionData::Index16(idx) => {
                index_key.position = IndexAttrKey::Physical(*idx as IndexTy)
            }
            PositionData::Direct(x) => {
                let sid = scratch.direct_data.insert_position(*x);
                index_key.position = sid;
            }
        }

        match &vertex.normals {
            NormalData::NotPresent => index_key.normals = IndexAttrKey::NotPresent,
            NormalData::Index8(idx) => match idx {
                NormalIndex::Single(idx) => {
                    index_key.normals = IndexAttrKey::Physical(*idx as IndexTy)
                }
                NormalIndex::Triple(idxs) => todo!("multi normal indices"),
            },
            NormalData::Index16(idx) => match idx {
                NormalIndex::Single(idx) => {
                    index_key.normals = IndexAttrKey::Physical(*idx as IndexTy)
                }
                NormalIndex::Triple(idxs) => todo!("multi normal indices"),
            },
            NormalData::Direct(x) => {
                let sid = scratch.direct_data.insert_normal(x.clone());
                index_key.normals = sid;
            }
        }

        // Check if the index has already been seen before. In case it hasn't
        // a new index will be generated.
        let translated_index = match scratch.vertex_map.get(&index_key) {
            // If the given multi index has already been seen before, load its translated
            // index.
            Some(&x) => {
                stats.reused_indices += 1;
                x
            }
            // Otherwise insert the index and its vertex data into the map.
            None => {
                let interleaved = self.generate_interleaved_vertex(polygon, &index_key, scratch)?;

                scratch.vertices.push(interleaved);
                let new_index = VertexIndex(scratch.vertices.len() as IndexTy - 1);

                scratch.vertex_map.insert(index_key.clone(), new_index);

                new_index
            }
        };

        Ok(translated_index)
    }

    /// Resolves a list of triangles.
    ///
    /// This is the most straightforward topology as it maps straight to what the editor's
    /// graphics pipeline uses.
    #[tracing::instrument(skip_all, fields(vertex_count = vertices.len()))]
    fn resolve_triangle_list(
        &self,
        polygon: &Polygon,
        vertices: &[OpVertex],
        scratch: &mut PolygonScratch,
        stats: &mut TranslationStats,
    ) -> SlipstreamResult<()> {
        tracing::trace!("Resolving a {} triangle list", vertices.len());

        scratch.indices.reserve(vertices.len());
        for vertex in vertices {
            let resolved = self.resolve_vertex(polygon, vertex, scratch, stats)?;
            scratch.indices.push(resolved);
        }

        Ok(())
    }

    /// Resolves a triangle strip topology.
    ///
    /// While wgpu also supports triangle strip topologies, the topology is converted into a
    /// basic triangle list. Since a render pipeline is only capable of rendering a single toplogy, this
    /// reduces the amount of pipelines that have to be created.
    #[tracing::instrument(skip_all, fields(vertex_count = vertices.len()))]
    fn resolve_triangle_strip(
        &self,
        shape: &InspectableReadGuard<Polygon>,
        vertices: &[OpVertex],
        scratch: &mut PolygonScratch,
        stats: &mut TranslationStats,
    ) -> SlipstreamResult<()> {
        tracing::trace!("Resolving a {} triangle strip", vertices.len());

        let expanded_len = (vertices.len() - 2) * 3;
        scratch.indices.reserve(expanded_len);

        for (i, [v1, v2, v3]) in vertices.array_windows().enumerate() {
            let r1 = self.resolve_vertex(shape, v1, scratch, stats)?;
            let r2 = self.resolve_vertex(shape, v2, scratch, stats)?;
            let r3 = self.resolve_vertex(shape, v3, scratch, stats)?;

            if i % 2 == 0 {
                // Even triangles should keep their original winding order.
                scratch.indices.extend([r1, r2, r3]);
            } else {
                // Odd triangle should have their first two vertices reversed.
                scratch.indices.extend([r2, r1, r3]);
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all, fields(shape = shape.label(), id = shape.id().into_inner()))]
    fn resolve_shape(
        &self,
        shape: &InspectableReadGuard<Shape>,
    ) -> SlipstreamResult<PolygonScratch> {
        let mut scratch = PolygonScratch::new(shape);
        let mut stats = TranslationStats::default();

        tracing::trace!(
            "Resolving {} draw calls",
            shape.vertex_data_gx.commands.len(),
        );

        for call in &shape.vertex_data_gx.commands {
            match call {
                GxOpCode::DrawTriangles(DrawOpCode { vertices }) => {
                    self.resolve_triangle_list(&shape, vertices, &mut scratch, &mut stats)?;
                }
                GxOpCode::DrawTriangleStrip(DrawOpCode { vertices }) => {
                    self.resolve_triangle_strip(&shape, vertices, &mut scratch, &mut stats)?;
                }
                _ => tracing::error!("unsupported opcode: {call:?}"),
            }
        }

        dbg!(&stats);

        Ok(scratch)
    }

    #[tracing::instrument(skip_all)]
    pub fn resolve_shapes(&self, device: &wgpu::Device) -> SlipstreamResult<DrawableModel> {
        tracing::trace!("Resolving {} shapes", self.shapes.len());

        let mut pipeline_registry = PipelineRegistry::new(device);
        let mut model = DrawableModel {
            polygons: Vec::with_capacity(self.shapes.len()),
        };

        for &shape_id in &self.shapes {
            let shape = self.map.get_inspectable::<Shape>(shape_id).ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: format!("virtual node {shape_id} did not exist"),
                    ..Default::default()
                })
            })?;

            let scratch = self.resolve_shape(&shape)?;
            let polygon =
                ModelPolygon::from_scratch(device, &shape, &scratch, &mut pipeline_registry);

            model.polygons.push(polygon);
        }

        Ok(model)
    }

    #[tracing::instrument(skip_all, fields(node))]
    pub fn from_root(node: IrNodeKey, map: IrArena) -> SlipstreamResult<Self> {
        tracing::trace!("Constructing model buffer block from MDL0 file");

        let mut bufs = Self::new(map.clone());

        let root_children = map.get_children(node).ok_or_else(|| {
            SlipstreamError::from(InvalidInputError {
                reason: format!("virtual node {node} did not exist"),
                ..Default::default()
            })
        })?;

        for &section_id in &root_children {
            let children = map
                .get_children(section_id)
                .expect("unable to find children of node");

            for &child in &children {
                let handle = map.get(child).expect("did not find child node");
                let guard = handle.read();

                match guard.kind {
                    IrNodeType::Vertices => bufs.positions.push(child),
                    IrNodeType::Normals => bufs.normals.push(child),
                    IrNodeType::Colors => bufs.colors.push(child),
                    IrNodeType::Uvs => bufs.uvs.push(child),
                    IrNodeType::Shape => bufs.shapes.push(child),
                    _ => {}
                }
            }
        }

        tracing::trace!("Constructed model buffer block successfully");
        Ok(bufs)
    }

    pub fn get_vertices(&self, index: usize) -> SlipstreamResult<InspectableReadGuard<VertexBuffer>> {
        let node_id = *self.positions.get(index).ok_or_else(|| {
            SlipstreamError::from(InvalidInputError {
                reason: format!("vertex buffer {index} does not exist"),
                ..Default::default()
            })
        })?;

        self.map
            .get_inspectable::<VertexBuffer>(node_id)
            .ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: format!("vertex buffer {index} was not found"),
                    ..Default::default()
                })
            })
    }

    pub fn get_normals(&self, index: usize) -> SlipstreamResult<InspectableReadGuard<NormalBuffer>> {
        let node_id = *self.normals.get(index).ok_or_else(|| {
            SlipstreamError::from(InvalidInputError {
                reason: format!("normal buffer {index} does not exist"),
                ..Default::default()
            })
        })?;

        self.map
            .get_inspectable::<NormalBuffer>(node_id)
            .ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: format!("normal buffer {index} was not found"),
                    ..Default::default()
                })
            })
    }
}
