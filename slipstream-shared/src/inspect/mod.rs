pub mod num;
pub mod value;

pub use num::*;
pub use value::*;

use std::borrow::Cow;
use std::ops::{Deref, DerefMut, RangeInclusive};

#[derive(Debug)]
pub struct InspectFieldHelper<'a> {
    pub label: &'static str,
    pub category: Option<&'static str>,
    pub value: FieldValue<'a>,
}

pub trait Inspect: std::fmt::Debug {
    fn label(&self) -> &str;
    /// Executes a function, giving it access to the type's field data.
    ///
    /// This is done this way to reduce allocations.
    /// Returning the fields from a function would require them to be allocated into a `Vec` while
    /// returning an array implementing some trait would no longer make this trait dyn compatible.
    fn draw(&mut self, draw_fn: &mut dyn Fn(&mut [InspectFieldHelper<'_>]));
}

/// Blanket implementation that implements the trait for mutable references if the type itself implements the trait.
/// This is used in [`FieldValue`].
impl<T: Inspect> Inspect for &mut T {
    fn label(&self) -> &str {
        (**self).label()
    }

    fn draw(&mut self, draw_fn: &mut dyn Fn(&mut [InspectFieldHelper<'_>])) {
        (*self).draw(draw_fn);
    }
}
