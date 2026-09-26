use crate::node::node::Inspectable;
use crate::shared::util::RefCursor;

#[derive(Debug, Clone, PartialEq)]
pub struct Raw {
    pub bytes: RefCursor<[u8]>,
}

impl Inspectable for Raw {
    fn draw_properties(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("File size: {} bytes", self.bytes.remaining_len()));
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
