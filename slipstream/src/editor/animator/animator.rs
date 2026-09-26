use crate::editor::Editor;

impl Editor {
    pub fn draw_animator_window(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("animator_panel").show(ui, |ui| {
            ui.heading("Keyframes");
        });
    }
}
