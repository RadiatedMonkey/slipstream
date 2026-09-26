use crate::format::mdl0::tevs::Tev;
use crate::node::node::Inspectable;
use egui::Ui;

impl Inspectable for Tev {
    fn draw_properties(&mut self, ui: &mut Ui) {
        ui.label(format!("{self:#?}"));
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
