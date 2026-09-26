use std::sync::mpsc;
use std::{path::PathBuf, sync::Arc};

use crate::cmd::AppCommandChannel;
use crate::decorations::{self, WindowState};
use crate::error::{EditorError, EditorResult, InvalidInputError};
use crate::node::refs::{VirtualNodeId, VirtualNodeMap, VirtualRefCacheMap};
use crate::node::root::{self};
use crate::pages::RoutablePage;
use crate::pages::intro::IntroPage;
use crate::panes::inspector::InspectorPane;
use crate::panes::log::LogPane;
use crate::panes::outliner::OutlinerPane;
use crate::panes::viewer::ViewerPane;
use crate::panes::{ContentSignature, Pane, PaneAction, PaneBehavior, RequestNewPane};
use crate::shared::GraphicsState;
use crate::shared::util::RefCursor;

pub struct Properties {
    pub label: String,
    pub node_id: VirtualNodeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenedFileInfo {
    Native {
        path: PathBuf,
        file_name: String,
        content: Vec<u8>,
    },
    Web {
        file_name: String,
        content: Vec<u8>,
    },
}

impl OpenedFileInfo {
    pub fn content(&self) -> &[u8] {
        match self {
            Self::Native { content, .. } => content,
            Self::Web { content, .. } => content,
        }
    }

    pub fn file_name(&self) -> &str {
        match self {
            Self::Native { file_name, .. } => file_name,
            Self::Web { file_name, .. } => file_name,
        }
    }
}

/// Data specific to the editor page.
pub struct Editor {
    pub cmd: AppCommandChannel,
    pub render_state: GraphicsState,
    /// The path of the current file open in the editor.
    ///
    /// This is a regular filesystem path, pointing to the root file.
    /// Not an internal URI.
    pub file_info: OpenedFileInfo,
    /// The root node of the file.
    pub file_base_node: VirtualNodeId,
    pub node_map: VirtualNodeMap,

    pub pane_behavior: PaneBehavior,
    pub pane_tree: egui_tiles::Tree<Box<dyn Pane>>,
}

impl Editor {
    pub fn new(
        file_info: OpenedFileInfo,
        cmd_channel: AppCommandChannel,
        render_state: GraphicsState,
    ) -> EditorResult<Box<dyn RoutablePage>> {
        let contents = file_info.content();
        let cursor = RefCursor::new(Arc::<[u8]>::from(contents));

        let node_map = Arc::new(VirtualRefCacheMap::new());
        let root_node = root::deserialize_maybe_compressed(
            cursor,
            &node_map,
            file_info.file_name().to_owned(),
        )?;

        let mut tiles = egui_tiles::Tiles::default();

        let (tx, rx) = mpsc::channel();

        let container = egui_tiles::Linear::new(egui_tiles::LinearDir::Horizontal, Vec::new());
        let container_id = tiles.insert_container(container);

        let outliner_sig = RequestNewPane::Outliner { root: root_node }.content_signature();
        let outliner =
            OutlinerPane::new(tx.clone(), outliner_sig, root_node, Arc::clone(&node_map));

        let viewer_sig = RequestNewPane::Viewer { viewed: None }.content_signature();
        let viewer = ViewerPane::new(
            tx.clone(),
            viewer_sig,
            None,
            Arc::clone(&node_map),
            render_state.clone(),
        );

        let panes = [outliner, viewer?]
            .into_iter()
            .map(|pane| tiles.insert_pane(pane))
            .collect::<Vec<_>>();

        let egui_tiles::Tile::Container(grid) = tiles.get_mut(container_id).unwrap() else {
            unreachable!()
        };

        panes.iter().for_each(|&id| grid.add_child(id));

        let pane_behavior = PaneBehavior {
            receiver: rx,
            sender: tx,
            focused_tile: None,
        };

        let pane_tree =
            egui_tiles::Tree::new(egui::Id::new("editor_pane_tree"), container_id, tiles);

        Ok(Box::new(Self {
            cmd: cmd_channel,
            render_state: render_state.clone(),

            node_map,
            file_info,
            file_base_node: root_node,

            pane_behavior,
            pane_tree,
        }))
    }

    pub fn get_active_pane(&self) -> Option<egui_tiles::TileId> {
        self.pane_behavior.focused_tile
    }

    /// Returns the tile ID of the currently active container.
    ///
    /// This container will be the parent of the currently active pane.
    pub fn get_active_container(&mut self) -> Option<egui_tiles::TileId> {
        self.pane_tree.active_tiles().first().copied()
    }

