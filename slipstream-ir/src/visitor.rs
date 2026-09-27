use crate::mdl0::{
    bones::Bone, bytecode::Definitions, colors::ColorBuffer, materials::MaterialBuffer,
    normals::NormalBuffer, polygon::Polygon, uvs::UvBuf, vertices::VertexBuffer,
};

#[allow(unused_variables)]
pub trait Visitor {
    fn visit_definitions(&mut self, definitions: &Definitions) {}
    fn visit_bone(&mut self, bone: &Bone) {}
    fn visit_vertices(&mut self, vertex_buf: &VertexBuffer) {}
    fn visit_normals(&mut self, normal_buf: &NormalBuffer) {}
    fn visit_colors(&mut self, color_buf: &ColorBuffer) {}
    fn visit_uvs(&mut self, uv_buf: &UvBuf) {}
    fn visit_polygon(&mut self, polygon: &Polygon) {}
    fn visit_material(&mut self, material: &MaterialBuffer) {}
}

pub trait Visitable {
    fn accept(&self, visitor: &mut dyn Visitor);
}
