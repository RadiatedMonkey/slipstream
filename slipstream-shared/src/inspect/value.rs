use std::{borrow::Cow, fmt, ops::RangeInclusive};

use crate::inspect::{Inspect, InspectFieldHelper, num::NumValue};

#[derive(Debug)]
pub struct FieldConfig {
    pub range: Option<RangeInclusive<f64>>,
    pub read_only: bool,
}

pub trait AsFieldValue {
    fn as_field_value(&mut self, config: &FieldConfig) -> FieldValue<'_>;
}

impl<T: AsFieldValue> AsFieldValue for Vec<T> {
    fn as_field_value(&mut self, config: &FieldConfig) -> FieldValue<'_> {
        let vec = self
            .iter_mut()
            .map(|elem| elem.as_field_value(config))
            .collect::<Vec<_>>();

        FieldValue::Array(vec)
    }
}

impl<T: AsFieldValue> AsFieldValue for [T] {
    fn as_field_value(&mut self, config: &FieldConfig) -> FieldValue<'_> {
        let vec = self
            .iter_mut()
            .map(|elem| elem.as_field_value(config))
            .collect::<Vec<_>>();

        FieldValue::Array(vec)
    }
}

impl<T: Inspect> AsFieldValue for T {
    fn as_field_value(&mut self, _config: &FieldConfig) -> FieldValue<'_> {
        FieldValue::Struct(self)
    }
}

#[derive(Debug)]
pub enum FieldValue<'a> {
    Array(Vec<FieldValue<'a>>),
    Struct(&'a mut dyn Inspect),
    Byte(NumValue<'a, u8>),
    Word(NumValue<'a, u32>),
    Float(NumValue<'a, f32>),
    /// Represented by a checkbox.
    Bool {
        read_only: bool,
        value: bool,
    },
    /// Represented by a small text field.
    Text(&'a mut Cow<'a, str>),
}
