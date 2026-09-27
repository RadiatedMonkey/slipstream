use std::sync::Mutex;

use egui::load::SizedTexture;
use slipstream_shared::error::SlipstreamResult;

use crate::{
    cmd::AppCommandChannel,
    config::{APP_TITLE, DEFAULT_SIZE},
    pages::{RoutablePage, intro::IntroPage},
    shared::GraphicsState,
};

pub static BACKGROUND_TEXTURE: Mutex<Option<SizedTexture>> = Mutex::new(None);

/// Loading screen shown when the app is opening.
///
/// This is used to load initial resources the app might need in the home screen.
pub struct SplashPage {
    ctx: egui::Context,
    /// State of the renderer, this is only stored here to pass it on to an editor page
    /// when it is loaded.
    render_state: GraphicsState,
    /// Channel to communicate with the parent app instance.
    cmd_channel: AppCommandChannel,
}

impl SplashPage {
    pub fn new(
        ctx: egui::Context,
        render_state: GraphicsState,
        mut cmd_channel: AppCommandChannel,
    ) -> Box<dyn RoutablePage> {
        // Failing to center the window can be ignored.
        let _ = cmd_channel.try_center_window();

        ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Launching...".into()));

        Box::new(Self {
            ctx,
            cmd_channel,
            render_state,
        })
    }

    /// Polls the status of resources being loaded by egui.
    fn poll_preload(&mut self) -> Result<egui::load::TexturePoll, egui::load::LoadError> {
        let bg_image = egui::include_image!("../../images/intro_bg.png");
        bg_image.load(
            &self.ctx,
            egui::TextureOptions {
                magnification: egui::TextureFilter::Linear,
                minification: egui::TextureFilter::Linear,
                mipmap_mode: None,
                wrap_mode: egui::TextureWrapMode::Repeat,
            },
            egui::SizeHint::Size {
                width: 3840,
                height: 2160,
                maintain_aspect_ratio: true,
            },
        )
    }
}

impl RoutablePage for SplashPage {
    fn name(&self) -> &str {
        "Splash"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        egui::Area::new(egui::Id::new("splash_panel"))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ui, |ui| {
                ui.set_max_width(360.0);

                ui.vertical_centered(|ui| {
                    let title_text = egui::RichText::new(APP_TITLE);
                    let title = egui::Label::new(title_text).selectable(false);
                    ui.add(title);

                    ui.add_space(0.2 * ui.available_height());

                    let spinner = egui::Spinner::new().size(32.0);
                    ui.add(spinner);
                });
            });

        let poll = self.poll_preload();
        match poll {
            Ok(egui::load::TexturePoll::Ready { texture }) => {
                // Resources are ready, launch the home menu.

                tracing::trace!("Resources loaded");

                // Reset decorations

                self.ctx
                    .send_viewport_cmd(egui::ViewportCommand::Resizable(true));
                self.ctx
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(DEFAULT_SIZE));

                self.ctx
                    .send_viewport_cmd(egui::ViewportCommand::Title("Slipstream".to_owned()));

                // `center_window` does not work here since it would still be using the old window size.

                let mut window_rect = self.ctx.viewport_rect();
                // adjust the existing window rect to include the new size.
                window_rect.max = window_rect.min + DEFAULT_SIZE;

                let sizex = window_rect.max.x - window_rect.min.x;
                let sizey = window_rect.max.y - window_rect.min.y;

                if let Some(monitor_size) = self.ctx.input(|i| i.viewport().monitor_size) {
                    let monitor_pos = egui::pos2(0.0, 0.0);

                    let center_x = monitor_pos.x + (monitor_size.x - sizex) / 2.0;
                    let center_y = monitor_pos.y + (monitor_size.y - sizey) / 2.0;

                    self.ctx
                        .send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                            center_x, center_y,
                        )));
                }

                *BACKGROUND_TEXTURE.lock().unwrap() = Some(texture);

                self.cmd_channel.try_route(IntroPage::new(
                    self.cmd_channel.clone(),
                    self.render_state.clone(),
                ))?;
            }
            Ok(egui::load::TexturePoll::Pending { .. }) => {
                // ui.ctx().request_repaint();
            }
            Err(err) => {
                tracing::error!("failed to load background texture: {err}");
            }
        }

        Ok(())
    }
}
