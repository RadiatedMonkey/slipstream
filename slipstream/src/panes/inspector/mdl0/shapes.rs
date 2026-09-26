use crate::{format::mdl0::shapes::Shape, node::node::Inspectable};

impl Inspectable for Shape {
    fn draw_properties(&mut self, ui: &mut egui::Ui) {
        ui.label("hello");
        // ui.label(format!("{self:#?}"));
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
