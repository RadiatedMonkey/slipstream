pub mod mdl0;
pub mod raw;
pub mod widgets;

use std::sync::mpsc;

use crate::{
    node::refs::{VirtualNodeId, VirtualNodeMap},
    panes::{ContentSignature, Pane, PaneAction},
    reg_icon,
};

pub struct InspectorPane {
    cmd_sender: mpsc::Sender<PaneAction>,
    content_sig: ContentSignature,

    node_map: VirtualNodeMap,
    node: VirtualNodeId,
}

impl InspectorPane {
    pub fn new(
        cmd_sender: mpsc::Sender<PaneAction>,
        content_sig: ContentSignature,
        node: VirtualNodeId,
        node_map: VirtualNodeMap,
    ) -> Box<dyn Pane> {
        Box::new(Self {
            cmd_sender,
            content_sig,
            node,
            node_map,
        })
    }
}

impl Pane for InspectorPane {
    fn content_signature(&self) -> ContentSignature {
        self.content_sig
    }

    fn title(&self) -> egui::WidgetText {
        egui::WidgetText::Text(String::from("Inspector"))
    }

    fn draw(&mut self, ui: &mut egui::Ui, tile_id: egui_tiles::TileId) -> egui_tiles::UiResponse {
        let drag_started = ui.heading("Inspector").drag_started();

        let open_node = self.node_map.get(self.node).unwrap();

        let mut node_ref = open_node.write();
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&node_ref.label);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(reg_icon!(X)).clicked() {
                        self.cmd_sender.send(PaneAction::RemoveTile(tile_id));
                    }
                });
            });

            ui.add_space(0.02 * ui.available_height());

            let inspectable = node_ref.body.evaluate().unwrap().inspectable.as_mut();
            if let Some(node_content) = inspectable {
                node_content.draw_properties(ui);
            }
        });

        if drag_started {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        }
    }
}
