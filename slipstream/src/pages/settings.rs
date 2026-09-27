use slipstream_shared::error::SlipstreamResult;

use crate::{
    cmd::AppCommandChannel,
    decorations,
    pages::{RoutablePage, splash::BACKGROUND_TEXTURE},
    shared::GraphicsState,
};

pub struct SettingsPage {
    bg_image: egui::load::SizedTexture,
    render_state: GraphicsState,
    cmd_channel: AppCommandChannel,
}

impl SettingsPage {
    pub fn new(
        cmd_channel: AppCommandChannel,
        render_state: GraphicsState,
    ) -> Box<dyn RoutablePage> {
        let bg_image = BACKGROUND_TEXTURE.lock().unwrap().unwrap();
        Box::new(Self {
            cmd_channel,
            render_state,
            bg_image,
        })
    }
}

impl RoutablePage for SettingsPage {
    fn name(&self) -> &str {
        "Settings"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        decorations::draw_background(&egui::Image::from_texture(self.bg_image), ui);
        decorations::draw_basic_title_bar(ui);
        decorations::draw_tool_buttons(&mut self.cmd_channel, &self.render_state, ui)?;

        let window_bg = ui.visuals().panel_fill;

        egui::CentralPanel::default()
            .frame(egui::Frame::default())
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.25);
                    ui.spacing_mut().button_padding = egui::vec2(12.0, 8.0);

                    egui::Frame::new()
                        .fill(window_bg)
                        .inner_margin(24.0)
                        .show(ui, |ui| {
                            ui.set_max_width(400.0);

                            ui.heading("Settings");
                            ui.separator();

                            ui.add_space(0.05 * ui.available_height());

                            // if ui
                            //     .radio(ui.theme() == egui::Theme::Dark, "Dark mode")
                            //     .clicked()
                            // {
                            //     if ui.theme() == egui::Theme::Dark {
                            //         ui.set_theme(egui::Theme::Light);
                            //     } else {
                            //         ui.set_theme(egui::Theme::Dark);
                            //     }
                            // }
                        })
                });
            });

        Ok(())
    }
}
