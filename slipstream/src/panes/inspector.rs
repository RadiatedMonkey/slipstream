use std::sync::{mpsc, Arc};
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_shared::{SlipstreamError, SlipstreamResult};
use crate::panes::{ContentSignature, Pane, PaneAction};

pub struct InspectorPane {
    cmd_sender: mpsc::Sender<PaneAction>,

    content_sig: ContentSignature,
    inspected: IrNodeKey,
    arena: Arc<IrArena>,
}

impl InspectorPane {
    pub fn new(
        cmd_sender: mpsc::Sender<PaneAction>,
        content_sig: ContentSignature,
        inspected: IrNodeKey,
        arena: Arc<IrArena>,
    ) -> Box<dyn Pane> {
        Box::new(Self {
            cmd_sender, content_sig, inspected, arena
        })
    }
    
    fn draw_properties(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        self.arena.update(self.inspected, |node| {
            ui.label(format!("Inspecting {} (ID {:?})", node.label, node.key()));
            
            Ok::<_, SlipstreamError>(())
        }).transpose()?;
        
        Ok(())
    }
}

impl Pane for InspectorPane {
    fn content_signature(&self) -> ContentSignature {
        self.content_sig
    }
    
    fn title(&self) -> egui::WidgetText {
        egui::WidgetText::Text(String::from("Inspector"))
    }
    
    fn draw(&mut self, ui: &mut egui::Ui, _tile_id: egui_tiles::TileId) -> egui_tiles::UiResponse {
        let drag_started = ui.heading("Inspector").drag_started();
        
        self.draw_properties(ui).unwrap();
        
        if drag_started {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        }
    }
}