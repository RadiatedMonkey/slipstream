//! Translates between Wii models and wgpu ones.

use slipstream_ir::gx::draw::{InlineNormal, InlinePosition};
use slipstream_ir::mdl0::polygon::Polygon;
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_ir::node::node::IrNodeType;
use slipstream_shared::error::{InvalidInputError, SlipstreamError, SlipstreamResult};
use slipstream_shared::verify;
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::Arc;
use slipstream_ir::mdl0::normals::NormalBuffer;
use slipstream_ir::mdl0::vertices::VertexBuffer;
use slipstream_ir::visitor::{VisitorContext, Visitor};
use crate::panes::viewer::pipeline::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT, TARGET_FORMAT};

#[derive(Debug, Default)]
pub struct ModelVisitor {
    vertices: Vec<IrNodeKey>,
    normals: Vec<IrNodeKey>,
    polygons: Vec<IrNodeKey>
}

impl ModelVisitor {
    pub fn from_root(root: IrNodeKey, arena: &IrArena) -> SlipstreamResult<Self> {
        let mut visitor = Self::default();
        arena.walk(root, &mut visitor)?;
        Ok(visitor)
    }
}

impl Visitor for ModelVisitor {
    fn visit_vertices(&mut self, vertices: VisitorContext<'_, VertexBuffer>) -> ControlFlow<()> {
        self.vertices.push(vertices.node.key);
        ControlFlow::Continue(())
    }

    fn visit_normals(&mut self, normals: VisitorContext<'_, NormalBuffer>) -> ControlFlow<()> {
        self.normals.push(normals.node.key);
        ControlFlow::Continue(())
    }

    fn visit_polygon(&mut self, polygon: VisitorContext<'_, Polygon>) -> ControlFlow<()> {
        self.polygons.push(polygon.node.key);
        ControlFlow::Continue(())
    }
}