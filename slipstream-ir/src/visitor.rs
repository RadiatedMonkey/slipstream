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

    // Mutable functions
    // =============================================================================================
    
    fn visit_arc_mut(&mut self, arc: &mut ArcDirectory) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_mdl0_mut(&mut self, model: &mut mdl0::Model) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_definitions_mut(&mut self, definitions: &mut Definitions) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_bone_mut(&mut self, bone: &mut Bone) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_vertices_mut(&mut self, vertex_buf: &mut VertexBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_normals_mut(&mut self, normal_buf: &mut NormalBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_colors_mut(&mut self, color_buf: &mut ColorBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_uvs_mut(&mut self, uv_buf: &mut UvBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_polygon_mut(&mut self, polygon: &mut Polygon) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_material_mut(&mut self, material: &mut MaterialBuffer) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_tev_mut(&mut self, tev: &mut Tev) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_palette_links_mut(&mut self, links: &mut PaletteLinks) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_texture_links_mut(&mut self, links: &mut TextureLinks) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_unknown_mut(&mut self, unknown: &mut UnknownFile) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

pub trait Visitable {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()>;
    fn accept_mut(&mut self, visitor: &mut dyn Visitor) -> ControlFlow<()>;
}
