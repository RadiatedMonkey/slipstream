use crate::format::mdl0::vertices::VertexBufData;

impl VertexBufData {
    pub const fn format(&self) -> wgpu::VertexFormat {
        match self {
            Self::Xy(_) => wgpu::VertexFormat::Float32x2,
            Self::Xyz(_) => wgpu::VertexFormat::Float32x3,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex3 {
    coordinates: [f32; 3],
}

impl Vertex3 {
    pub const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        }
    }
}

pub const CUBE_VERTICES: [Vertex3; 8] = [
    Vertex3 {
        coordinates: [-0.5, -0.5, 0.5],
    }, // 0: Bottom-left-front
    Vertex3 {
        coordinates: [0.5, -0.5, 0.5],
    }, // 1: Bottom-right-front
    Vertex3 {
        coordinates: [0.5, 0.5, 0.5],
    }, // 2: Top-right-front
    Vertex3 {
        coordinates: [-0.5, 0.5, 0.5],
    }, // 3: Top-left-front
    Vertex3 {
        coordinates: [-0.5, -0.5, -0.5],
    }, // 4: Bottom-left-back
    Vertex3 {
        coordinates: [0.5, -0.5, -0.5],
    }, // 5: Bottom-right-back
    Vertex3 {
        coordinates: [0.5, 0.5, -0.5],
    }, // 6: Top-right-back
    Vertex3 {
        coordinates: [-0.5, 0.5, -0.5],
    }, // 7: Top-left-back
];

pub const CUBE_INDICES: [u16; 36] = [
    0, 1, 2, 2, 3, 0, // front
    1, 5, 6, 6, 2, 1, // right
    5, 4, 7, 7, 6, 5, // back
    4, 0, 3, 3, 7, 4, // left
    3, 2, 6, 6, 7, 3, // top
    4, 5, 1, 1, 0, 4, // bottom
];
