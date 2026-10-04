//! Translates an intermediate model to a wgpu-compatible one.

use slipstream_shared::SlipstreamResult;
use wgpu::util::DeviceExt;

use crate::viewer::translation::{IntermediateModel, IntermediatePolygon};
use crate::viewer::{
    pipeline::CameraState,
    wgpu::{PipelineDescriptor, PipelineRegistry, PipelineSignature},
};

pub struct WgpuModel {
    camera_bind_group: wgpu::BindGroup,
    bind_pose_group: wgpu::BindGroup,

    pipelines: PipelineRegistry,
    polygons: Vec<WgpuPolygon>,
}

impl WgpuModel {
    pub fn from_intermediate(
        device: &wgpu::Device,
        camera_state: &CameraState,
        ir: IntermediateModel,
    ) -> SlipstreamResult<Self> {
        let bind_pose_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bind pose storage buffer"),
            contents: bytemuck::cast_slice(&ir.bind_poses),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bind_pose_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind pose bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    min_binding_size: None,
                    has_dynamic_offset: false,
                },
                count: None,
            }],
        });

        let bind_pose_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind pose bind group"),
            layout: &bind_pose_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &bind_pose_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let mut pipelines = PipelineRegistry::new(device.clone());

        let mut polygons = Vec::with_capacity(ir.polygons.len());
        for polygon in ir.polygons {
            polygons.push(WgpuPolygon::from_intermediate(
                device,
                &bind_pose_layout,
                &camera_state.bind_group_layout,
                &mut pipelines,
                polygon,
            )?);
        }

        Ok(Self {
            camera_bind_group: camera_state.bind_group.clone(),
            bind_pose_group,
            pipelines,
            polygons,
        })
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass) {
        for polygon in &self.polygons {
            polygon.draw(
                &self.pipelines,
                &self.camera_bind_group,
                &self.bind_pose_group,
                render_pass,
            );
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
        array_stride: 56,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Uint32x4,
            3 => Float32x4,
        ],
        step_mode: wgpu::VertexStepMode::Vertex,
    };

    pub fn from_intermediate(
        device: &wgpu::Device,
        bind_pose_layout: &wgpu::BindGroupLayout,
        camera_layout: &wgpu::BindGroupLayout,
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
            name: "polygon",
            bind_group_layouts: &[Some(camera_layout), Some(bind_pose_layout)],
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
        camera_group: &wgpu::BindGroup,
        bind_pose_group: &wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass,
    ) {
        let pipeline = pipelines.get(self.pipeline).expect("pipeline not found");

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, camera_group, &[]);
        render_pass.set_bind_group(1, bind_pose_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), Self::INDEX_FORMAT);
        render_pass.draw_indexed(0..self.indices, 0, 0..1);
    }
}
