use std::{
    collections::HashMap,
    num::NonZeroU64,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use std::ops::ControlFlow;
use parking_lot::RwLock;
use slipstream_shared::error::InvalidInputError;
use slipstream_shared::{SlipstreamError, SlipstreamResult};
use crate::node::{
    guard::ContentReadGuard,
    node::{ContentSlot, IrNode, IrNodeType},
};
use crate::visitor::{Visitor, VisitorContextNode};

/// A key that can be used to refer to a node.
///
/// This uses a nonzero u64 internally to enable niche optimizations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IrNodeKey(NonZeroU64);

impl From<IrNodeKey> for NonZeroU64 {
    fn from(value: IrNodeKey) -> Self {
        value.0
    }
}

impl From<IrNodeKey> for u64 {
    fn from(value: IrNodeKey) -> Self {
        value.0.get()
    }
}

pub type IrNodeRef = Arc<RwLock<IrNode>>;

pub struct IrNodeDescriptor {
    pub label: String,
    pub ty: IrNodeType,
    pub parent: Option<IrNodeKey>,
    pub children: Vec<IrNodeKey>,
    pub contents: ContentSlot,
}

impl Default for IrNodeDescriptor {
    fn default() -> Self {
        Self {
            label: String::from("<null>"),
            ty: IrNodeType::Unknown,
            parent: None,
            children: Vec::new(),
            contents: const { ContentSlot::none() },
        }
    }
}

pub struct IrArena {
    counter: AtomicU64,
    map: RwLock<HashMap<IrNodeKey, IrNodeRef>>,
}

impl IrArena {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(1),
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Reserves a key for future use.
    ///
    /// The keys returned by this function are guaranteed to be unique, but
    /// may not be monotonic.
    pub fn reserve_key(&self) -> IrNodeKey {
        IrNodeKey(
            NonZeroU64::new(self.counter.fetch_add(1, Ordering::Relaxed))
                .expect("arena counter was set to 0"),
        )
    }

    /// Inserts a node at a previously reserved location.
    pub fn insert_at(&self, key: IrNodeKey, desc: IrNodeDescriptor) {
        self.map.write().insert(
            key,
            Arc::new(RwLock::new(IrNode {
                label: desc.label,
                ty: desc.ty,
                parent: desc.parent,
                key,
                children: desc.children,
                contents: desc.contents,
            })),
        );
    }

    /// Inserts a node, returning its assigned key.
    pub fn insert(&self, desc: IrNodeDescriptor) -> IrNodeKey {
        let key = self.reserve_key();

        self.map.write().insert(
            key,
            Arc::new(RwLock::new(IrNode {
                label: desc.label,
                ty: desc.ty,
                parent: desc.parent,
                key,
                children: desc.children,
                contents: desc.contents,
            })),
        );

        key
    }

    /// Attempts to load the node's contents. Parsing it if it has not been evaluated yet.
    ///
    /// ## Note:
    /// This function is blocking, it should not be used on the UI thread.
    pub fn get_content(&self, key: IrNodeKey) -> Option<ContentReadGuard> {
        self.map.read().get(&key).map(ContentReadGuard::new)
    }

    /// Loads the given node and runs `inspect_fn` with a shared reference to it.
    pub fn inspect<T, F>(&self, key: IrNodeKey, inspect_fn: F) -> Option<T>
    where
        F: FnOnce(&IrNode) -> T,
    {
        let guard = self.map.read();
        guard.get(&key).map(|lock| {
            let guard = lock.write();
            inspect_fn(&guard)
        })
    }

    /// Loads the given node and runs `update_fn` with a mutable reference to it.
    pub fn update<T, F>(&self, key: IrNodeKey, update_fn: F) -> Option<T>
    where
        F: FnOnce(&mut IrNode) -> T,
    {
        let mut guard = self.map.write();
        guard.get_mut(&key).map(|lock| {
            let mut guard = lock.write();
            update_fn(&mut guard)
        })
    }

    /// Walks the entire tree from a root node. This function forces any lazy nodes it encounters
    /// to be evaluated.
    ///
    /// # Errors
    /// This function returns an error if the given root node does not exist.
    pub fn walk(&self, root: IrNodeKey, visitor: &mut dyn Visitor) -> SlipstreamResult<()> {
        let root = self.map.read().get(&root).ok_or_else(|| SlipstreamError::from(InvalidInputError {
            reason: format!("root node {root:?} does not exist"),
            ..Default::default()
        }))?.clone();

        let guard = root.read();
        for &child in &guard.children {
            tracing::trace!("{:?}", child);

            // Visits the child's contents and returns a control flow.
            let flow = self.inspect(child, |child| {
                if let Some(contents) = child.contents.get_or_try_init()? {
                    return Ok::<_, SlipstreamError>(contents.accept(VisitorContextNode::from(child), visitor))
                }

                Ok(ControlFlow::Continue(()))
            }).transpose()?.unwrap_or(ControlFlow::Continue(()));

            if flow.is_continue() {
                // Walk this node's children only if the visitor wants to continue.
                self.walk(child, visitor)?;
            }
        }

        Ok(())
    }
}

impl Default for IrArena {
    fn default() -> Self {
        Self::new()
    }
}
