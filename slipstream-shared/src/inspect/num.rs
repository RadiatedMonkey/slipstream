use std::ops::RangeInclusive;

use crate::inspect::{AsFieldValue, FieldConfig, FieldValue};

#[diagnostic::on_unimplemented(
    label = "non-numerical type",
    message = "The attributes `min` and `max` cannot be used on this field's type"
)]
pub trait IntoBounds<T: Into<f64>> {
    fn into_bounds(min: Option<T>, max: Option<T>) -> RangeInclusive<f64>;
}

#[derive(Debug)]
pub struct NumValue<'a, T> {
    range: Option<RangeInclusive<T>>,
    read_only: bool,
    value: &'a mut T,
}

macro_rules! impl_number {
    ($($ty:ty => $kind:ident),*) => {
        $(
            impl AsFieldValue for $ty {
                fn as_field_value(&mut self, config: &FieldConfig) -> FieldValue<'_> {
                    FieldValue::$kind(NumValue {
                        range: config
                            .range
                            .clone()
                            .map(|range| *range.start() as $ty..=*range.end() as $ty),
                        read_only: config.read_only,
                        value: self
                    })
                }
            }

            impl IntoBounds<$ty> for $ty {
                fn into_bounds(min: Option<$ty>, max: Option<$ty>) -> RangeInclusive<f64> {
                    min.unwrap_or(<$ty>::MIN).into()..=max.unwrap_or(<$ty>::MAX).into()
                }
            }
        )*
    }
}

impl_number!(
    u8 => Byte,
    u32 => Word,
    f32 => Float
);
