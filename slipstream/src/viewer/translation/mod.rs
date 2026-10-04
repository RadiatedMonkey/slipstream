mod skeleton;
mod vertex;

pub use skeleton::*;
pub use vertex::*;

use slipstream_ir::gx::GxOpCode;
use slipstream_ir::gx::draw::{
    DrawOpCode, InlineNormal, InlinePosition, NormalData, NormalIndex, OpVertex, PositionData,
};
use slipstream_ir::mdl0::bones::Bone;
use slipstream_ir::mdl0::definitions::{DRAW_OPA_NAME, Definitions, NODE_MIX_NAME, NODE_TREE_NAME};
use slipstream_ir::mdl0::normals::NormalBuffer;
use slipstream_ir::mdl0::polygon::Polygon;
use slipstream_ir::mdl0::vertices::VertexBuffer;
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_ir::visitor::{Visitable, Visitor, VisitorContext};
use slipstream_shared::{SlipstreamResult, try_unwrap, verify};
use std::ops::ControlFlow;

#[derive(Default, Debug)]
pub struct IntermediateModel {
    pub bone_map: BoneMap,
    pub bone_weights: Option<BoneWeights>,

    /// The transformation matrices to obtain the bind pose for each bone.
    /// The bone ID is a matrix into this array.
    pub bind_poses: Vec<glam::Mat4>,

    pub polygons: Vec<IntermediatePolygon>,
}

#[derive(Clone)]
pub struct ModelContents<'a> {
    arena: &'a IrArena,

    node_tree: Option<IrNodeKey>,
    node_mix: Option<IrNodeKey>,
    draw_opaque: Option<IrNodeKey>,

    skeleton_root: Option<IrNodeKey>,

    vertices: Vec<IrNodeKey>,
    normals: Vec<IrNodeKey>,
    polygons: Vec<IrNodeKey>,
}

impl<'a> ModelContents<'a> {
    pub fn from_root(root: IrNodeKey, arena: &'a IrArena) -> SlipstreamResult<Self> {
        let mut visitor = Self {
            arena,

            node_tree: None,
            node_mix: None,
            draw_opaque: None,

            skeleton_root: None,

            vertices: Vec::new(),
            normals: Vec::new(),
            polygons: Vec::new(),
        };
        arena.walk(root, &mut visitor)?;
        Ok(visitor)
    }

    /// Retrieves the given key from the map, downcasts it to `U`
    /// and runs `inspect_fn` on it.
    pub(super) fn try_inspect_inner<F, T, U>(
        &self,
        key: IrNodeKey,
        inspect_fn: F,
    ) -> SlipstreamResult<T>
    where
        U: Visitable,
        F: FnOnce(&U) -> SlipstreamResult<T>,
    {
        let out = self
            .arena
            .inspect(key, |node| {
                let buf = try_unwrap!(
                    node.contents.get_or_try_init()?,
                    "vertex buffer had no content"
                )?;

                let buf = try_unwrap!(
                    buf.as_any().downcast_ref::<U>(),
                    "vertex buffer had an incorrect `Visitable` type"
                )?;

                inspect_fn(buf)
            })
            .transpose()?;

        try_unwrap!(out, "vertex buffer {key:?} did not exist")
    }

    fn translate_node_tree(
        &self,
        out: &mut IntermediateModel,
        arena: &IrArena,
    ) -> SlipstreamResult<()> {
        struct DefinitionVisitor<'a> {
            model: &'a ModelContents<'a>,
            result: SlipstreamResult<BoneMap>,
        }

