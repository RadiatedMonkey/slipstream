use crate::panes::{ContentSignature, Pane, PaneAction};
use slipstream_ir::mdl0::definitions::Definitions;
use slipstream_ir::mdl0::tex_links::TextureLinks;
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_ir::visitor::{Visitable, Visitor};
use slipstream_shared::SlipstreamResult;
use slipstream_shared::inspect::Inspect;
use std::ops::ControlFlow;
use std::sync::{Arc, mpsc};

struct InspectorVisitor<'ui> {
    pub ui: &'ui mut egui::Ui,
}

impl Visitor for InspectorVisitor<'_> {
    fn visit_definitions(&mut self, definitions: &Definitions) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn visit_texture_links_mut(&mut self, links: &mut TextureLinks) -> ControlFlow<()> {
        links.draw(&mut |fields| {
            for field in fields {}
        });

        ControlFlow::Continue(())
    }
}

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
            cmd_sender,
            content_sig,
            inspected,
            arena,
        })
    }

    fn draw_contents(
        contents: &mut (dyn Visitable + Send + Sync),
        ui: &mut egui::Ui,
    ) -> SlipstreamResult<()> {
        let mut visitor = InspectorVisitor { ui };
        let _ = contents.accept_mut(&mut visitor); // ignore the control flow as we're not continuing anyways.

        Ok(())
    }

    fn draw_properties(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        self.arena
            .update(self.inspected, |node| {
                ui.label(format!("Inspecting {} (ID {:?})", node.label, node.key()));

                // Evaluate contents if lazy
                let contents = node.contents.get_or_try_init_mut()?.unwrap();
                Self::draw_contents(contents, ui)
            })
            .transpose()?;

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
