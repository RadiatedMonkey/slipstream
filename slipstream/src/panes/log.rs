use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::LazyLock,
};

use crate::{
    panes::{ContentSignature, Pane},
    shared::mem_logger::GLOBAL_MEM_LOGS,
};

/// All log panes have the same ID because they simply show the same content.
static LOG_PANE_CONTENT_ID: LazyLock<ContentSignature> = LazyLock::new(|| {
    let mut hasher = DefaultHasher::new();
    "logs".hash(&mut hasher);

    ContentSignature(hasher.finish())
});

pub struct LogPane;

impl LogPane {
    pub fn new() -> Box<dyn Pane> {
        Box::new(LogPane)
    }
}

impl Pane for LogPane {
    fn content_signature(&self) -> ContentSignature {
        *LOG_PANE_CONTENT_ID
    }

    fn title(&self) -> egui::WidgetText {
        egui::WidgetText::Text(String::from("Logs"))
    }

    fn draw(&mut self, ui: &mut egui::Ui, _tile_id: egui_tiles::TileId) -> egui_tiles::UiResponse {
        let drag_started = ui.heading("Logs").drag_started();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for log in GLOBAL_MEM_LOGS.lock().iter() {
                    ui.label(format!("{log:?}"));
                }
            });

        if drag_started {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        }
    }
}
