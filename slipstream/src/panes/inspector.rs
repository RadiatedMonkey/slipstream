use crate::panes::{ContentSignature, Pane, PaneAction};
use slipstream_ir::mdl0::definitions::Definitions;
use slipstream_ir::mdl0::tex_links::TextureLinks;
use slipstream_ir::mdl0::vertices::VertexBuffer;
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_ir::visitor::{
    Visitable, Visitor, VisitorContext, VisitorContextMut, VisitorContextNodeMut,
};
use slipstream_shared::inspect::Inspect;
use slipstream_shared::{SlipstreamError, SlipstreamResult};
use std::ops::ControlFlow;
use std::sync::{Arc, mpsc};
use slipstream_ir::mdl0::bones::Bone;
use crate::reg_icon;

struct InspectorVisitor<'ui> {
    pub ui: &'ui mut egui::Ui,
}

impl Visitor for InspectorVisitor<'_> {
    fn visit_definitions_mut(
        &mut self,
        context: VisitorContextMut<'_, Definitions>,
    ) -> ControlFlow<()> {
        egui::ScrollArea::vertical().show(self.ui, |ui| {
            ui.label(format!("{:#?}", context.content));
        });
        ControlFlow::Continue(())
    }

    fn visit_bone_mut(
        &mut self, context: VisitorContextMut<'_, Bone>,
    ) -> ControlFlow<()> {
        egui::ScrollArea::vertical().show(self.ui, |ui| {
            ui.label(format!("{:#?}", context.content));
        });
        ControlFlow::Continue(())
    }

    fn visit_vertices_mut(
        &mut self,
        context: VisitorContextMut<'_, VertexBuffer>,
    ) -> ControlFlow<()> {
        egui::ScrollArea::vertical().show(self.ui, |ui| {
            ui.label(format!("{:#?}", context.content));
        });
        ControlFlow::Continue(())
    }

    fn visit_texture_links_mut(
        &mut self,
        mut links: VisitorContextMut<'_, TextureLinks>,
    ) -> ControlFlow<()> {
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

    fn draw_properties(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        self.arena
            .update(self.inspected, |node| {
                ui.label(format!("Inspecting {} (ID {:?})", node.label, node.key()));

                let context = VisitorContextNodeMut {
                    key: node.key(),
                    label: &mut node.label,
                    ty: node.ty,
                };

                // Evaluate contents if lazy
                let contents = node.contents.get_or_try_init_mut()?.unwrap();

                let mut visitor = InspectorVisitor { ui };
                let _ = contents.accept_mut(context, &mut visitor); // ignore the control flow as we're not continuing anyways.

                Ok::<_, SlipstreamError>(())
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

    fn draw(&mut self, ui: &mut egui::Ui, tile_id: egui_tiles::TileId) -> egui_tiles::UiResponse {
        let egui::InnerResponse { inner, .. } = ui.horizontal(|ui| {
            let drag_started = ui.heading("Inspector").drag_started();

            if ui.button(reg_icon!(X)).clicked() {
                self.cmd_sender.send(PaneAction::RemoveTile(tile_id))?;
            }

            Ok::<_, SlipstreamError>(drag_started)
        });
        let drag_started = inner.expect("failed to send pane close request");

        self.draw_properties(ui).unwrap();

        if drag_started {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        }
    }
}
