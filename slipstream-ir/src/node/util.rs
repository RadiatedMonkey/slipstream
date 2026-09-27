pub(super) trait UncheckedOptionExt<T> {
    unsafe fn unchecked_unwrap(self) -> T;
}

impl<T> UncheckedOptionExt<T> for Option<T> {
    #[inline]
    unsafe fn unchecked_unwrap(self) -> T {
        match self {
            Some(x) => x,
            None => unsafe { unreachable() },
        }
    }
}

#[inline]
unsafe fn unreachable() -> ! {
    if cfg!(debug_assertions) {
        unreachable!();
    } else {
        unsafe { std::hint::unreachable_unchecked() }
    }
}
