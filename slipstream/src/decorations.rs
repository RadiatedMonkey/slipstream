use crate::{
    cmd::AppCommandChannel,
    config::APP_TITLE,
    icons::icons,
    pages::{info::InfoPage, settings::SettingsPage},
    shared::GraphicsState,
};

/// The size state of the window.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum WindowState {
    /// The window is in a normal state, not minimized or maximized.
    #[default]
    Normal,
    /// The window is currently maximized.
    Maximized,
    /// The window is currently minimized.
    Minimized,
}

impl WindowState {
    /// Determines the current state of the window.
    pub fn get_state(ui: &egui::Ui) -> Self {
        let states = ui.input(|i| {
            let vp = i.viewport();
            (vp.maximized, vp.minimized)
        });

        match states {
            (Some(true), _) => Self::Maximized,
            (_, Some(true)) => Self::Minimized,
            _ => Self::Normal,
        }
    }
}

/// Renders the double square icon to unmaximize the window.
/// It is a separate function instead of an icon since `phosphoricons` did not have
/// a fitting icon. The icon is drawn manually using egui's painter.
///
/// This should only be called when the window is maximized.
pub fn draw_unmaximize(ui: &mut egui::Ui) -> egui::Response {
    let padding = ui.style().spacing.button_padding;
    let icon_size = egui::vec2(10.0, 10.0);

    let total_width = icon_size.x + padding.x * 2.0;
    let total_size = egui::vec2(total_width, ui.available_height());

    let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let visuals = ui.style().interact(&response);

        painter.rect(
            rect,
            0.0,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        let stroke_color = if response.hovered() {
            ui.visuals().widgets.hovered.fg_stroke.color
        } else {
            ui.visuals().widgets.inactive.fg_stroke.color
        };

        let stroke = egui::Stroke::new(1.0, stroke_color);

        let box_size = egui::vec2(8.0, 8.0);
        let back_min = rect.min + egui::vec2(padding.x + 3.0, padding.y + 1.0);
        let back_rect = egui::Rect::from_min_size(back_min, box_size);

        // Draws the top line of the rear square
        painter.line_segment([back_rect.left_top(), back_rect.right_top()], stroke);

        // Draws the right line of the rear square.
        painter.line_segment([back_rect.right_top(), back_rect.right_bottom()], stroke);

        let front_min = rect.min + egui::vec2(padding.x + 0.0, padding.y + 4.0);
        let front_rect = egui::Rect::from_min_size(front_min, box_size);

        painter.rect_filled(front_rect, 0.0, visuals.weak_bg_fill);
        painter.rect_stroke(front_rect, 0.0, stroke, egui::StrokeKind::Middle);
    }

    response
}

/// Draws the current version and Git hash in the bottom of the screen.
pub fn draw_version_details(ui: &mut egui::Ui) {
    let screen_rect = ui.viewport_rect();
    let pos = egui::pos2(screen_rect.min.x + 12.0, screen_rect.max.y - 12.0);

    ui.ctx()
        .layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("version_overlay"),
        ))
        .text(
            pos,
            egui::Align2::LEFT_BOTTOM,
            format!(
                "v{} ({})",
                env!("CARGO_PKG_VERSION"),
                &env!("VERGEN_GIT_SHA")[..8]
            ),
            egui::FontId::proportional(12.0),
            egui::Color32::from_white_alpha(200),
        );
}

/// Draws the background image with a gray overlay.
pub fn draw_background(bg_image: &egui::Image, ui: &mut egui::Ui) {
    let viewport_rect = ui.ctx().viewport_rect();

    bg_image.paint_at(ui, viewport_rect);

    let bg_overlay = if ui.theme() == egui::Theme::Dark {
        egui::Color32::from_black_alpha(100)
    } else {
        egui::Color32::TRANSPARENT
    };

    ui.painter().rect_filled(viewport_rect, 0.0, bg_overlay);
}

