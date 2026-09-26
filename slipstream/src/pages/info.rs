use crate::{
    cmd::AppCommandChannel,
    error::SlipstreamResult,
    pages::{RoutablePage, intro::IntroPage},
    shared::GraphicsState,
};

/// Displays version and licensing info of the editor.
pub struct InfoPage {
    cmd_channel: AppCommandChannel,
    render_state: GraphicsState,
}

impl InfoPage {
    pub fn new(
        cmd_channel: AppCommandChannel,
        render_state: GraphicsState,
    ) -> Box<dyn RoutablePage> {
        Box::new(Self {
            cmd_channel,
            render_state,
        })
    }
}

impl RoutablePage for InfoPage {
    fn name(&self) -> &str {
        "Info"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        let window_bg = ui.visuals().panel_fill;

        egui::Modal::new(egui::Id::new("app_info_modal"))
            .backdrop_color(egui::Color32::from_black_alpha(125))
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(window_bg)
                    .inner_margin(24.0)
                    .show(ui, |ui| {
                        if ui
                            .vertical_centered(|ui| {
                                ui.heading(format!("Version {}", env!("CARGO_PKG_VERSION")));
                                ui.heading(format!("Git SHA: {}", &env!("VERGEN_GIT_SHA")[..8]));
                                ui.heading(format!(
                                    "Git commit timestamp: {}",
                                    env!("VERGEN_GIT_COMMIT_TIMESTAMP")
                                ));
                                ui.heading(format!(
                                    "Build timestamp: {}",
                                    env!("VERGEN_BUILD_TIMESTAMP")
                                ));
                                ui.heading(format!(
                                    "Built with rustc {}",
                                    env!("VERGEN_RUSTC_SEMVER")
                                ));

                                ui.spacing_mut().button_padding = egui::vec2(10.0, 10.0);
                                ui.visuals_mut().widgets.inactive.fg_stroke =
                                    egui::Stroke::new(0.0, egui::Color32::WHITE);

                                ui.visuals_mut().widgets.active.fg_stroke =
                                    egui::Stroke::new(0.0, egui::Color32::WHITE);

                                ui.hyperlink("https://github.com/RadiatedMonkey/slipstream");

                                ui.add_space(ui.available_height() * 0.25);
                                if ui.button("Close").clicked() {
                                    self.cmd_channel
                                        .try_route(IntroPage::new(
                                            self.cmd_channel.clone(),
                                            self.render_state.clone(),
                                        ))
                                        .unwrap();
                                }
                            })
                            .response
                            .clicked_elsewhere()
                        {
                            self.cmd_channel
                                .try_route(IntroPage::new(
                                    self.cmd_channel.clone(),
                                    self.render_state.clone(),
                                ))
                                .unwrap();
                        }
                    });
            });

        Ok(())
    }
}
