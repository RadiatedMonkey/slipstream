use std::fmt::Debug;
use std::sync::OnceLock;

use slipstream_shared::cursor::RefCursor;

use crate::node::arena::{IrArena, IrNodeKey};
use crate::visitor::Visitable;

pub struct DeferredPayload {
    reader: RefCursor<[u8]>,
    ty: IrNodeType,
}

pub enum ContentSlot {
    /// The content has been evaluated eagerly, i.e. immediately.
    ///
    /// This should also be used when the node has no content.
    Eager(Option<Box<dyn Visitable + Send + Sync>>),
    Lazy(OnceLock<Box<dyn Visitable + Send + Sync>>, DeferredPayload),
}

impl ContentSlot {
    pub fn lazy(reader: RefCursor<[u8]>, ty: IrNodeType) -> Self {
        Self::Lazy(OnceLock::new(), DeferredPayload { reader, ty })
    }

    pub fn eager(content: Box<dyn Visitable + Send + Sync>) -> Self {
        Self::Eager(Some(content))
    }

    pub const fn none() -> Self {
        Self::Eager(None)
    }
}

/// A node in the filesystem. The editor's file system consists of just a tree with IDs (+ node types). The file contents
/// are stored in a central cache instead of in the tree.
///
/// Every (real and virtual) file and directory is stored as a node with unique ID.
/// These contents are stored in a central map that can be queried for any other node via its ID.
pub struct IrNode {
    /// The label that is displayed in the outliner. This is pretty much only for visuals as the nodes mostly
    /// refer to each other with IDs instead of names.
    pub label: String,
    /// The ID of this node. This is what other nodes use to refer to this one.
    ///
    /// This key should never be changed for a node and is therefore read-only.
    pub(super) key: IrNodeKey,
    /// Determines what type this node is. This affects how the node is displayed in the outliner and how
    /// other parts of the editor will treat this node. Setting the incorrect type for a node will likely cause
    /// a panic.
    pub ty: IrNodeType,
    /// The parent of this node.
    ///
    /// This will be `null` if the parent is unknown or this node does not have a parent.
    pub parent: Option<IrNodeKey>,
    /// A list of keys of children of this node.
    ///
    /// This value may be lazily evaluated, i.e. it may not be known yet.
    /// By accessing this value, it will be evaluated.
    pub children: Vec<IrNodeKey>,
    pub contents: ContentSlot,
}

impl IrNode {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn key(&self) -> IrNodeKey {
        self.key
    }

    pub fn ty(&self) -> IrNodeType {
        self.ty
    }

    pub fn set_ty(&mut self, ty: IrNodeType) {
        self.ty = ty;
    }

    pub fn children_keys(&self) -> &[IrNodeKey] {
        &self.children
    }

    pub fn content(&self) -> Option<&(dyn Visitable + Send + Sync)> {
        match &self.contents {
            ContentSlot::Eager(content) => content.as_deref(),
            ContentSlot::Lazy(lock, parser) => {
                todo!()
            }
        }
    }
}

/// The category this node belongs to. This affects visuals such as the icon but also how the editor treats this node.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IrNodeType {
    /// This virtual node can contain other nodes.
    ///
    /// This is used for both directories and files that contain multiple subfiles/sections.
    ArcDirectory {
        /// Whether the directory is empty. If it is, it will be inactive and have a special icon.
        empty: bool,
    },
    BrresFile,
    /// A directory in a BRRES file.
    BrresDirectory,
    /// The root of an MDL0 model. This should contain the section directories `Vertices`, `Normals`.
    Mdl0Root,
    /// The bytecode section of an MDL0 file.
    Definitions,
    /// The bone section of an MDL0 file.
    Bone {
        /// Whether this is the end bone of a limb. This makes sure it is not displayed as a folder.
        end: bool,
    },
    /// A vertex buffer in an MDL0 file.
    VertexBuffer,
    /// A normal buffer in an MDL0 file.
    Normals,
    /// A color buffer in an MDL0 file.
    Colors,
    Uvs,
    Material,
    Tevs,
    Polygon,
    TextureLinks,
    PaletteLinks,
    Unknown,
}
