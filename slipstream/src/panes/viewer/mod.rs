pub mod grid;
pub mod pipeline;
pub mod translator;

use std::{
    hash::Hasher,
    sync::{Arc, mpsc},
};

use eframe::egui_wgpu;
use slipstream_ir::node::arena::{IrArena, IrNodeKey};
use slipstream_shared::error::SlipstreamResult;

use crate::panes::viewer::translator::{DrawableModel, ModelScratch};
use crate::{
    panes::{
        ContentSignature, Pane, PaneAction,
        viewer::pipeline::{TEXTURE_FILTER_MODE, ViewerCallback, ViewerPipeline},
    },
    shared::{GraphicsState, camera::CameraController},
};

pub struct ViewerPane {
    cmd_sender: mpsc::Sender<PaneAction>,
    content_sig: ContentSignature,
    node: Option<IrNodeKey>,
    arena: Arc<IrArena>,

    model: Option<DrawableModel>,
    render_state: GraphicsState,
}

impl ViewerPane {
    /// `mdl0_node` should be the ID of an MDL0 file.
    pub fn new(
        cmd_sender: mpsc::Sender<PaneAction>,
        content_sig: ContentSignature,
        mdl0_node: Option<IrNodeKey>,
        arena: Arc<IrArena>,
        render_state: GraphicsState,
    ) -> SlipstreamResult<Box<dyn Pane>> {
        let model = mdl0_node
            .map(|node| ModelScratch::from_root(node, Arc::clone(&arena)))
            .transpose()?;

        let model = None;

        let pipeline = ViewerPipeline::new(render_state.clone());
        render_state
            .renderer
            .write()
            .callback_resources
            .insert(pipeline);

        tracing::trace!("Viewer pipeline initialized");

        Ok(Box::new(Self {
            cmd_sender,
            content_sig,
            node: mdl0_node,
            arena,
            render_state,
            model,
        }))
    }
}

impl Pane for ViewerPane {
    fn content_signature(&self) -> ContentSignature {
        self.content_sig
    }

    fn title(&self) -> egui::WidgetText {
        egui::WidgetText::Text(String::from("3D Viewer"))
    }

    fn draw(&mut self, ui: &mut egui::Ui, _tile_id: egui_tiles::TileId) -> egui_tiles::UiResponse {
        let drag_started = ui.heading("3D Viewer").drag_started();

        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            let target_size = ui.available_size();
            let panel_bounds = egui::Rect::from_min_size(ui.cursor().min, target_size);

            let mut renderer = self.render_state.renderer.write();
            let mut pipeline = renderer
                .callback_resources
                .get_mut::<ViewerPipeline>()
                .unwrap();

            let has_resized =
                pipeline.update_size(glam::uvec2(target_size.x as u32, target_size.y as u32));

            if has_resized {
                // Create copies to temporarily drop the pipeline borrow.
                //
                // This allows us to mutably borrow renderer, which would otherwise be borrowed
                // by the viewer pipeline.
                let output_view = pipeline.screen_texture_state.output_view.clone();
                let egui_tex_id = pipeline.screen_texture_state.egui_texture_id;

                renderer.update_egui_texture_from_wgpu_texture(
                    &self.render_state.device,
                    &output_view,
                    TEXTURE_FILTER_MODE,
                    egui_tex_id,
                );

                // Then reborrow the pipeline.
                pipeline = renderer
                    .callback_resources
                    .get_mut::<ViewerPipeline>()
                    .unwrap();
            }

            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                panel_bounds,
                ViewerCallback,
            ));

            let image_widget = egui::Image::new(egui::load::SizedTexture {
                id: pipeline.screen_texture_state.egui_texture_id,
                size: panel_bounds.size(),
            })
            .sense(egui::Sense::click_and_drag());

            // Camera
            // =============================================================================

            pipeline
                .camera_state
                .camera
                .set_aspect_ratio(target_size.x / target_size.y);

            // If the screen has resized, we need to update to adjust the aspect ratio.
            let mut camera_updated = has_resized;

            let response = ui.add(image_widget);
            if response.dragged() {
                tracing::debug!(
                    "moving image {:?}, pane {:?}",
                    pipeline.screen_texture_state.egui_texture_id,
                    self.content_sig
                );

                let drag_delta = response.drag_delta();

                pipeline
                    .camera_state
                    .camera
                    .drag_delta(glam::vec2(drag_delta.x, drag_delta.y));

                camera_updated = true;
            }

            ui.input(|i| {
                if i.zoom_delta() != 1.0 {
                    // Support zooming using a touchpad for laptops
                    // without a scroll wheel (or for the maniacs trying
                    // to use this editor on a mobile device)
                    let delta = 20.0 * (i.zoom_delta() - 1.0);

                    pipeline.camera_state.camera.scroll_delta(delta);

                    camera_updated = true;
                }

                if i.is_scrolling() && response.contains_pointer() {
                    let scroll_delta = i.smooth_scroll_delta();

                    pipeline.camera_state.camera.scroll_delta(scroll_delta.y);

                    camera_updated = true;
                }
            });

            if camera_updated {
                pipeline.camera_state.update(&self.render_state);
            }
        });

        if drag_started {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        }
    }
}