        impl Visitor for DefinitionVisitor<'_> {
            fn visit_definitions(
                &mut self,
                context: VisitorContext<'_, Definitions>,
            ) -> ControlFlow<()> {
                tracing::trace!("Translating definition `{}`", context.meta.label);
                self.result = BoneMap::from_definitions(context.content);

                ControlFlow::Continue(())
            }
        }

        if let Some(node_tree) = self.node_tree {
            let mut visitor = DefinitionVisitor {
                model: self,
                result: Ok(BoneMap::default()),
            };
            arena.visit(node_tree, &mut visitor)?;
            out.bone_map = visitor.result?;
        }

        Ok(())
    }

    fn translate_node_mix(
        &self,
        out: &mut IntermediateModel,
        arena: &IrArena,
    ) -> SlipstreamResult<()> {
        struct DefinitionVisitor<'a> {
            model: &'a ModelContents<'a>,
            result: SlipstreamResult<BoneWeights>,
        }

        impl Visitor for DefinitionVisitor<'_> {
            fn visit_definitions(
                &mut self,
                context: VisitorContext<'_, Definitions>,
            ) -> ControlFlow<()> {
                tracing::trace!("Translating definition `{}`", context.meta.label);
                self.result = BoneWeights::from_definitions(context.content);

                ControlFlow::Continue(())
            }
        }

        if let Some(node_mix) = self.node_mix {
            let mut visitor = DefinitionVisitor {
                model: self,
                result: Ok(BoneWeights::default()),
            };
            arena.visit(node_mix, &mut visitor)?;
            out.bone_weights = Some(visitor.result?);
        }

        Ok(())
    }

    fn translate_polygons(
        &self,
        out: &mut IntermediateModel,
        arena: &IrArena,
    ) -> SlipstreamResult<()> {
        struct PolygonVisitor<'a> {
            out: &'a IntermediateModel,
            model: &'a ModelContents<'a>,
            scratch: &'a mut IntermediatePolygon,
            result: SlipstreamResult<()>,
        }

        impl Visitor for PolygonVisitor<'_> {
            fn visit_polygon(&mut self, context: VisitorContext<'_, Polygon>) -> ControlFlow<()> {
                tracing::trace!("Translating polygon `{}`", context.meta.label);
                self.result = self
                    .model
                    .translate_polygon(self.out, self.scratch, context.content);

                ControlFlow::Break(())
            }
        }

        out.polygons.reserve(self.polygons.len());
        for &polygon in &self.polygons {
            let mut intermediate = IntermediatePolygon::default();
            let mut visitor = PolygonVisitor {
                out,
                model: self,
                scratch: &mut intermediate,
                result: Ok(()),
            };
            let _ = arena.visit(polygon, &mut visitor)?;
            visitor.result?;

            out.polygons.push(intermediate);
        }

        Ok(())
    }

    fn traverse_skeleton(
        &self,
        out: &mut IntermediateModel,
        arena: &IrArena,
    ) -> SlipstreamResult<()> {
        struct BoneVisitor<'a> {
            out: &'a mut IntermediateModel,
            result: SlipstreamResult<()>,
        }

        impl Visitor for BoneVisitor<'_> {
            fn visit_bone(&mut self, bone: VisitorContext<'_, Bone>) -> ControlFlow<()> {
                let [a, b, c] = bone.rotation_vector;
                let mat = glam::Mat4::from_scale_rotation_translation(
                    glam::Vec3::from_array(bone.scaling_vector),
                    glam::Quat::from_euler(glam::EulerRot::XYZEx, a, b, c),
                    glam::Vec3::from_array(bone.translation_vector),
                );

                if bone.index as usize > self.out.bind_poses.len() {
                    self.out
                        .bind_poses
                        .resize(bone.index as usize + 1, glam::Mat4::ZERO);
                }

                self.out.bind_poses.insert(bone.index as usize, mat);

                tracing::debug!("{mat:#?}");
                tracing::error!("TOOD: Nested matrices should be multiplied");

                ControlFlow::Continue(())
            }
        }

        let skeleton_root = try_unwrap!(self.skeleton_root, "skeleton root was not found")?;

        let mut visitor = BoneVisitor {
            out,
            result: Ok(()),
        };
        arena.walk(skeleton_root, &mut visitor)?;
        visitor.result?;

        tracing::debug!("bind poses buffer: {:#?}", out.bind_poses);

        Ok(())
    }

    /// Converts the raw buffers to an [`IntermediateModel`].
    ///
    /// This intermediate model can then be converted into wgpu buffers and commands
    /// in the next step.
    pub fn to_intermediate(&self, arena: &IrArena) -> SlipstreamResult<IntermediateModel> {
        let mut model = IntermediateModel::default();

        self.translate_node_tree(&mut model, arena)?;
        self.translate_node_mix(&mut model, arena)?;
        self.traverse_skeleton(&mut model, arena)?;
        self.translate_polygons(&mut model, arena)?;

        Ok(model)
    }
}

impl Visitor for ModelContents<'_> {
    fn visit_definitions(
        &mut self,
        definitions: VisitorContext<'_, Definitions>,
    ) -> ControlFlow<()> {
        match definitions.meta.label {
            DRAW_OPA_NAME => self.draw_opaque = Some(definitions.meta.key),
            NODE_TREE_NAME => self.node_tree = Some(definitions.meta.key),
            NODE_MIX_NAME => self.node_mix = Some(definitions.meta.key),
            _ => tracing::warn!("Unknown definitions file: `{}`", definitions.meta.label),
        }

        ControlFlow::Break(())
    }

    fn visit_bone(&mut self, bone: VisitorContext<'_, Bone>) -> ControlFlow<()> {
        // First bone we find should be the root, but still verify
        if bone.parent.is_some() {
            tracing::error!(
                "First bone in the model has an assigned parent while it should be the root"
            );
        }

        self.skeleton_root = Some(bone.meta.key);
        ControlFlow::Break(())
    }

    fn visit_vertices(&mut self, vertices: VisitorContext<'_, VertexBuffer>) -> ControlFlow<()> {
        self.vertices.push(vertices.meta.key);
        ControlFlow::Break(())
    }

    fn visit_normals(&mut self, normals: VisitorContext<'_, NormalBuffer>) -> ControlFlow<()> {
        self.normals.push(normals.meta.key);
        ControlFlow::Break(())
    }

    fn visit_polygon(&mut self, polygon: VisitorContext<'_, Polygon>) -> ControlFlow<()> {
        self.polygons.push(polygon.meta.key);
        ControlFlow::Break(())
    }
}
