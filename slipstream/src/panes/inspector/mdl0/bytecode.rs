use crate::editor::Editor;
use crate::format::mdl0::bytecode::Bytecode;
use crate::node::node::Inspectable;

impl Inspectable for Bytecode {
    fn draw_properties(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("{self:#?}"));
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
