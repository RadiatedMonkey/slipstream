use egui::emath;
use slipstream_shared::error::SlipstreamResult;
use std::borrow::Cow;
use std::ops::RangeInclusive;

pub trait AsFieldValue {
    fn as_field_value(&mut self) -> FieldValue<'_>;
}

impl AsFieldValue for bool {
    fn as_field_value(&mut self) -> FieldValue<'_> {
        FieldValue::Bool(self)
    }
}

impl AsFieldValue for i64 {
    fn as_field_value(&mut self) -> FieldValue<'_> {
        FieldValue::Int {
            val: self,
            range: 0..=2,
        }
    }
}

pub enum FieldValue<'a> {
    /// Represented by a draggable value.
    Int {
        val: &'a mut i64,
        range: RangeInclusive<i64>,
    },
    /// Represented by a draggable value.
    Float {
        val: &'a mut f64,
        range: RangeInclusive<f64>,
    },
    /// Represented by a checkbox.
    Bool(&'a mut bool),
    /// Represented by a small text field.
    Text(&'a mut Cow<'a, str>),
}

pub struct InspectFieldHelper<'a> {
    pub label: &'static str,
    pub category: Option<&'static str>,
    pub value: FieldValue<'a>,
}

impl InspectFieldHelper<'_> {
    fn draw(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let response = match &mut self.value {
            FieldValue::Int { val, range } => {
                let drag = egui::DragValue::new(*val).range(range.clone());

                ui.add(drag)
            }
            FieldValue::Float { val, range } => {
                let drag = egui::DragValue::new(*val).range(range.clone());

                ui.add(drag)
            }
            FieldValue::Bool(val) => {
                let check = egui::Checkbox::new(val, "hello");

                ui.add(check)
            }
            FieldValue::Text(val) => {
                let edit = egui::TextEdit::singleline(*val);

                ui.add(edit)
            }
        };

        response
    }
}

pub trait Inspect {
    fn draw_properties(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()>;
}

impl<T: emath::Numeric> Inspect for &mut T {
    fn draw_properties(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        let drag = egui::DragValue::new(*self);
        ui.add(drag);

        Ok(())
    }
}