/// Draws the title buttons (close, minimize, maximize)
pub fn draw_title_buttons(ui: &mut egui::Ui) {
    let layout_bg = ui.style().visuals.panel_fill;

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let old_hover = ui.visuals().widgets.hovered.weak_bg_fill;
        let old_hover_fill = ui.visuals().widgets.hovered.fg_stroke;

        // Make the close button red on hover.
        let red_color = ui.visuals().error_fg_color;
        ui.visuals_mut().widgets.hovered.weak_bg_fill = red_color;
        ui.visuals_mut().widgets.hovered.fg_stroke =
            egui::Stroke::new(old_hover_fill.width, egui::Color32::WHITE);

        // Remove margins between buttons
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

        // Disable stroke
        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::new(0.0, egui::Color32::BLACK);

        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::new(0.0, egui::Color32::BLACK);

        // Set button background to layout background
        ui.visuals_mut().widgets.inactive.weak_bg_fill = layout_bg;
        ui.spacing_mut().button_padding = egui::vec2(16.0, 8.0);

        let close_button = egui::Button::new(icons::regular::X).corner_radius(0.0);
        if ui.add(close_button).clicked() {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Don't make the other buttons red on hover.
        ui.visuals_mut().widgets.hovered.weak_bg_fill = old_hover;

        let window_state = WindowState::get_state(ui);
        if window_state == WindowState::Maximized {
            if draw_unmaximize(ui).clicked() {
                ui.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            }
        } else {
            let maximize_button = egui::Button::new(icons::regular::SQUARE).corner_radius(0.0);
            if ui.add(maximize_button).clicked() {
                ui.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
        }

        let minimize_button = egui::Button::new(icons::regular::MINUS).corner_radius(0.0);
        if ui.add(minimize_button).clicked() {
            ui.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    });
}

/// Without decorations there is also no frame to resize the window.
///
/// This function reimplements resizing by emulating its behaviour when the cursor is near
/// the borders of the window.
pub fn handle_frameless_resize(ctx: &egui::Context) {
    let border_width = 6.0;
    let screen_rect = ctx.viewport_rect();

    let Some(pointer_pos) = ctx.pointer_interact_pos() else {
        return;
    };

    let on_left = pointer_pos.x <= screen_rect.min.x + border_width;
    let on_right = pointer_pos.x >= screen_rect.max.x - border_width;
    let on_top = pointer_pos.y <= screen_rect.min.y + border_width;
    let on_bottom = pointer_pos.y >= screen_rect.max.y - border_width;

    if !on_left && !on_right && !on_top && !on_bottom {
        return;
    }

    let direction = match (on_left, on_right, on_top, on_bottom) {
        (true, false, false, false) => Some(egui::ResizeDirection::West),
        (false, true, false, false) => Some(egui::ResizeDirection::East),
        (false, false, true, false) => Some(egui::ResizeDirection::North),
        (false, false, false, true) => Some(egui::ResizeDirection::South),
        (true, false, true, false) => Some(egui::ResizeDirection::NorthWest),
        (true, false, false, true) => Some(egui::ResizeDirection::SouthWest),
        (false, true, true, false) => Some(egui::ResizeDirection::NorthEast),
        (false, true, false, true) => Some(egui::ResizeDirection::SouthEast),
        _ => {
            tracing::error!(
                "Hmmm, the cursor seems to be at opposite sides of the window at the same time"
            );
            None
        }
    };

    if let Some(dir) = direction {
        ctx.set_cursor_icon(match dir {
            egui::ResizeDirection::North | egui::ResizeDirection::South => {
                egui::CursorIcon::ResizeVertical
            }
            egui::ResizeDirection::East | egui::ResizeDirection::West => {
                egui::CursorIcon::ResizeHorizontal
            }
            egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => {
                egui::CursorIcon::ResizeNeSw
            }
            egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
                egui::CursorIcon::ResizeNwSe
            }
        });

        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
        }
    }
}

/// Draws a basic title bar with the window title and title buttons.
pub fn draw_basic_title_bar(ui: &mut egui::Ui) {
    let layout_bg = ui.style().visuals.panel_fill;
    let decorations_id = egui::Id::new("titlebar_panel");

    egui::Panel::top(decorations_id)
        .frame(
            egui::Frame::new()
                .fill(layout_bg)
                .inner_margin(egui::Margin::ZERO)
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
                ui.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            }

            if response.drag_started() {
                // The window should be unmaximized when dragging starts.
                ui.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ui.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            ui.horizontal_centered(|ui| {
                // Draws the title text in the center.
                let rect = ui.max_rect();
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    APP_TITLE,
                    egui::FontId::proportional(14.0),
                    ui.visuals().text_color(),
                );

                draw_title_buttons(ui);
            })
        });
}

/// Draws the vertical tool buttons on the right of the menu screen.
pub fn draw_tool_buttons(
    cmd_channel: &mut AppCommandChannel,
    gfx_state: &GraphicsState,
    ui: &mut egui::Ui,
) -> SlipstreamResult<()> {
    let egui::InnerResponse { inner, .. } = egui::Area::new(egui::Id::new("home_tool_buttons"))
        .anchor(egui::Align2::RIGHT_CENTER, egui::vec2(-10.0, 0.0))
        .show(ui, |ui| -> SlipstreamResult<()> {
            let egui::InnerResponse { inner, .. } = ui.vertical(|ui| -> SlipstreamResult<()> {
                ui.spacing_mut().button_padding = egui::vec2(10.0, 10.0);
                ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);

                let power_button = egui::Button::new(icons::regular::POWER);
                if ui
                    .add(power_button)
                    .on_hover_text_at_pointer("Quit")
                    .clicked()
                {
                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                };

                if ui.theme() == egui::Theme::Dark {
                    if ui
                        .button(icons::regular::SUN)
                        .on_hover_text("Switch to light theme")
                        .clicked()
                    {
                        ui.ctx().set_theme(egui::Theme::Light);
                    }
                } else {
                    if ui
                        .button(icons::regular::MOON)
                        .on_hover_text("Switch to dark theme")
                        .clicked()
                    {
                        ui.ctx().set_theme(egui::Theme::Dark);
                    }
                }

                if ui
                    .button(icons::regular::GEAR_FINE)
                    .on_hover_text("Open settings")
                    .clicked()
                {
                    cmd_channel
                        .try_route(SettingsPage::new(cmd_channel.clone(), gfx_state.clone()))?;
                }

                if ui
                    .button(icons::regular::INFO)
                    .on_hover_text("Open app info")
                    .clicked()
                {
                    cmd_channel.try_route(InfoPage::new(cmd_channel.clone(), gfx_state.clone()))?;
                }

                if ui
                    .button(icons::regular::GITHUB_LOGO)
                    .on_hover_text("Open the project on GitHub")
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl {
                        url: "https://github.com/RadiatedMonkey/slipstream".to_owned(),
                        new_tab: true,
                    });
                }

                Ok(())
            });

            inner
        });

    inner
}
