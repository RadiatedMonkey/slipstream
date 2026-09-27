use std::{ops::Deref, sync::Arc};

use parking_lot::{ArcRwLockReadGuard, RawRwLock, RwLock};

use crate::node::node::{ContentSlot, IrNode};

type LockReadGuard = ArcRwLockReadGuard<RawRwLock, IrNode>;

pub struct ContentReadGuard {
    inner: LockReadGuard,
}

impl ContentReadGuard {
    pub fn new(lock: &Arc<RwLock<IrNode>>) -> Self {
        let guard = lock.read_arc();

        Self { inner: guard }
    }

    pub fn into_inner(self) -> LockReadGuard {
        self.inner
    }
}

impl Deref for ContentReadGuard {
    type Target = ContentSlot;

    fn deref(&self) -> &Self::Target {
        &self.inner.contents
    }
}

impl AsRef<ContentSlot> for ContentReadGuard {
    fn as_ref(&self) -> &ContentSlot {
        self.deref()
    }
}
