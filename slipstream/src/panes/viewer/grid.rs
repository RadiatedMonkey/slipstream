use std::num::NonZeroU64;

use wgpu::util::DeviceExt;

use crate::panes::viewer::pipeline::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT, TARGET_FORMAT};

// const GRID_SCREEN_RECT_VERTICES: &[[f32; 2]] = &[[-1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];
//
// #[derive(Debug, Copy, Clone, PartialEq, bytemuck::Zeroable, bytemuck::Pod)]
// #[repr(C)]
// struct GridUniform {
//     pub inverse_view_proj: glam::Mat4,
// }

pub struct GridPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl GridPipeline {
    pub fn new(device: &wgpu::Device, camera_bind_layout: &wgpu::BindGroupLayout) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid pipeline layout"),
            bind_group_layouts: &[Some(&camera_bind_layout)],
            immediate_size: 0,
        });

        let module = device.create_shader_module(wgpu::include_wgsl!("../../../shaders/grid.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid render pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: TARGET_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self { pipeline }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass, camera_bind_group: &wgpu::BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
