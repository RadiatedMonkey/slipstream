use std::{
    hash::{Hasher},
    sync::mpsc,
};

use slipstream_ir::node::{arena::{IrArena, IrNodeKey}, node::{IrNode, IrNodeType}};
use slipstream_shared::error::{InvalidInputError, SlipstreamError, SlipstreamResult};

use crate::{icons::NodeVisualsExt, panes::{ContentSignature, Pane, PaneAction, PaneId, RequestNewPane, inspector::InspectorPane}};

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
    /// Creates a new outliner pane.
    /// 
    /// `root_key` is the node that will be the root of the outliner. This makes it possible
    /// to create outliners of subsets of the project.
    pub fn new(
        cmd_sender: mpsc::Sender<PaneAction>,
        content_sig: ContentSignature,
        root_key: IrNodeKey,
        arena: IrArena,
    ) -> Box<dyn Pane> {
        Box::new(Self {
            cmd_sender,
            content_sig,
            root: root_key,
            arena,
        })
    }

    /// Generates a persistent ID for the collapsible state of the given node.
    /// 
    /// Top 32 bits are the pane ID, bottom 32 bits are the node ID.
    fn get_state_id(node_key: IrNodeKey, ui: &mut egui::Ui) -> egui::Id {
        let salt = ((PaneId::Outliner as u64) << 32) | node_key.into();
        ui.make_persistent_id(salt)
    }

    fn draw_directory_node(&self, node: &IrNode, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        let state_id = Self::get_state_id(node.key(), ui);
        let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(), state_id, false
        );

        // Determines the rect that should be coloured when the node is hovered over.
        let row_height = ui.spacing().interact_size.y;
        let row_rect = egui::Rect::from_min_size(
            ui.cursor().min, egui::vec2(ui.available_width(), row_height)
        );

        let row_response = ui.interact(
            row_rect, state_id.with("interact"), egui::Sense::click()
        );

        // If the cursor hovers over the node, fill the background with a different colour.
        if ui.rect_contains_pointer(row_rect) {
            ui.painter().rect_filled(
                row_rect, 
                ui.visuals().widgets.hovered.corner_radius,
                ui.visuals().widgets.hovered.bg_fill
            );
        }

        ui.horizontal(|ui| {
            // Draw the folder icon and label.
            //
            // This block also handles responses.
            ui.allocate_ui(egui::vec2(row_height, row_height), |ui| {
                let icon_response = state.show_toggle_button(ui, move |ui, openness, response| {
                    draw_outliner_node_icon(ui, openness, node.ty, response)
                });

                let label_response = ui.label(node.label());

                // This is kind of hack, but the collapsing states responses kind of suck.
                //
                // We generate our own responses on the outliner row and label of the file, as the collapsing header does
                // not respond to these by default. 
                // We also need to ensure the icon is not below the cursor, as the icon lies within the outliner row. Otherwise
                // the collapsing state itself will also respond and we will attempt to toggle the node twice.
                if (row_response.clicked() || label_response.clicked()) && !icon_response.hovered() {
                    state.toggle(ui);
                }

                // We also need separate context menus for the row and label responses, although they both display the same content.
                row_response.context_menu(|ui| {
                    todo!();
                });

                label_response.context_menu(|ui| {
                    todo!();
                });
            });

            if ui.rect_contains_pointer(row_rect) {
                // Set a custom cursor to make the outliner feel more responsive.
                ui.set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            let body_response = state.show_body_indented(&row_response, ui, |ui| {
                // Render the children of this node.
                for &child in node.children_keys() {
                    // Then start the whole file tree process over again, but for this sub node.
                    self.draw_file_tree(child, ui)?;
                }

                Ok::<(), SlipstreamError>(())
            });

            if let Some(egui::InnerResponse { inner, .. }) = body_response {
                inner?;
            }
        });

        Ok(())
    }

    /// Draws a file in the outliner.
    /// 
    /// This file will have no more subnodes.
    fn draw_leaf_node(&self, node: &IrNode, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        ui.horizontal(|ui| {
            ui.label(node.ty.closed_icon());

            let response = ui.button(node.label());
            if response.clicked() {
                self.cmd_sender.send(PaneAction::RequestNewPane(RequestNewPane::Inspector {
                    inspected: node.key()
                })).expect("failed to send inspector pane open request");
            }

            response.context_menu(|ui| {
                todo!("draw node context menu");
            });
        });

        Ok(())
    }

    /// Draws the file tree under the current node.
    ///
    /// Lazy nodes are automatically evaluated once their folder is opened.
    ///
    /// If a specific node has been opened, this function returns the ID of its cache entry.
    fn draw_file_tree(&mut self, root_node: IrNodeKey, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        self.arena.inspect(root_node, |curr_node| {
            let node_ty = curr_node.ty;

            if node_ty.is_expandable() {
                self.draw_directory_node(curr_node, ui)
            } else {
                self.draw_leaf_node(curr_node, ui)
            }
        }).transpose()?;

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
