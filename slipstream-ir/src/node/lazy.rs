use std::{cell::UnsafeCell, fmt, mem::ManuallyDrop, ptr};

use slipstream_shared::{assert::AssertSendSync, cursor::RefCursor};

use crate::{
    node::{
        node::IrNodeType,
        once::{Once, OnceState},
    },
    visitor::Visitable,
};

type DynContent = dyn Visitable + Send + Sync;

#[derive(Clone)]
pub struct DeferPayload {
    pub reader: RefCursor<[u8]>,
    pub ty: IrNodeType,
}

union LazyInner {
    payload: ManuallyDrop<DeferPayload>,
    content: ManuallyDrop<Box<DynContent>>,
}

pub struct LazyContent {
    once: Once,
    data: UnsafeCell<LazyInner>,
}

impl LazyContent {
    pub const fn new(reader: RefCursor<[u8]>, ty: IrNodeType) -> Self {
        Self {
            once: Once::new(),
            data: UnsafeCell::new(LazyInner {
                payload: ManuallyDrop::new(DeferPayload { reader, ty }),
            }),
        }
    }

    /// Consumes this `LazyContentLock` returning the stored value.
    ///
    /// Returns `Ok(value)` if the cell was initialized and `Err(payload)` otherwise.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    pub fn into_inner(this: Self) -> Result<Box<DynContent>, DeferPayload> {
        let state = this.once.state();
        match state {
            OnceState::Poisoned => panic_poisoned(),
            state => {
                let this = ManuallyDrop::new(this);
                let data = unsafe { ptr::read(&this.data) }.into_inner();
                match state {
                    OnceState::Incomplete => Err(ManuallyDrop::into_inner(unsafe { data.payload })),
                    OnceState::Complete => Ok(ManuallyDrop::into_inner(unsafe { data.content })),
                    OnceState::Poisoned => unreachable!(),
                    OnceState::InProgress => todo!(),
                }
            }
        }
    }

    /// Forces the evaluation of this lazy value and returns a mutable reference
    /// to the result.
    #[inline]
    pub fn force_mut(this: &mut LazyContent) -> &mut DynContent {
        /// # Safety
        ///
        /// May only be called when the state is `Incomplete`.
        #[cold]
        unsafe fn really_init_mut(this: &mut LazyContent) -> &mut DynContent {
            struct PoisonOnPanic<'a>(&'a mut LazyContent);

            impl Drop for PoisonOnPanic<'_> {
                #[inline]
                fn drop(&mut self) {
                    self.0.once.set_state(OnceState::Poisoned);
                }
            }

            // SAFETY: We always poison if the initializer panics (then we never check the data)
            // or set the data on success.
            let payload = unsafe { ManuallyDrop::take(&mut this.data.get_mut().payload) };

            // INVARIANT: Initiated from mutable reference, don't drop because we read it.
            let guard = PoisonOnPanic(this);

            let data = parse_payload(payload.clone());
            guard.0.data.get_mut().content = ManuallyDrop::new(data);
            guard.0.once.set_state(OnceState::Complete);

            // Ensure the lock does not get poisoned.
            std::mem::forget(guard);

            // SAFETY: We put the value in there a few lines above this one.
            unsafe { &mut this.data.get_mut().content }.as_mut()
        }

        let state = this.once.state();
        match state {
            // SAFETY: The `Once` states we completed the initialisation.
            OnceState::Complete => unsafe { &mut this.data.get_mut().content }.as_mut(),
            // SAFETY: The `Once` state is `Incomplete`.
            OnceState::Incomplete => unsafe { really_init_mut(this) },
            OnceState::Poisoned => panic_poisoned(),
            OnceState::InProgress => todo!("init in progress"),
        }
    }

    #[inline]
    pub fn force(this: &LazyContent) -> &DynContent {
        this.once.call_once_force(|state| {
            if state.poisoned() {
                panic_poisoned();
            }

            // SAFETY: `call_once` only runs this closure once, ever.
            let data = unsafe { &mut *this.data.get() };
            let payload = unsafe { ManuallyDrop::take(&mut data.payload) };
            let value = parse_payload(payload);
            data.content = ManuallyDrop::new(value);
        });

        // SAFETY:
        // There are four possible scenarios:
        // * the closure was called and initialized `content`.
        // * the closure was called and panicked, so this point is never reached.
        // * the closure was not called, but a previous call initialized `content`.
        // * the closure was not called because the `Once` is poisoned, which we handled above.
        // So `content` has definitely been initialized and will not be modified again.
        unsafe { &*(*this.data.get()).content }.as_ref()
    }

    /// Returns a mutable reference to the value if initialized. Otherwise (if uninitialized or
    /// poisoned), returns `None`.
    #[inline]
    pub fn get_mut(this: &mut LazyContent) -> Option<&mut DynContent> {
        let state = this.once.state();
        match state {
            // SAFETY:
            // The closure has been run successfully, so `content` has been initialized.
            OnceState::Complete => Some(unsafe { &mut this.data.get_mut().content }.as_mut()),
            _ => None,
        }
    }

    /// Returns a reference to the value if initialized. Otherwise (if uninitialized or poisoned),
    /// returns `None`.
    #[inline]
    pub fn get(this: &LazyContent) -> Option<&DynContent> {
        let state = this.once.state();
        match state {
            // SAFETY:
            // The closure has been run successfully, so `content` has been initialized.
            OnceState::Complete => Some(unsafe { &(*this.data.get()).content }.as_ref()),
            _ => None,
        }
    }

    #[inline]
    pub fn initialized(this: &LazyContent) -> bool {
        this.once.state() == OnceState::Complete
    }
}

impl Drop for LazyContent {
    fn drop(&mut self) {
        match self.once.state() {
            OnceState::Incomplete => unsafe {
                ManuallyDrop::drop(&mut self.data.get_mut().payload)
            },
            OnceState::Complete => unsafe { ManuallyDrop::drop(&mut self.data.get_mut().content) },
            OnceState::Poisoned => {}
            OnceState::InProgress => unreachable!(),
        }
    }
}

impl fmt::Debug for LazyContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("LazyContentLock");
        match LazyContent::get(self) {
            Some(_) => d.field(&format_args!("<data>")),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}

impl From<Box<DynContent>> for LazyContent {
    #[inline]
    fn from(value: Box<DynContent>) -> Self {
        LazyContent {
            once: Once::new_complete(),
            data: UnsafeCell::new(LazyInner {
                content: ManuallyDrop::new(value),
            }),
        }
    }
}

impl AssertSendSync for Box<DynContent> {}
impl AssertSendSync for DeferPayload {}
unsafe impl Sync for LazyContent {}
unsafe impl Send for LazyContent {}

#[cold]
fn parse_payload(payload: DeferPayload) -> Box<DynContent> {
    todo!()
}

#[cold]
#[inline(never)]
fn panic_poisoned() -> ! {
    panic!("LazyContentLock was poisoned")
}
