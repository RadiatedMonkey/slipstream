use crate::node::arena::IrNodeKey;
use crate::node::node::{ContentSlot, IrNode, IrNodeType};
use crate::tex0::Texture;
use crate::{
    arc::{ArcDirectory, UnknownFile},
    mdl0::{
        self, bones::Bone, colors::ColorBuffer, definitions::Definitions,
        materials::MaterialBuffer, normals::NormalBuffer, pal_links::PaletteLinks,
        polygon::Polygon, tevs::Tev, tex_links::TextureLinks, uvs::UvBuffer,
        vertices::VertexBuffer,
    },
};
use downcast_rs::{Downcast, impl_downcast};
use std::any::Any;
use std::marker::PhantomData;
use std::ops::{ControlFlow, Deref, DerefMut};

pub struct VisitorContextNode<'a> {
    pub label: &'a str,
    pub ty: IrNodeType,
    pub key: IrNodeKey,
}

impl<'a> From<&'a IrNode> for VisitorContextNode<'a> {
    #[inline]
    fn from(node: &'a IrNode) -> Self {
        Self {
            label: &node.label,
            ty: node.ty,
            key: node.key(),
        }
    }
}

pub struct VisitorContext<'a, T> {
    pub meta: VisitorContextNode<'a>,
    pub content: &'a T,
}

impl<'a, T> VisitorContext<'a, T> {
    #[inline]
    pub(crate) fn new(node: VisitorContextNode<'a>, content: &'a T) -> Self {
        Self {
            meta: node,
            content,
        }
    }
}

impl<'a, T> Deref for VisitorContext<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.content
    }
}

pub struct VisitorContextNodeMut<'a> {
    pub label: &'a mut String,
    pub ty: IrNodeType,
    pub key: IrNodeKey,
}

impl<'a> From<&'a mut IrNode> for VisitorContextNodeMut<'a> {
    fn from(node: &'a mut IrNode) -> Self {
        Self {
            key: node.key(),
            label: &mut node.label,
            ty: node.ty,
        }
    }
}

/// The contents can be accessed via the deref implementations.
pub struct VisitorContextMut<'a, T> {
    pub meta: VisitorContextNodeMut<'a>,
    pub content: &'a mut T,
}

impl<'a, T> VisitorContextMut<'a, T> {
    #[inline]
    pub(crate) fn new(node: VisitorContextNodeMut<'a>, content: &'a mut T) -> Self {
        Self {
            meta: node,
            content,
        }
    }
}

impl<'a, T: 'static> Deref for VisitorContextMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.content
    }
}

impl<'a, T: 'static> DerefMut for VisitorContextMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.content
    }
}

/// Returning `ControlFlow::Break` from the visitor will end the walk in the current branch.
/// This means that further children of the node will not be visited.
#[allow(unused_variables)]
pub trait Visitor {
    fn visit_arc(&mut self, arc: VisitorContext<'_, ArcDirectory>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn visit_arc_mut(&mut self, arc: VisitorContextMut<'_, ArcDirectory>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    // MDL0 visitor methods
    // ==================================================================================================

    fn visit_mdl0(&mut self, model: VisitorContext<'_, mdl0::Model>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_definitions(
        &mut self,
        definitions: VisitorContext<'_, Definitions>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_bone(&mut self, bone: VisitorContext<'_, Bone>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_vertices(&mut self, vertex_buf: VisitorContext<'_, VertexBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_normals(&mut self, normal_buf: VisitorContext<'_, NormalBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_colors(&mut self, color_buf: VisitorContext<'_, ColorBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_uvs(&mut self, uv_buf: VisitorContext<'_, UvBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_polygon(&mut self, polygon: VisitorContext<'_, Polygon>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_material(&mut self, material: VisitorContext<'_, MaterialBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_tev(&mut self, tev: VisitorContext<'_, Tev>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_palette_links(&mut self, links: VisitorContext<'_, PaletteLinks>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_texture_links(&mut self, links: VisitorContext<'_, TextureLinks>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_unknown(&mut self, unknown: VisitorContext<'_, UnknownFile>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    // Mutable MDL0 visitor methods
    // ==================================================================================================

    fn visit_mdl0_mut(&mut self, model: VisitorContextMut<'_, mdl0::Model>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_definitions_mut(
        &mut self,
        definitions: VisitorContextMut<'_, Definitions>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_bone_mut(&mut self, bone: VisitorContextMut<'_, Bone>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_vertices_mut(
        &mut self,
        vertex_buf: VisitorContextMut<'_, VertexBuffer>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_normals_mut(
        &mut self,
        normal_buf: VisitorContextMut<'_, NormalBuffer>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_colors_mut(
        &mut self,
        color_buf: VisitorContextMut<'_, ColorBuffer>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_uvs_mut(&mut self, uv_buf: VisitorContextMut<'_, UvBuffer>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_polygon_mut(&mut self, polygon: VisitorContextMut<'_, Polygon>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_material_mut(
        &mut self,
        material: VisitorContextMut<'_, MaterialBuffer>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_tev_mut(&mut self, tev: VisitorContextMut<'_, Tev>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_palette_links_mut(
        &mut self,
        links: VisitorContextMut<'_, PaletteLinks>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_texture_links_mut(
        &mut self,
        links: VisitorContextMut<'_, TextureLinks>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn visit_unknown_mut(
        &mut self,
        unknown: VisitorContextMut<'_, UnknownFile>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    // TEX0 methods
    // ==================================================================================================

    fn visit_tex0(&mut self, texture: VisitorContext<'_, Texture>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn visit_tex0_mut(&mut self, texture: VisitorContextMut<'_, Texture>) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

pub trait Visitable: Downcast {
    fn accept(&self, node: VisitorContextNode, visitor: &mut dyn Visitor) -> ControlFlow<()>;
    fn accept_mut(
        &mut self,
        node: VisitorContextNodeMut,
        visitor: &mut dyn Visitor,
    ) -> ControlFlow<()>;
}
impl_downcast!(Visitable);
