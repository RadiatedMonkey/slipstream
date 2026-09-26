use crate::{format::mdl0::colors::ColorBuf, node::node::Inspectable};

impl Inspectable for ColorBuf {
    fn draw_properties(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("{self:?}"));
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
