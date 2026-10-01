use std::ops::{ControlFlow, Deref, DerefMut};

use crate::{
    arc::{ArcDirectory, UnknownFile},
    mdl0::{
        self, bones::Bone, colors::ColorBuffer, definitions::Definitions,
        materials::MaterialBuffer, normals::NormalBuffer, pal_links::PaletteLinks,
        polygon::Polygon, tevs::Tev, tex_links::TextureLinks, uvs::UvBuffer,
        vertices::VertexBuffer,
    },
};
use crate::node::arena::IrNodeKey;

pub struct IrNodeContext<'a, T> {
    pub label: &'a str,
    pub key: IrNodeKey,
    pub content: &'a T
}

impl<'a, T> Deref for IrNodeContext<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.content
    }
}

pub struct IrNodeContextMut<'a, T> {
    pub label: &'a str,
    pub key: IrNodeKey,
    pub content: &'a mut T
}

impl<'a, T> Deref for IrNodeContextMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.content
    }
}

impl<'a, T> DerefMut for IrNodeContextMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.content
    }
}

#[allow(unused_variables)]
pub trait Visitor {
    fn visit_arc(&mut self, arc: IrNodeContext<'_, ArcDirectory>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_mdl0(&mut self, model: IrNodeContext<'_, mdl0::Model>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_definitions(&mut self, definitions: IrNodeContext<'_, Definitions>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_bone(&mut self, bone: IrNodeContext<'_, Bone>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_vertices(&mut self, vertex_buf: IrNodeContext<'_, VertexBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_normals(&mut self, normal_buf: IrNodeContext<'_, NormalBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_colors(&mut self, color_buf: IrNodeContext<'_, ColorBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_uvs(&mut self, uv_buf: IrNodeContext<'_, UvBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_polygon(&mut self, polygon: IrNodeContext<'_, Polygon>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_material(&mut self, material: IrNodeContext<'_, MaterialBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_tev(&mut self, tev: IrNodeContext<'_, Tev>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_palette_links(&mut self, links: IrNodeContext<'_, PaletteLinks>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_texture_links(&mut self, links: IrNodeContext<'_, TextureLinks>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_unknown(&mut self, unknown: IrNodeContext<'_, UnknownFile>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    // Mutable functions
    // =============================================================================================
    
    fn visit_arc_mut(&mut self, arc: IrNodeContextMut<'_, ArcDirectory>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_mdl0_mut(&mut self, model: IrNodeContextMut<'_, mdl0::Model>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_definitions_mut(&mut self, definitions: IrNodeContextMut<'_, Definitions>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_bone_mut(&mut self, bone: IrNodeContextMut<'_, Bone>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_vertices_mut(&mut self, vertex_buf: IrNodeContextMut<'_, VertexBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_normals_mut(&mut self, normal_buf: IrNodeContextMut<'_, NormalBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_colors_mut(&mut self, color_buf: IrNodeContextMut<'_, ColorBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_uvs_mut(&mut self, uv_buf: IrNodeContextMut<'_, UvBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_polygon_mut(&mut self, polygon: IrNodeContextMut<'_, Polygon>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_material_mut(&mut self, material: IrNodeContextMut<'_, MaterialBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_tev_mut(&mut self, tev: IrNodeContextMut<'_, Tev>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_palette_links_mut(&mut self, links: IrNodeContextMut<'_, PaletteLinks>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_texture_links_mut(&mut self, links: IrNodeContextMut<'_, TextureLinks>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_unknown_mut(&mut self, unknown: IrNodeContextMut<'_, UnknownFile>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

pub trait Visitable {
    fn accept(&self, visitor: &mut dyn Visitor) -> ControlFlow<()>;
    fn accept_mut(&mut self, visitor: &mut dyn Visitor) -> ControlFlow<()>;
}