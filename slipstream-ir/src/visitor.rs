use std::ops::ControlFlow;

use crate::{
    arc::{ArcDirectory, UnknownFile},
    mdl0::{
        self, bones::Bone, colors::ColorBuffer, definitions::Definitions,
        materials::MaterialBuffer, normals::NormalBuffer, pal_links::PaletteLinks,
        polygon::Polygon, tevs::Tev, tex_links::TextureLinks, uvs::UvBuffer,
        vertices::VertexBuffer,
    },
};

#[allow(unused_variables)]
pub trait Visitor {
    fn visit_arc(&mut self, arc: &ArcDirectory) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_mdl0(&mut self, model: &mdl0::Model) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_definitions(&mut self, definitions: &Definitions) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_bone(&mut self, bone: &Bone) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_vertices(&mut self, vertex_buf: &VertexBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_normals(&mut self, normal_buf: &NormalBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_colors(&mut self, color_buf: &ColorBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_uvs(&mut self, uv_buf: &UvBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_polygon(&mut self, polygon: &Polygon) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_material(&mut self, material: &MaterialBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_tev(&mut self, tev: &Tev) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_palette_links(&mut self, links: &PaletteLinks) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_texture_links(&mut self, links: &TextureLinks) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_unknown(&mut self, unknown: &UnknownFile) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

pub trait Visitable {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()>;
}
