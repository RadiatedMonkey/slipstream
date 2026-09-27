use crate::{
    format::mdl0::{
        bytecode::{Bytecode, BytecodeCommand},
        shapes::Shape,
        vertices::VertexBuf,
    },
    node::arena::{IrArena, IrNodeKey},
};

// pub struct ModelDescriptor<'a> {
//     bytecode: &'a Bytecode,
//     vertices: &'a Vertices,
//     polygons: &'a Polygon,
// }

// pub fn build_model<'a>(
//     desc: ModelDescriptor<'a>,
//     encoder: &mut wgpu::CommandEncoder,
//     arena: &IrArena,
// ) {
//     for cmd in &desc.bytecode.commands {
//         match cmd {
//             DrawCommand::DrawPolygon(polygon) => {}
//         }
//     }
// }
