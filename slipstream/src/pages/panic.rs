use crate::app::App;

impl App {
    /// Draws a modal displaying information of a panic that was caught.
    ///
    /// When the user closes the window, the app's panic info is reset to `None`.
    pub fn draw_panic_modal(&mut self, ui: &mut egui::Ui) {
        let modal_id = egui::Id::new("panic_modal");
        egui::Modal::new(modal_id).show(ui.ctx(), |ui| {
            let panic = self.panic_info.as_ref().unwrap();

            ui.heading("Panic caught:");

            let text = if let Some(&msg) = panic.downcast_ref::<&'static str>() {
                msg
            } else if let Some(msg) = panic.downcast_ref::<String>() {
                msg
            } else {
                "Unknown"
            };

            let code = egui::RichText::new(text).code().monospace();
            ui.label(code);

            if ui.button("Acknowledge").clicked() {
                self.panic_info = None;
            }
        });
    }
}
