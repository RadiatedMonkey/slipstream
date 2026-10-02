use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::panes::viewer::{
    intermediate::IntermediatePolygon,
    pipeline::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT, TARGET_FORMAT},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineSignature(u64);

impl PipelineSignature {
    pub fn from_layouts(layouts: &[Option<wgpu::VertexBufferLayout<'_>>]) -> Self {
        let mut hasher = DefaultHasher::new();
        for layout in layouts {
            layout.hash(&mut hasher);
        }
        PipelineSignature(hasher.finish())
    }
}

pub struct PipelineDescriptor<'a> {
    pub vertex_layouts: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    pub bind_groups: &'a [Option<&'a wgpu::BindGroupLayout>],
}

pub struct PipelineEntry {
    layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
}

pub struct PipelineRegistry {
    device: wgpu::Device,
    pipelines: HashMap<PipelineSignature, PipelineEntry>,
}

impl PipelineRegistry {
    pub fn new(device: wgpu::Device) -> Self {
        Self {
            device,
            pipelines: HashMap::new(),
        }
    }

    pub fn get(&self, signature: PipelineSignature) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(&signature).map(|entry| &entry.pipeline)
    }

    pub fn register(&mut self, desc: PipelineDescriptor<'_>) -> PipelineSignature {
        let signature = PipelineSignature::from_layouts(&desc.vertex_layouts);
        self.pipelines.entry(signature).or_insert_with(|| {
            let module = self
                .device
                .create_shader_module(wgpu::include_wgsl!("../../../../shaders/viewer.wgsl"));

            let layout = self
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: desc.bind_groups,
                    immediate_size: 0,
                });

            let pipeline = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs_main"),
                        buffers: desc.vertex_layouts,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                        unclipped_depth: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: Some(true),
                        depth_compare: Some(wgpu::CompareFunction::Less),
                        bias: wgpu::DepthBiasState::default(),
                        stencil: wgpu::StencilState::default(),
                    }),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: TARGET_FORMAT,
                            blend: Some(wgpu::BlendState::REPLACE),
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

            PipelineEntry { layout, pipeline }
        });

        signature
    }
}
