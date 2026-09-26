use crate::editor::{Editor, OpenedFileInfo};
use crate::shared::GraphicsState;
use crate::{
    cmd::AppCommandChannel,
    decorations,
    error::SlipstreamResult,
    pages::{RoutablePage, splash::BACKGROUND_TEXTURE},
};

pub struct IntroPage {
    bg_image: egui::load::SizedTexture,
    render_state: GraphicsState,
    cmd_channel: AppCommandChannel,
}

impl IntroPage {
    pub fn new(
        cmd_channel: AppCommandChannel,
        render_state: GraphicsState,
    ) -> Box<dyn RoutablePage> {
        let bg_image = BACKGROUND_TEXTURE.lock().unwrap().unwrap();
        Box::new(Self {
            cmd_channel,
            bg_image,
            render_state,
        })
    }
}

impl RoutablePage for IntroPage {
    fn name(&self) -> &str {
        "Intro"
    }

    fn draw(&mut self, ui: &mut egui::Ui) -> SlipstreamResult<()> {
        decorations::draw_version_details(ui);
        decorations::draw_background(&egui::Image::from_texture(self.bg_image), ui);
        decorations::draw_basic_title_bar(ui);

        let window_bg = ui.visuals().panel_fill;

        let egui::InnerResponse { inner, .. } = egui::CentralPanel::default()
            .frame(egui::Frame::default())
            .show(ui, |ui| -> SlipstreamResult<()> {
                // Layout container to keep inner window centered
                let egui::InnerResponse { inner, .. } = ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.25); // Top spacing
                    ui.spacing_mut().button_padding = egui::vec2(12.0, 8.0);

                    let mut frame_width = 0.0;

                    egui::Frame::new()
                        .fill(window_bg)
                        .corner_radius(0.0)
                        .inner_margin(24.0)
                        .show(ui, |ui| {
                            ui.set_max_width(400.0);

                            frame_width = ui.available_width();

                            ui.heading("Recent files");
                            ui.separator();

                            ui.add_space(0.05 * ui.available_height());

                            ui.columns(2, |ui| {
                                for ui in ui {
                                    for i in 0..4 {
                                        if ui.button(format!("driver_{i}.brres")).clicked() {
                                            tracing::debug!("whooshdsd");
                                        }
                                    }
                                }
                            });
                        });

                    let egui::InnerResponse { inner, .. } = egui::Frame::new()
                        .fill(window_bg)
                        .corner_radius(0.0)
                        .inner_margin(24.0)
                        .show(ui, |ui| -> SlipstreamResult<()> {
                            ui.set_width(frame_width);
                            ui.set_height(40.0);

                            let inner = ui.columns_const(|[col1, col2]| -> SlipstreamResult<()> {
                                let egui::InnerResponse { inner, .. } =
                                    col1.vertical_centered(|ui| -> SlipstreamResult<()> {
                                        if ui
                                            .button(egui::RichText::new("Open another file"))
                                            .clicked()
                                        {
                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                let mut cmd_channel = self.cmd_channel.clone();
                                                let render_state = self.render_state.clone();

                                                // Opening files does not happen often, so we just spawn a new thread
                                                // to run the future to completion.
                                                std::thread::spawn(move || {
                                                    let future = async {
                                                        let selected_file =
                                                            rfd::AsyncFileDialog::new()
                                                                .set_title("Select a file to edit")
                                                                .pick_file()
                                                                .await;

                                                        if let Some(selected_file) = selected_file {
                                                            let file_name =
                                                                selected_file.file_name();
                                                            let file_path =
                                                                selected_file.path().to_path_buf();
                                                            let content =
                                                                selected_file.read().await;

                                                            let editor = Editor::new(
                                                                OpenedFileInfo::Native {
                                                                    path: file_path,
                                                                    file_name,
                                                                    content,
                                                                },
                                                                cmd_channel.clone(),
                                                                render_state,
                                                            )
                                                            .unwrap();

                                                            cmd_channel.try_route(editor).unwrap();
                                                        }
                                                    };

                                                    futures::executor::block_on(future);
                                                });
                                            }

                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let mut cmd_channel = self.cmd_channel.clone();
                                                let render_state = self.render_state.clone();

                                                wasm_bindgen_futures::spawn_local(async move {
                                                    let selected_file = rfd::AsyncFileDialog::new()
                                                        .set_title("Select a file to edit")
                                                        .pick_file()
                                                        .await;

                                                    if let Some(selected_file) = selected_file {
                                                        let file_name = selected_file.file_name();
                                                        let content = selected_file.read().await;

                                                        tracing::info!(
                                                            "Opening file `{file_name}`"
                                                        );

                                                        let editor = Editor::new(
                                                            OpenedFileInfo::Web {
                                                                file_name,
                                                                content,
                                                            },
                                                            cmd_channel.clone(),
                                                            render_state,
                                                        )
                                                        .unwrap();

                                                        cmd_channel.try_route(editor);
                                                    }
                                                });
                                            }
                                        }

                                        Ok(())
                                    });

                                col2.vertical_centered(|ui| {
                                    ui.add_space(0.2 * ui.available_height());

                                    let label = egui::RichText::new("Or drop a file here");
                                    ui.label(label)
                                });

                                inner
                            });

                            inner
                        });

                    inner
                });

                inner?;
                decorations::draw_tool_buttons(&mut self.cmd_channel, &self.render_state, ui)
            });

        inner?;

        let hovered_file =
            ui.input_mut(|input| input.raw.hovered_files.pop().map(|file| file.path.unwrap()));

        let alpha = (ui.animate_bool_with_time(
            ui.make_persistent_id("file_drop_overlay_fade"),
            hovered_file.is_some(),
            0.25,
        ) * 100.0) as u8;

        if alpha > 0 {
            let viewport_rect = ui.ctx().viewport_rect();

            let bg_overlay = if ui.theme() == egui::Theme::Dark {
                egui::Color32::from_white_alpha(alpha)
            } else {
                egui::Color32::from_black_alpha(alpha)
            };

            ui.painter().rect_filled(viewport_rect, 0.0, bg_overlay);

            egui::Area::new(egui::Id::new("file_hover_overlay"))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .order(egui::Order::Foreground)
                .show(ui, |ui| {
                    let heading =
                        egui::RichText::new("Drop to edit file").color(egui::Color32::BLACK);
                    ui.label(heading);
                });
        }

        Ok(())
    }
}
