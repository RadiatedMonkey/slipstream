//! Translates an intermediate model to a wgpu-compatible one.

use slipstream_shared::SlipstreamResult;
use wgpu::util::DeviceExt;

use crate::panes::viewer::intermediate::{IntermediateModel, IntermediatePolygon};

pub struct WgpuModel {
    polygons: Vec<WgpuPolygon>,
}

impl WgpuModel {
    pub fn from_intermediate(
        device: &wgpu::Device,
        ir: IntermediateModel,
    ) -> SlipstreamResult<Self> {
        let mut polygons = Vec::with_capacity(ir.polygons.len());
        for polygon in ir.polygons {
            polygons.push(WgpuPolygon::from_intermediate(device, polygon)?);
        }

        todo!()
    }
}

pub struct WgpuPolygon {
    indices: u32,
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl WgpuPolygon {
    pub const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
    pub const VERTEX_FORMAT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: 3 * size_of::<f32>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        }],
    };

    pub fn from_intermediate(
        device: &wgpu::Device,
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

        Ok(Self {
            indices: ir.indices.len() as u32,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), Self::INDEX_FORMAT);
        render_pass.draw_indexed(0..self.indices, 0, 0..1);
    }
}
