use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::mpsc,
};

use crate::{
    error::{InvalidInputError, SlipstreamError, SlipstreamResult},
    node::{
        defer::Deferred,
        node::IrNodeType,
        refs::{IrArena, IrNodeKey},
    },
    panes::{ContentSignature, Pane, PaneAction, RequestNewPane, inspector::InspectorPane},
};

/// The outliner displays a file tree.
///
/// It only needs a root node and to start from and will explore and draw the rest
/// of the file tree by itself.
///
/// New suboutliners can be made by creating new panes with a child node set to root.
pub struct OutlinerPane {
    cmd_sender: mpsc::Sender<PaneAction>,
    /// The content signature of this pane.
    ///
    /// The signature only contains the type of window and the root node ID.
    content_sig: ContentSignature,
    root: IrNodeKey,
    arena: IrArena,
}

impl OutlinerPane {
    pub fn new(
        cmd_sender: mpsc::Sender<PaneAction>,
        content_sig: ContentSignature,
        root: IrNodeKey,
        arena: IrArena,
    ) -> Box<dyn Pane> {
        Box::new(Self {
            cmd_sender,
            content_sig,
            root,
            arena,
        })
    }

    /// Draws the file tree under the current node.
    ///
    /// Lazy nodes are automatically evaluated once their folder is opened.
    ///
    /// If a specific node has been opened, this function returns the ID of its cache entry.
    fn draw_file_tree(&mut self, root_node: IrNodeKey, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        let curr_node_lock = self
            .arena
            .get(root_node)
            .ok_or_else(|| {
                SlipstreamError::from(InvalidInputError {
                    reason: format!("virtual root node {} does not exist", root_node),
                    ..Default::default()
                })
            })?
            .clone();

        let curr_node = curr_node_lock.read();
        let node_kind = curr_node.kind;

        // Persistent ID to make sure the `openness` state of the folders
        // survives structural UI changes.
        //
        // This ID includes the outliner base ID as well since there may be multiple
        // outliners.
        let state_node_id =
            ui.make_persistent_id(format!("outliner{}_state{}", self.root, curr_node.id));

        if node_kind.is_expandable() {
            let mut collapsing_state =
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    state_node_id,
                    false,
                );

            let row_height = ui.spacing().interact_size.y;
            let row_rect = egui::Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(ui.available_width(), row_height),
            );

            let row_response = ui.interact(
                row_rect,
                state_node_id.with("response"),
                egui::Sense::click(),
            );

            // Draw a full width background when the cursor is hovering over the header.
            if ui.rect_contains_pointer(row_rect) {
                ui.painter().rect_filled(
                    row_rect,
                    ui.visuals().widgets.hovered.corner_radius,
                    ui.visuals().widgets.hovered.bg_fill,
                );
            }

            ui.horizontal(|ui| {
                ui.allocate_ui(egui::vec2(row_height, row_height), |ui| {
                    let node_kind = curr_node.kind;
                    let icon_response =
                        collapsing_state.show_toggle_button(ui, move |ui, openness, response| {
                            draw_outliner_node_icon(ui, openness, node_kind, response)
                        });

                    let label_response = ui.label(&curr_node.label);

                    if (row_response.clicked() || label_response.clicked())
                        && !icon_response.hovered()
                    {
                        collapsing_state.toggle(ui);
                    }

                    row_response.context_menu(|ui| {
                        curr_node.draw_context_menu(&mut self.cmd_sender, ui);
                    });

                    label_response.context_menu(|ui| {
                        curr_node.draw_context_menu(&mut self.cmd_sender, ui);
                    });
                });
            });

            if ui.rect_contains_pointer(row_rect) {
                ui.set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            let body_response = collapsing_state.show_body_indented(&row_response, ui, |ui| {
                // Render children if this node has already been evaluated.
                if let Deferred::Evaluated(body) = &curr_node.body {
                    for &child in &body.children {
                        self.draw_file_tree(child, ui)?;
                    }
                } else {
                    {
                        drop(curr_node);
                        let mut curr_node = curr_node_lock.write();
                        curr_node.evaluate()?;
                    }

                    let curr_node = curr_node_lock.read();
                    let Deferred::Evaluated(body) = &curr_node.body else {
                        unreachable!()
                    };

                    for &child in &body.children {
                        self.draw_file_tree(child, ui)?;
                    }
                }

                Ok::<(), SlipstreamError>(())
            });

            if let Some(response) = body_response {
                response.inner?;
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(curr_node.kind.icon_closed());

                let response = ui.button(&curr_node.label);

                if response.clicked() {
                    self.cmd_sender
                        .send(PaneAction::RequestNewPane(RequestNewPane::Inspector {
                            inspected: curr_node.id,
                        }))
                        .expect("failed to send inspector pane open request");
                }

                response.context_menu(|ui| {
                    curr_node.draw_context_menu(&mut self.cmd_sender, ui);
                });
            });
        }

        Ok(())
    }
}

impl Pane for OutlinerPane {
    fn content_signature(&self) -> ContentSignature {
        self.content_sig
    }

    fn title(&self) -> egui::WidgetText {
        egui::WidgetText::Text(String::from("Outliner"))
    }

    fn draw(&mut self, ui: &mut egui::Ui, _tile_id: egui_tiles::TileId) -> egui_tiles::UiResponse {
        let drag_started = ui.heading("Outliner").drag_started();

        ui.spacing_mut().item_spacing.y = 7.5;

        egui::ScrollArea::both().show(ui, |ui| {
            self.draw_file_tree(self.root, ui).unwrap();
        });

        if drag_started {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        }
    }
}

/// Draws the icon of files and folders in the outliner.
fn draw_outliner_node_icon(
    ui: &mut egui::Ui,
    openness: f32,
    node_kind: IrNodeType,
    response: &egui::Response,
) {
    let icon = if openness < 0.5 {
        node_kind.icon_closed()
    } else {
        node_kind.icon_open()
    };

    let galley = egui::WidgetText::from(icon).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    );

    let center_pos = response.rect.center() - (galley.size() * 0.5);

    ui.painter()
        .galley(center_pos, galley, ui.visuals().text_color());
}
