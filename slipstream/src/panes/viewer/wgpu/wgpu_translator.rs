//! Translates an intermediate model to a wgpu-compatible one.

use slipstream_shared::SlipstreamResult;
use wgpu::util::DeviceExt;

use crate::panes::viewer::{
    intermediate::{IntermediateModel, IntermediatePolygon},
    pipeline::CameraState,
    wgpu::{PipelineDescriptor, PipelineRegistry, PipelineSignature},
};

pub struct WgpuModel {
    camera_bind_group: wgpu::BindGroup,
    pipelines: PipelineRegistry,
    polygons: Vec<WgpuPolygon>,
}

impl WgpuModel {
    pub fn from_intermediate(
        device: &wgpu::Device,
        camera_state: &CameraState,
        ir: IntermediateModel,
    ) -> SlipstreamResult<Self> {
        let mut pipelines = PipelineRegistry::new(device.clone());

        let mut polygons = Vec::with_capacity(ir.polygons.len());
        for polygon in ir.polygons {
            polygons.push(WgpuPolygon::from_intermediate(
                device,
                &camera_state.bind_group_layout,
                &mut pipelines,
                polygon,
            )?);
        }

        Ok(Self {
            camera_bind_group: camera_state.bind_group.clone(),
            pipelines,
            polygons,
        })
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass) {
        for polygon in &self.polygons {
            polygon.draw(&self.pipelines, &self.camera_bind_group, render_pass);
        }
    }
}

pub struct WgpuPolygon {
    pipeline: PipelineSignature,

    indices: u32,
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl WgpuPolygon {
    pub const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
    pub const VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: 6 * size_of::<f32>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 3 * size_of::<f32>() as u64,
                shader_location: 1,
            },
        ],
    };

    pub fn from_intermediate(
        device: &wgpu::Device,
        camera_bg: &wgpu::BindGroupLayout,
        pipelines: &mut PipelineRegistry,
        ir: IntermediatePolygon,
    ) -> SlipstreamResult<Self> {
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("polygon index buffer"),
            contents: bytemuck::cast_slice(&ir.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("polygon vertex buffer"),
            contents: bytemuck::cast_slice(&ir.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let pipeline_signature = pipelines.register(PipelineDescriptor {
            bind_groups: &[Some(camera_bg)],
            vertex_layouts: &[Some(Self::VERTEX_LAYOUT)],
        });

        tracing::trace!("Generated wgpu model with {} indices", ir.indices.len());

        Ok(Self {
            pipeline: pipeline_signature,
            indices: ir.indices.len() as u32,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn draw(
        &self,
        pipelines: &PipelineRegistry,
        camera_bg: &wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass,
    ) {
        let pipeline = pipelines.get(self.pipeline).expect("pipeline not found");

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, camera_bg, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), Self::INDEX_FORMAT);
        render_pass.draw_indexed(0..self.indices, 0, 0..1);
    }
}
