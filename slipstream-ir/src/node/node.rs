use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, mpsc};

use parking_lot::{ArcRwLockReadGuard, Mutex, RawRwLock, RwLock};

use crate::node::defer::Deferred;
use crate::node::refs::VirtualNodeId;

/// An object that can be opened in the inspector window, with its own customizable contents.
pub trait Inspectable: Send + Sync + Debug + 'static {
    /// Draws the contents of the inspector window.
    ///
    /// The inspector creates the basic window and title bar, but the rest of the pane
    /// is controlled by this function.
    fn draw_properties(&mut self, ui: &mut egui::Ui);
    /// Converts this object to an immutable any trait object for downcasting.
    fn as_any(&self) -> &dyn Any;
    /// Converts this object to a mutable any trait object for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Custom guard that improves ergonomics of accessing inspectables in nodes.
///
/// This guard automatically dereferences and downcasts into the inspectable type
/// given at creation time.
pub struct InspectableReadGuard<T> {
    guard: ArcRwLockReadGuard<RawRwLock, VirtualNode>,
    _marker: PhantomData<T>,
}

impl<T> InspectableReadGuard<T> {
    /// The label of this node.
    pub fn label(&self) -> &str {
        &self.guard.label
    }

    /// The node's ID.
    pub fn id(&self) -> VirtualNodeId {
        self.guard.id
    }

    /// The type of content that this node contains.
    pub fn kind(&self) -> VirtualNodeKind {
        self.guard.kind
    }

    /// Returns the inner rwlock guard, consuming this guard.
    pub fn into_inner(self) -> ArcRwLockReadGuard<RawRwLock, VirtualNode> {
        self.guard
    }
}

impl<T> From<Arc<RwLock<VirtualNode>>> for InspectableReadGuard<T> {
    fn from(value: Arc<RwLock<VirtualNode>>) -> Self {
        let guard = value.read_arc();
        Self {
            guard,
            _marker: PhantomData,
        }
    }
}

impl<T: Inspectable> Deref for InspectableReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard
            .body
            .get()
            .and_then(|body| body.inspectable.as_ref())
            .and_then(|obj| obj.as_any().downcast_ref::<T>())
            .expect("node was deferred or had incorrect inspectable content")
    }
}

impl<T: Inspectable> AsRef<T> for InspectableReadGuard<T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

/// The contents of a virtual node.
///
/// This can either be a further file tree or file contents.
#[derive(Debug)]
pub struct VirtualNodeBody {
    /// The nodes that are contained in this one.
    ///
    /// These will also be loaded by the outliner to show a proper file tree.
    pub children: Vec<VirtualNodeId>,
    /// Optional contents of this node. This is mostly used for files.
    pub inspectable: Option<Box<dyn Inspectable>>,
}

/// A node in the filesystem. The editor's file system consists of just a tree with IDs (+ node types). The file contents
/// are stored in a central cache instead of in the tree.
///
/// Every (real and virtual) file and directory is stored as a node with unique ID.
/// These contents are stored in a central map that can be queried for any other node via its ID.
///
/// Initially I had the contents embedded directly into the tree, but that made jumping to arbitrary other files
/// quite difficult.
#[derive(Debug)]
pub struct VirtualNode {
    /// The label that is displayed in the outliner. This is pretty much only for visuals as the nodes mostly
    /// refer to each other with IDs instead of names.
    pub label: String,
    /// The ID of this node. This is what other nodes use to refer to this one.
    pub id: VirtualNodeId,
    /// Determines what type this node is. This affects how the node is displayed in the outliner and how
    /// other parts of the editor will treat this node. Setting the incorrect type for a node will likely cause
    /// a panic.
    pub kind: VirtualNodeKind,
    /// The parent of this node (if known).
    pub parent: Option<VirtualNodeId>,
    /// The contents of this node. They can be deferred, meaning they will be parsed lazily.
    ///
    /// This is pretty much only used for the contents of the subfiles (like BRRES) files. This makes a slight difference
    /// in editor opening time by only parsing contents when they are needed.
    pub body: Deferred<VirtualNodeBody>,
}

impl VirtualNode {
    /// Attempts to load this node's inspectable properties.
    ///
    /// If the node has not been evaluated yet or has no contents, this will return `None`.
    /// The contents are downcasted from a general trait object, so if the type `T` is incorrect for these
    /// contents, `None` will also be returned.
    pub fn inspectable<T: Inspectable>(&self) -> Option<&T> {
        self.body
            .get()?
            .inspectable
            .as_ref()?
            .as_any()
            .downcast_ref::<T>()
    }

