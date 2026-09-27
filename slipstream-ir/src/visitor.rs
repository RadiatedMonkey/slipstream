use crate::mdl0::{
    bones::Bone, colors::ColorBuffer, definitions::Definitions, materials::MaterialBuffer,
    normals::NormalBuffer, pal_links::PaletteLinks, polygon::Polygon, tevs::Tev,
    tex_links::TextureLinks, uvs::UvBuffer, vertices::VertexBuffer,
};

#[allow(unused_variables)]
pub trait Visitor {
    fn visit_definitions(&mut self, definitions: &Definitions) {}
    fn visit_bone(&mut self, bone: &Bone) {}
    fn visit_vertices(&mut self, vertex_buf: &VertexBuffer) {}
    fn visit_normals(&mut self, normal_buf: &NormalBuffer) {}
    fn visit_colors(&mut self, color_buf: &ColorBuffer) {}
    fn visit_uvs(&mut self, uv_buf: &UvBuffer) {}
    fn visit_polygon(&mut self, polygon: &Polygon) {}
    fn visit_material(&mut self, material: &MaterialBuffer) {}
    fn visit_tev(&mut self, tev: &Tev) {}
    fn visit_palette_links(&mut self, links: &PaletteLinks) {}
    fn visit_texture_links(&mut self, links: &TextureLinks) {}
}

pub trait Visitable {
    fn accept(&self, visitor: &mut dyn Visitor);
}
