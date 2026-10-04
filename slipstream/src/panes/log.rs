use crate::panes::PaneAction;
use crate::{
    panes::{ContentSignature, Pane},
    reg_icon,
    shared::mem_logger::GLOBAL_MEM_LOGS,
};
use slipstream_shared::SlipstreamError;
use std::sync::mpsc;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::LazyLock,
};

/// All log panes have the same ID because they simply show the same content.
static LOG_PANE_CONTENT_ID: LazyLock<ContentSignature> = LazyLock::new(|| {
    let mut hasher = DefaultHasher::new();
    "logs".hash(&mut hasher);

    ContentSignature(hasher.finish())
});

pub struct LogPane {
    cmd_sender: mpsc::Sender<PaneAction>,
}

impl LogPane {
    pub fn new(cmd_sender: mpsc::Sender<PaneAction>) -> Box<dyn Pane> {
        Box::new(LogPane { cmd_sender })
    }
}

impl Pane for LogPane {
    fn content_signature(&self) -> ContentSignature {
        *LOG_PANE_CONTENT_ID
    }

    fn title(&self) -> egui::WidgetText {
        egui::WidgetText::Text(String::from("Logs"))
    }

    fn draw(&mut self, ui: &mut egui::Ui, tile_id: egui_tiles::TileId) -> egui_tiles::UiResponse {
        let egui::InnerResponse { inner, .. } = ui.horizontal(|ui| {
            let drag_started = ui.heading("Logs").drag_started();

            if ui.button(reg_icon!(X)).clicked() {
                self.cmd_sender.send(PaneAction::RemoveTile(tile_id))?;
            }

            Ok::<_, SlipstreamError>(drag_started)
        });
        let drag_started = inner.expect("failed to send pane close request");

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
