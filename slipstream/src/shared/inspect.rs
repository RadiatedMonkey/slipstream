use egui::{emath, Ui};
use slipstream_shared::error::SlipstreamResult;

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