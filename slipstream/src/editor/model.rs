use crate::{
    format::mdl0::{
        bytecode::{Bytecode, BytecodeCommand},
        shapes::Shape,
        vertices::VertexBuf,
    },
    node::refs::{VirtualNodeId, VirtualNodeMap},
};

// pub struct ModelDescriptor<'a> {
//     bytecode: &'a Bytecode,
//     vertices: &'a Vertices,
//     polygons: &'a Polygon,
// }

// pub fn build_model<'a>(
//     desc: ModelDescriptor<'a>,
//     encoder: &mut wgpu::CommandEncoder,
//     node_map: &VirtualNodeMap,
// ) {
//     for cmd in &desc.bytecode.commands {
//         match cmd {
//             DrawCommand::DrawPolygon(polygon) => {}
//         }
//     }
// }