    /// Handles a [`RequestNewPane`] request by either focusing an existing pane with
    /// the same contents, or creating a new pane.
    ///
    /// The existing panes are compared with the potential one by comparing [`ContentSignature`]s. If an
    /// equal signature is found, that window is focused instead of creating a new pane.
    ///
    /// If no existing pane was found, this function will first try to find the root node.
    /// In case this root node was found and happens to be an individual pane, the root will be turned into a horizontal container
    /// with both the new pane and old root pane contained in it. If this root was already a container, the new pane will simply be appended to it.
    /// If no tiles were found at all, this pane will be set as root.
    pub fn on_new_pane_request(
        &mut self,
        request: RequestNewPane,
    ) -> EditorResult<egui_tiles::TileId> {
        // Check if this pane already exists.
        // This is done using its content ID
        let content_sig = request.content_signature();
        if let Some(existing_tile) = self
            .pane_tree
            .tiles
            .iter()
            .find_map(|(id, pane)| {
                let egui_tiles::Tile::Pane(pane) = pane else {
                    return None;
                };

                (pane.content_signature() == content_sig).then_some(id)
            })
            .copied()
        {
            // An existing tile has been found, make it active.
            todo!("Found existing pane: {existing_tile:?}");
            return Ok(existing_tile);
        }

        // Pane was not found, create a new one
        let new_pane = match request {
            RequestNewPane::Outliner { root } => OutlinerPane::new(
                self.pane_behavior.sender.clone(),
                content_sig,
                root,
                self.node_map.clone(),
            ),
            RequestNewPane::Inspector { inspected } => InspectorPane::new(
                self.pane_behavior.sender.clone(),
                content_sig,
                inspected,
                self.node_map.clone(),
            ),
            RequestNewPane::Viewer { viewed } => ViewerPane::new(
                self.pane_behavior.sender.clone(),
                content_sig,
                viewed,
                self.node_map.clone(),
                self.render_state.clone(),
            )?,
            RequestNewPane::Log => LogPane::new(),
        };

        let new_pane_id = self.pane_tree.tiles.insert_pane(new_pane);

        match self.pane_tree.root {
            None => {
                // Tree is completely empty, just make the pane the root.
                self.pane_tree.root = Some(new_pane_id);
                tracing::trace!("Handled open pane request, setting it as root");
            }
            Some(root_id) => match self.pane_tree.tiles.get_mut(root_id) {
                // Tree has some content
                Some(egui_tiles::Tile::Container(container)) => {
                    // If the root is a container, just add to it.
                    container.add_child(new_pane_id);
                    tracing::trace!("Handled open pane request, adding it to root");
                }
                Some(egui_tiles::Tile::Pane(_)) => {
                    // If the root is a pane, we can't add another pane to it.
                    // Thus we create a container and add both panes as children.
                    // Then the container is set as root.

                    let new_root = self
                        .pane_tree
                        .tiles
                        .insert_horizontal_tile(vec![root_id, new_pane_id]);

                    self.pane_tree.root = Some(new_root);
                    tracing::trace!(
                        "Handled open pane request, creating a new root container and moving the panes into it"
                    );
                }
                None => {
                    tracing::error!(
                        "Tile root points to a non-existent tile, overriding root with new pane"
                    );

                    self.pane_tree.root = Some(new_pane_id);
                }
            },
        }

        Ok(new_pane_id)
    }

    /// Draws the editor's upper toolbar.
    ///
    /// These are the `File`, `Edit`, buttons you often see in application .
    fn draw_upper_toolbar(&mut self, ui: &mut egui::Ui) {
        let layout_bg = ui.style().visuals.panel_fill;
        let decorations_id = egui::Id::new("title_panel");

        egui::Panel::top(decorations_id)
            .frame(
                egui::Frame::new()
                    .fill(layout_bg)
                    .inner_margin(egui::Margin::symmetric(8, 0))
                    .outer_margin(egui::Margin::ZERO),
            )
            .resizable(false)
            .show(ui, |ui| {
                // Respond to dragging and click of the title bar.
                let response = ui.interact(
                    ui.ctx().viewport_rect(),
                    decorations_id,
                    egui::Sense::click_and_drag(),
                );

                if response.double_clicked() {
                    let window_state = WindowState::get_state(ui);
                    if window_state == WindowState::Maximized {
                        ui.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                    } else {
                        ui.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    }
                }

                if response.drag_started() {
                    // ui.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                    ui.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.horizontal_centered(|ui| {
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Save").clicked() {
                                todo!()
                            }

                            if ui.button("Save as").clicked() {
                                todo!()
                            }

                            if ui.button("Close").clicked() {
                                self.cmd
                                    .try_route(IntroPage::new(
                                        self.cmd.clone(),
                                        self.render_state.clone(),
                                    ))
                                    .unwrap();
                            }

                            if ui.button("Quit").clicked() {
                                ui.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });

                        if ui.button("Logs").clicked() {
                            self.on_new_pane_request(RequestNewPane::Log);
                        }

                        ui.menu_button("Settings", |_ui| {});
                    });

                    decorations::draw_title_buttons(ui);
                });
            });
    }
}

impl RoutablePage for Editor {
    fn name(&self) -> &str {
        "Editor"
    }

    fn update(&mut self) -> EditorResult<()> {
        while let Ok(cmd) = self.pane_behavior.receiver.try_recv() {
            match cmd {
                PaneAction::RequestPaneEdit(_) => todo!(),
                PaneAction::RequestNewPane(request) => {
                    self.on_new_pane_request(request).unwrap();
                }
                PaneAction::RemoveTile(tile) => {
                    self.pane_tree.tiles.remove(tile);
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> EditorResult<()> {
        self.draw_upper_toolbar(ui);
        self.pane_tree.ui(&mut self.pane_behavior, ui);

        Ok(())
    }
}