    /// Attempts to load this node's inspectable properties.
    ///
    /// If the node has not been evaluated yet or has no contents, this will return `None`.
    /// The contents are downcasted from a general trait object, so if the type `T` is incorrect for these
    /// contents, `None` will also be returned.
    pub fn inspectable_mut<T: Inspectable>(&mut self) -> Option<&mut T> {
        self.body
            .get_mut()?
            .inspectable
            .as_mut()?
            .as_any_mut()
            .downcast_mut::<T>()
    }

    /// Renders the context menu of this node.
    pub fn draw_context_menu(&self, cmd: &mut mpsc::Sender<PaneAction>, ui: &mut egui::Ui) {
        if ui.button("Export").clicked() {
            todo!("export");
        }

        if ui.button("Rename").clicked() {
            todo!("rename");
        }

        if ui.button("Inspect").clicked() {
            cmd.send(PaneAction::RequestNewPane(RequestNewPane::Inspector {
                inspected: self.id,
            }));
        }

        if self.kind == VirtualNodeKind::Mdl0Root {
            if ui.button("Open in 3D viewer").clicked() {
                cmd.send(PaneAction::RequestNewPane(RequestNewPane::Viewer {
                    viewed: Some(self.id),
                }));
            }
        }

        if ui.button("Open in new Outliner").clicked() {
            cmd.send(PaneAction::RequestNewPane(RequestNewPane::Outliner {
                root: self.id,
            }));
        }
    }

    pub fn evaluate(&mut self) -> SlipstreamResult<()> {
        self.body.evaluate()?;
        Ok(())
    }

    pub fn is_deferred(&self) -> bool {
        self.body.is_deferred()
    }
}

/// The category this node belongs to. This affects visuals such as the icon but also how the editor treats this node.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VirtualNodeKind {
    /// This virtual node can contain other nodes.
    ///
    /// This is used for both directories and files that contain multiple subfiles/sections.
    ArcDirectory {
        /// Whether the directory is empty. If it is, it will be inactive and have a special icon.
        empty: bool,
    },
    /// A directory in a BRRES file.
    BrresDirectory,
    /// The root of an MDL0 model. This should contain the section directories `Vertices`, `Normals`.
    Mdl0Root,
    /// The bytecode section of an MDL0 file.
    Bytecode,
    /// The bone section of an MDL0 file.
    Bone {
        /// Whether this is the end bone of a limb. This makes sure it is not displayed as a folder.
        end: bool,
    },
    /// A vertex buffer in an MDL0 file.
    Vertices,
    /// A normal buffer in an MDL0 file.
    Normals,
    /// A color buffer in an MDL0 file.
    Colors,
    Uvs,
    Materials,
    Tevs,
    Shape,
    TextureLinks,
    PaletteLinks,
    Unknown,
}

impl VirtualNodeKind {
    /// Whether this node is expandable.
    ///
    /// This determines whether this node will have a collapsible header.
    pub fn is_expandable(&self) -> bool {
        match self {
            Self::ArcDirectory { empty: true }
            | Self::Bytecode
            | Self::Bone { end: true }
            | Self::Vertices
            | Self::Normals
            | Self::Colors
            | Self::Uvs
            | Self::Materials
            | Self::Tevs
            | Self::Shape
            | Self::TextureLinks
            | Self::PaletteLinks
            | Self::Unknown => false,
            _ => true,
        }
    }

    /// The icon to use when the folder/file is open.
    pub fn icon_open(&self) -> egui::RichText {
        match self {
            Self::ArcDirectory { empty: false } | Self::BrresDirectory => reg_icon!(FOLDER_OPEN),
            Self::ArcDirectory { empty: true } => reg_icon!(FOLDER_DASHED),
            Self::Mdl0Root => reg_icon!(PERSON),
            Self::Bytecode => reg_icon!(FILE_CODE),
            Self::Bone { end: false } => reg_icon!(BONE),
            Self::Bone { end: true } => fill_icon!(BONE),
            Self::Vertices => reg_icon!(POLYGON),
            Self::Normals => reg_icon!(ARROW_ELBOW_RIGHT),
            Self::Colors => reg_icon!(PAINT_BRUSH_HOUSEHOLD),
            Self::Uvs => reg_icon!(BOUNDING_BOX),
            Self::Materials => reg_icon!(PALETTE),
            Self::Tevs => reg_icon!(GRAPHICS_CARD),
            Self::Shape => reg_icon!(CUBE),
            Self::TextureLinks => reg_icon!(LINK),
            Self::PaletteLinks => reg_icon!(LINK),
            Self::Unknown => reg_icon!(FILE),
        }
    }

    /// The icon to use when the folder/file is closed.
    pub fn icon_closed(&self) -> egui::RichText {
        match self {
            Self::ArcDirectory { empty: false } | Self::BrresDirectory => reg_icon!(FOLDER),
            _ => self.icon_open(),
        }
    }
}
